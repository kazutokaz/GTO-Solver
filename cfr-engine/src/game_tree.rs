/// Game tree representation for heads-up postflop poker
/// OOP = Out of Position (acts first on each street)
/// IP  = In Position (acts second on each street)

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::cards::{Card, parse_card, parse_hand};
use crate::ranges::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Player {
    OOP = 0,
    IP = 1,
}

impl Player {
    pub fn opponent(self) -> Player {
        match self {
            Player::OOP => Player::IP,
            Player::IP => Player::OOP,
        }
    }
    pub fn index(self) -> usize { self as usize }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ActionKind {
    Fold,
    Check,
    Call,
    Bet(f64),   // fraction of pot
    Raise(f64), // fraction of pot
    AllIn,
}

impl ActionKind {
    pub fn to_string(&self) -> String {
        match self {
            ActionKind::Fold => "fold".to_string(),
            ActionKind::Check => "check".to_string(),
            ActionKind::Call => "call".to_string(),
            ActionKind::Bet(f) => format!("bet:{:.2}", f),
            ActionKind::Raise(f) => format!("raise:{:.2}", f),
            ActionKind::AllIn => "allin".to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BetSizeConfig {
    pub ip_bet: Vec<f64>,
    pub oop_bet: Vec<f64>,
    pub ip_raise: Vec<f64>,
    pub oop_raise: Vec<f64>,
    pub oop_donk: Vec<f64>,
}

impl BetSizeConfig {
    pub fn default_flop() -> Self {
        BetSizeConfig {
            ip_bet: vec![0.33, 0.67, 1.0],
            oop_bet: vec![0.33, 0.67, 1.0],
            ip_raise: vec![2.5, 4.0],
            oop_raise: vec![2.5, 4.0],
            oop_donk: vec![0.33, 0.67],
        }
    }
    pub fn default_turn() -> Self {
        BetSizeConfig {
            ip_bet: vec![0.5, 0.75, 1.0],
            oop_bet: vec![0.5, 0.75, 1.0],
            ip_raise: vec![2.5, 3.5],
            oop_raise: vec![2.5, 3.5],
            oop_donk: vec![0.5, 0.75],
        }
    }
    pub fn default_river() -> Self {
        BetSizeConfig {
            ip_bet: vec![0.5, 0.75, 1.0, 1.5],
            oop_bet: vec![0.5, 0.75, 1.0, 1.5],
            ip_raise: vec![2.5],
            oop_raise: vec![2.5],
            oop_donk: vec![0.75, 1.0],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FullBetSizeConfig {
    pub flop: BetSizeConfig,
    pub turn: BetSizeConfig,
    pub river: BetSizeConfig,
}

impl Default for FullBetSizeConfig {
    fn default() -> Self {
        FullBetSizeConfig {
            flop: BetSizeConfig::default_flop(),
            turn: BetSizeConfig::default_turn(),
            river: BetSizeConfig::default_river(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RakeConfig {
    pub percentage: f64,
    pub cap: f64,            // in BBs
    pub no_flop_no_drop: bool,
}

impl Default for RakeConfig {
    fn default() -> Self {
        RakeConfig { percentage: 0.0, cap: 0.0, no_flop_no_drop: true }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeLockEntry {
    pub action_path: Vec<String>,
    pub street: String,
    pub player: String,
    pub hand_strategies: HashMap<String, Vec<f64>>,
}

/// A node in the game tree
#[derive(Clone, Debug)]
pub struct TreeNode {
    pub id: usize,
    pub kind: NodeKind,
}

#[derive(Clone, Debug)]
pub enum NodeKind {
    /// Player decision node
    Action {
        player: Player,
        street: Street,
        pot: f64,         // current pot size in BBs
        stack_ip: f64,    // remaining stack IP
        stack_oop: f64,   // remaining stack OOP
        last_bet: f64,    // size of last bet/raise (0 if checked through)
        actions: Vec<ActionKind>,
        children: Vec<usize>, // child node IDs
        node_locked: bool,
        locked_strategy: Option<HashMap<[Card; 2], Vec<f64>>>,
    },
    /// Terminal: showdown or fold
    Terminal {
        street: Street,
        winner: TerminalWinner,
        pot: f64,
        invested_oop: f64,
        invested_ip: f64,
        saw_flop: bool,
    },
    /// Chance node: dealing a card
    Chance {
        street: Street,
        children: Vec<(Card, usize)>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Street {
    Flop,
    Turn,
    River,
}

#[derive(Clone, Debug)]
pub enum TerminalWinner {
    OOP,  // IP folded
    IP,   // OOP folded
    Showdown,
}

pub struct GameTree {
    pub nodes: Vec<TreeNode>,
    pub root: usize,
    pub stacks: f64,
    pub initial_pot: f64,
    pub board: Vec<Card>,
    pub bet_sizes: FullBetSizeConfig,
    pub rake: RakeConfig,
    pub turn_cards: Vec<Card>,
    pub river_cards: Vec<Card>,
}

impl GameTree {
    pub fn build(
        stack_size: f64,
        pot_size: f64,
        board: Vec<Card>,
        bet_sizes: FullBetSizeConfig,
        rake: RakeConfig,
        turn_cards: Vec<Card>,
        river_cards: Vec<Card>,
    ) -> Self {
        let mut tree = GameTree {
            nodes: Vec::new(),
            root: 0,
            stacks: stack_size,
            initial_pot: pot_size,
            board: board.clone(),
            bet_sizes,
            rake,
            turn_cards,
            river_cards,
        };

        let street = match board.len() {
            3 => Street::Flop,
            4 => Street::Turn,
            5 => Street::River,
            _ => Street::Flop,
        };

        // In heads-up, OOP acts first postflop
        tree.root = tree.build_node(
            Player::OOP,
            street,
            pot_size,
            stack_size - pot_size / 2.0,
            stack_size - pot_size / 2.0,
            0.0,
            false,
            true,
            board.len() >= 3,
        );

        tree
    }

    fn alloc_node(&mut self, kind: NodeKind) -> usize {
        let id = self.nodes.len();
        self.nodes.push(TreeNode { id, kind });
        id
    }

    /// Build a decision node with available actions and children
    fn build_node(
        &mut self,
        player: Player,
        street: Street,
        pot: f64,
        stack_ip: f64,
        stack_oop: f64,
        last_bet: f64,
        facing_bet: bool,    // is current player facing a bet?
        oop_to_act: bool,    // does OOP go next (vs IP)
        saw_flop: bool,
    ) -> usize {
        let min_stack = stack_ip.min(stack_oop);

        if min_stack <= 0.0 {
            // No more betting possible, go to showdown
            return self.alloc_node(NodeKind::Terminal {
                street,
                winner: TerminalWinner::Showdown,
                pot,
                invested_oop: self.stacks - stack_oop,
                invested_ip: self.stacks - stack_ip,
                saw_flop,
            });
        }

        let actions = self.get_actions(player, street, pot, stack_ip, stack_oop, last_bet, facing_bet);

        // Placeholder: build children after allocating this node
        let node_id = self.nodes.len();
        self.nodes.push(TreeNode {
            id: node_id,
            kind: NodeKind::Action {
                player,
                street,
                pot,
                stack_ip,
                stack_oop,
                last_bet,
                actions: actions.clone(),
                children: vec![],
                node_locked: false,
                locked_strategy: None,
            },
        });

        let mut children = Vec::new();
        for action in &actions {
            let child = self.build_child(
                player,
                street,
                pot,
                stack_ip,
                stack_oop,
                last_bet,
                action.clone(),
                saw_flop,
            );
            children.push(child);
        }

        // Update children list
        if let NodeKind::Action { children: ref mut ch, .. } = self.nodes[node_id].kind {
            *ch = children;
        }

        node_id
    }

    fn build_child(
        &mut self,
        player: Player,
        street: Street,
        pot: f64,
        stack_ip: f64,
        stack_oop: f64,
        last_bet: f64,
        action: ActionKind,
        saw_flop: bool,
    ) -> usize {
        let opponent = player.opponent();

        match action {
            ActionKind::Fold => {
                let winner = match player {
                    Player::OOP => TerminalWinner::IP,
                    Player::IP => TerminalWinner::OOP,
                };
                self.alloc_node(NodeKind::Terminal {
                    street,
                    winner,
                    pot,
                    invested_oop: self.stacks - stack_oop,
                    invested_ip: self.stacks - stack_ip,
                    saw_flop,
                })
            }
            ActionKind::Check => {
                if player == Player::IP || (player == Player::OOP && last_bet == 0.0) {
                    // Both checked: if IP checked (or both checked), go to next street or showdown
                    // OOP checks → IP still to act
                    if player == Player::OOP {
                        self.build_node(
                            Player::IP, street, pot,
                            stack_ip, stack_oop, 0.0, false, false, saw_flop,
                        )
                    } else {
                        // IP checks (both checked) → next street
                        self.next_street_or_terminal(street, pot, stack_ip, stack_oop, saw_flop)
                    }
                } else {
                    self.build_node(
                        opponent, street, pot,
                        stack_ip, stack_oop, 0.0, false, false, saw_flop,
                    )
                }
            }
            ActionKind::Call => {
                let call_amount = last_bet;
                let (new_sip, new_soop) = match player {
                    Player::IP => (stack_ip - call_amount, stack_oop),
                    Player::OOP => (stack_ip, stack_oop - call_amount),
                };
                let new_pot = pot + call_amount;
                self.next_street_or_terminal(street, new_pot, new_sip, new_soop, saw_flop)
            }
            ActionKind::Bet(frac) | ActionKind::Raise(frac) => {
                let bet_size = (pot * frac).min(match player {
                    Player::IP => stack_ip,
                    Player::OOP => stack_oop,
                });
                let (new_sip, new_soop) = match player {
                    Player::IP => (stack_ip - bet_size, stack_oop),
                    Player::OOP => (stack_ip, stack_oop - bet_size),
                };
                let new_pot = pot + bet_size;
                self.build_node(
                    opponent, street, new_pot, new_sip, new_soop,
                    bet_size, true, false, saw_flop,
                )
            }
            ActionKind::AllIn => {
                let allin_amount = match player {
                    Player::IP => stack_ip,
                    Player::OOP => stack_oop,
                };
                let (new_sip, new_soop) = match player {
                    Player::IP => (0.0, stack_oop),
                    Player::OOP => (stack_ip, 0.0),
                };
                let new_pot = pot + allin_amount;
                // Opponent must call or fold
                self.build_node(
                    opponent, street, new_pot, new_sip, new_soop,
                    allin_amount, true, false, saw_flop,
                )
            }
        }
    }

    fn next_street_or_terminal(
        &mut self,
        street: Street,
        pot: f64,
        stack_ip: f64,
        stack_oop: f64,
        saw_flop: bool,
    ) -> usize {
        match street {
            Street::River => {
                self.alloc_node(NodeKind::Terminal {
                    street,
                    winner: TerminalWinner::Showdown,
                    pot,
                    invested_oop: self.stacks - stack_oop,
                    invested_ip: self.stacks - stack_ip,
                    saw_flop,
                })
            }
            Street::Flop => {
                if self.turn_cards.is_empty() {
                    // No turn cards: terminate at flop showdown
                    return self.alloc_node(NodeKind::Terminal {
                        street,
                        winner: TerminalWinner::Showdown,
                        pot,
                        invested_oop: self.stacks - stack_oop,
                        invested_ip: self.stacks - stack_ip,
                        saw_flop,
                    });
                }
                let cards = self.turn_cards.clone();
                self.build_chance_node(Street::Turn, &cards, pot, stack_ip, stack_oop)
            }
            Street::Turn => {
                if self.river_cards.is_empty() {
                    // No river cards: terminate at turn showdown
                    return self.alloc_node(NodeKind::Terminal {
                        street,
                        winner: TerminalWinner::Showdown,
                        pot,
                        invested_oop: self.stacks - stack_oop,
                        invested_ip: self.stacks - stack_ip,
                        saw_flop,
                    });
                }
                let cards = self.river_cards.clone();
                self.build_chance_node(Street::River, &cards, pot, stack_ip, stack_oop)
            }
        }
    }

    fn build_chance_node(
        &mut self,
        next_street: Street,
        cards: &[Card],
        pot: f64,
        stack_ip: f64,
        stack_oop: f64,
    ) -> usize {
        let chance_id = self.nodes.len();
        self.nodes.push(TreeNode {
            id: chance_id,
            kind: NodeKind::Chance {
                street: next_street,
                children: vec![],
            },
        });

        let mut chance_children = Vec::new();
        for &card in cards {
            let action_child = self.build_node(
                Player::OOP, next_street, pot, stack_ip, stack_oop,
                0.0, false, true, true,
            );
            chance_children.push((card, action_child));
        }

        if let NodeKind::Chance { children: ref mut ch, .. } = self.nodes[chance_id].kind {
            *ch = chance_children;
        }
        chance_id
    }

    fn get_actions(
        &self,
        player: Player,
        street: Street,
        pot: f64,
        stack_ip: f64,
        stack_oop: f64,
        last_bet: f64,
        facing_bet: bool,
    ) -> Vec<ActionKind> {
        let min_stack = stack_ip.min(stack_oop);
        let player_stack = match player {
            Player::IP => stack_ip,
            Player::OOP => stack_oop,
        };

        let cfg = match street {
            Street::Flop => &self.bet_sizes.flop,
            Street::Turn => &self.bet_sizes.turn,
            Street::River => &self.bet_sizes.river,
        };

        let mut actions = Vec::new();

        if facing_bet {
            // Can fold, call, raise
            actions.push(ActionKind::Fold);

            let call_amount = last_bet.min(player_stack);
            actions.push(ActionKind::Call);

            // Raises
            let raise_sizes = match player {
                Player::IP => &cfg.ip_raise,
                Player::OOP => &cfg.oop_raise,
            };
            for &frac in raise_sizes {
                let raise_size = pot * frac;
                if raise_size < player_stack && raise_size > last_bet * 2.0 {
                    actions.push(ActionKind::Raise(frac));
                }
            }
            // All-in if meaningful
            if player_stack > last_bet * 2.0 {
                actions.push(ActionKind::AllIn);
            }
        } else {
            // Can check or bet
            actions.push(ActionKind::Check);

            let bet_sizes = if player == Player::OOP && last_bet == 0.0 {
                // OOP donk bet option (only when IP hasn't bet yet this street)
                // For simplicity, we use oop_bet here; donk is separate in full impl
                &cfg.oop_bet
            } else if player == Player::IP {
                &cfg.ip_bet
            } else {
                &cfg.oop_bet
            };

            for &frac in bet_sizes {
                let bet_size = pot * frac;
                if bet_size < player_stack {
                    actions.push(ActionKind::Bet(frac));
                }
            }
            if player_stack > 0.0 {
                actions.push(ActionKind::AllIn);
            }
        }

        actions
    }

    pub fn apply_node_locks(&mut self, locks: &[NodeLockEntry]) -> Vec<usize> {
        let mut locked_ids = Vec::new();
        for lock in locks {
            if let Some(node_id) = self.find_node_by_path(&lock.action_path) {
                let mut strategies: HashMap<[Card; 2], Vec<f64>> = HashMap::new();
                for (hand_str, strat) in &lock.hand_strategies {
                    if let Some(hand) = parse_hand(hand_str) {
                        strategies.insert(hand, strat.clone());
                    }
                }
                if let NodeKind::Action { ref mut node_locked, ref mut locked_strategy, .. } = self.nodes[node_id].kind {
                    *node_locked = true;
                    *locked_strategy = Some(strategies);
                    locked_ids.push(node_id);
                }
            }
        }
        locked_ids
    }

    /// Walk the game tree following an action path, skipping Chance nodes.
    /// Returns the node ID reached after consuming all actions.
    pub fn find_node_by_path(&self, action_path: &[String]) -> Option<usize> {
        let mut current = self.root;
        for action_name in action_path {
            current = self.skip_chance(current);
            match &self.nodes[current].kind {
                NodeKind::Action { actions, children, .. } => {
                    let idx = actions.iter().position(|a| a.to_string() == *action_name)?;
                    current = children[idx];
                }
                _ => return None,
            }
        }
        current = self.skip_chance(current);
        Some(current)
    }

    /// If node_id is a Chance node, return its first child; otherwise return node_id.
    fn skip_chance(&self, node_id: usize) -> usize {
        match &self.nodes[node_id].kind {
            NodeKind::Chance { children, .. } => {
                children.first().map(|(_, child)| *child).unwrap_or(node_id)
            }
            _ => node_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::parse_card;

    #[test]
    fn test_build_simple_tree() {
        let board = vec![
            parse_card("Qs").unwrap(),
            parse_card("8h").unwrap(),
            parse_card("4d").unwrap(),
        ];
        let tree = GameTree::build(
            100.0, 6.5, board,
            FullBetSizeConfig::default(),
            RakeConfig::default(),
            vec![parse_card("Ah").unwrap()],
            vec![parse_card("2s").unwrap()],
        );
        assert!(!tree.nodes.is_empty());
    }

    #[test]
    fn test_root_is_oop_action() {
        let board = vec![
            parse_card("Qs").unwrap(),
            parse_card("8h").unwrap(),
            parse_card("4d").unwrap(),
        ];
        let tree = GameTree::build(
            100.0, 6.5, board,
            FullBetSizeConfig::default(),
            RakeConfig::default(),
            vec![parse_card("Ah").unwrap()],
            vec![],
        );
        match &tree.nodes[tree.root].kind {
            NodeKind::Action { player, .. } => assert_eq!(*player, Player::OOP),
            _ => panic!("Root should be an action node"),
        }
    }

    #[test]
    fn test_root_has_check_action() {
        let board = vec![
            parse_card("Qs").unwrap(),
            parse_card("8h").unwrap(),
            parse_card("4d").unwrap(),
        ];
        let tree = GameTree::build(
            100.0, 6.5, board,
            FullBetSizeConfig::default(),
            RakeConfig::default(),
            vec![], vec![],
        );
        match &tree.nodes[tree.root].kind {
            NodeKind::Action { actions, .. } => {
                assert!(actions.iter().any(|a| *a == ActionKind::Check),
                    "OOP should have check option");
            }
            _ => panic!("Root should be an action node"),
        }
    }

    #[test]
    fn test_river_tree() {
        let board = vec![
            parse_card("Qs").unwrap(),
            parse_card("8h").unwrap(),
            parse_card("4d").unwrap(),
            parse_card("Jc").unwrap(),
            parse_card("2s").unwrap(),
        ];
        let tree = GameTree::build(
            100.0, 6.5, board,
            FullBetSizeConfig::default(),
            RakeConfig::default(),
            vec![], vec![],
        );
        assert!(!tree.nodes.is_empty());
        for node in &tree.nodes {
            if let NodeKind::Chance { .. } = &node.kind {
                panic!("River tree should have no chance nodes");
            }
        }
    }

    #[test]
    fn test_flop_only_no_turn_cards() {
        let board = vec![
            parse_card("Qs").unwrap(),
            parse_card("8h").unwrap(),
            parse_card("4d").unwrap(),
        ];
        let tree = GameTree::build(
            100.0, 6.5, board,
            FullBetSizeConfig::default(),
            RakeConfig::default(),
            vec![], vec![],
        );
        // No chance nodes when turn_cards is empty
        for node in &tree.nodes {
            if let NodeKind::Chance { .. } = &node.kind {
                panic!("Flop-only tree should have no chance nodes");
            }
        }
    }

    #[test]
    fn test_multi_turn_cards() {
        let board = vec![
            parse_card("Qs").unwrap(),
            parse_card("8h").unwrap(),
            parse_card("4d").unwrap(),
        ];
        let turn_cards = vec![
            parse_card("Ah").unwrap(),
            parse_card("7c").unwrap(),
            parse_card("2d").unwrap(),
        ];
        let tree = GameTree::build(
            100.0, 6.5, board,
            FullBetSizeConfig::default(),
            RakeConfig::default(),
            turn_cards, vec![],
        );
        // Should have a Chance node with 3 children for Turn
        let mut found_chance = false;
        for node in &tree.nodes {
            if let NodeKind::Chance { street, children, .. } = &node.kind {
                if *street == Street::Turn {
                    assert_eq!(children.len(), 3);
                    found_chance = true;
                }
            }
        }
        assert!(found_chance, "Should have a Turn chance node");
    }

    #[test]
    fn test_rake_config() {
        let rake = RakeConfig {
            percentage: 0.05,
            cap: 3.0,
            no_flop_no_drop: true,
        };
        assert_eq!(rake.percentage, 0.05);
        assert_eq!(rake.cap, 3.0);
        assert!(rake.no_flop_no_drop);
    }

    #[test]
    fn test_player_opponent() {
        assert_eq!(Player::OOP.opponent(), Player::IP);
        assert_eq!(Player::IP.opponent(), Player::OOP);
    }

    #[test]
    fn test_action_to_string() {
        assert_eq!(ActionKind::Fold.to_string(), "fold");
        assert_eq!(ActionKind::Check.to_string(), "check");
        assert_eq!(ActionKind::Call.to_string(), "call");
        assert_eq!(ActionKind::AllIn.to_string(), "allin");
        assert_eq!(ActionKind::Bet(0.33).to_string(), "bet:0.33");
        assert_eq!(ActionKind::Raise(2.5).to_string(), "raise:2.50");
    }

    #[test]
    fn test_find_node_by_path_check_check_to_turn() {
        let board = vec![
            parse_card("Qs").unwrap(),
            parse_card("8h").unwrap(),
            parse_card("4d").unwrap(),
        ];
        let tree = GameTree::build(
            100.0, 6.5, board,
            FullBetSizeConfig::default(),
            RakeConfig::default(),
            vec![parse_card("Ah").unwrap()],
            vec![parse_card("2s").unwrap()],
        );
        let node_id = tree.find_node_by_path(&[
            "check".to_string(), "check".to_string()
        ]);
        assert!(node_id.is_some(), "Should find node after check-check");
        let nid = node_id.unwrap();
        match &tree.nodes[nid].kind {
            NodeKind::Action { player, street, .. } => {
                assert_eq!(*player, Player::OOP);
                assert_eq!(*street, Street::Turn);
            }
            _ => panic!("Expected Turn OOP Action node"),
        }
    }

    #[test]
    fn test_apply_node_locks_by_path() {
        let board = vec![
            parse_card("Qs").unwrap(),
            parse_card("8h").unwrap(),
            parse_card("4d").unwrap(),
        ];
        let mut tree = GameTree::build(
            100.0, 6.5, board,
            FullBetSizeConfig::default(),
            RakeConfig::default(),
            vec![parse_card("Ah").unwrap()],
            vec![parse_card("2s").unwrap()],
        );
        let lock = NodeLockEntry {
            action_path: vec!["check".to_string(), "check".to_string()],
            street: "turn".to_string(),
            player: "oop".to_string(),
            hand_strategies: HashMap::new(),
        };
        let locked = tree.apply_node_locks(&[lock]);
        assert_eq!(locked.len(), 1);
        let nid = locked[0];
        match &tree.nodes[nid].kind {
            NodeKind::Action { node_locked, street, .. } => {
                assert!(*node_locked);
                assert_eq!(*street, Street::Turn);
            }
            _ => panic!("Expected Action node"),
        }
    }
}
