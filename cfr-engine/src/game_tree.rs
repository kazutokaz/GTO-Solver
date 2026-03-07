/// Game tree representation for heads-up postflop poker
/// OOP = Out of Position (acts first on each street)
/// IP  = In Position (acts second on each street)

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::cards::{Card, parse_card};
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
        locked_strategy: Option<Vec<f64>>,
    },
    /// Terminal: showdown or fold
    Terminal {
        street: Street,
        winner: TerminalWinner,
        pot: f64,
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
}

impl GameTree {
    pub fn build(
        stack_size: f64,
        pot_size: f64,
        board: Vec<Card>,
        bet_sizes: FullBetSizeConfig,
        rake: RakeConfig,
    ) -> Self {
        let mut tree = GameTree {
            nodes: Vec::new(),
            root: 0,
            stacks: stack_size,
            initial_pot: pot_size,
            board: board.clone(),
            bet_sizes,
            rake,
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
                    saw_flop,
                })
            }
            Street::Flop | Street::Turn => {
                let next_street = if street == Street::Flop { Street::Turn } else { Street::River };
                // Chance node for the next card
                let chance_id = self.nodes.len();
                self.nodes.push(TreeNode {
                    id: chance_id,
                    kind: NodeKind::Chance {
                        street: next_street,
                        children: vec![],
                    },
                });

                // Build one representative subtree per runout card
                // (actual card dealt during solve time via card removal)
                let action_child = self.build_node(
                    Player::OOP, next_street, pot, stack_ip, stack_oop,
                    0.0, false, true, true,
                );

                // For tree structure, use a placeholder single child
                // The CFR engine handles card dealing via reach probability scaling
                if let NodeKind::Chance { children: ref mut ch, .. } = self.nodes[chance_id].kind {
                    // Use card 0 as placeholder; actual dealing handled in solver
                    ch.push((0, action_child));
                }
                chance_id
            }
        }
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

    pub fn apply_node_locks(&mut self, locks: &[NodeLockEntry]) {
        // For now, apply locks by action path matching (simplified)
        // Full implementation would traverse the tree matching action paths
        // This is a placeholder that will be expanded in the CFR solver
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
        );
        // Root should be an OOP action node (OOP acts first postflop)
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
        );
        assert!(!tree.nodes.is_empty());
        // On river there should be no chance nodes
        for node in &tree.nodes {
            if let NodeKind::Chance { .. } = &node.kind {
                panic!("River tree should have no chance nodes");
            }
        }
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
}
