/// Discounted CFR (DCFR) implementation
/// Parameters: α=1.5, β=0.0, γ=2.0
///
/// The solver operates on a postflop game tree with two players (OOP/IP).
/// Each player has a range of hands. The strategy is stored per hand per node.

use std::collections::HashMap;
use crate::cards::{Card, card_bit};
use crate::game_tree::{GameTree, NodeKind, Player, Street, TerminalWinner, RakeConfig};
use crate::hand_eval::best_hand;
use crate::ranges::{Range, normalize};

// DCFR parameters
const ALPHA: f64 = 1.5;
const BETA: f64 = 0.0;
const GAMMA: f64 = 2.0;

/// Key for the information set: node_id + hand
type InfoKey = (usize, [Card; 2]);

pub struct InfoSetData {
    pub cumulative_regrets: Vec<f64>,
    pub cumulative_strategy: Vec<f64>,
    pub current_strategy: Vec<f64>,
}

impl InfoSetData {
    pub fn new(n_actions: usize) -> Self {
        let uniform = vec![1.0 / n_actions as f64; n_actions];
        InfoSetData {
            cumulative_regrets: vec![0.0; n_actions],
            cumulative_strategy: vec![0.0; n_actions],
            current_strategy: uniform,
        }
    }

    pub fn get_strategy(&self) -> Vec<f64> {
        let pos_regrets: Vec<f64> = self.cumulative_regrets.iter()
            .map(|&r| r.max(0.0))
            .collect();
        normalize(&pos_regrets)
    }

    pub fn get_average_strategy(&self) -> Vec<f64> {
        normalize(&self.cumulative_strategy)
    }
}

pub struct Solver {
    pub game_tree: GameTree,
    pub oop_range: Range,
    pub ip_range: Range,
    pub info_sets: HashMap<InfoKey, InfoSetData>,
    pub iteration: u32,
    pub user_locked_nodes: Vec<usize>,
}

impl Solver {
    pub fn new(game_tree: GameTree, oop_range: Range, ip_range: Range) -> Self {
        Solver {
            game_tree,
            oop_range,
            ip_range,
            info_sets: HashMap::new(),
            iteration: 0,
            user_locked_nodes: Vec::new(),
        }
    }

    pub fn set_user_locked_nodes(&mut self, nodes: Vec<usize>) {
        self.user_locked_nodes = nodes;
    }

    /// Run a single DCFR iteration
    pub fn iterate(&mut self) {
        self.iteration += 1;
        let t = self.iteration as f64;

        // Collect all hand pairs to iterate over
        let board = self.game_tree.board.clone();
        let board_mask: u64 = board.iter().fold(0u64, |m, &c| m | card_bit(c));

        let oop_hands: Vec<([Card; 2], f64)> = self.oop_range.iter()
            .filter(|&(&h, _)| {
                card_bit(h[0]) & board_mask == 0 && card_bit(h[1]) & board_mask == 0
            })
            .map(|(&h, &f)| (h, f))
            .collect();

        let ip_hands: Vec<([Card; 2], f64)> = self.ip_range.iter()
            .filter(|&(&h, _)| {
                card_bit(h[0]) & board_mask == 0 && card_bit(h[1]) & board_mask == 0
            })
            .map(|(&h, &f)| (h, f))
            .collect();

        // For each OOP hand, traverse the tree with all IP hands
        // This is the standard "range vs range" traversal approach
        for &(oop_hand, oop_freq) in &oop_hands {
            for &(ip_hand, ip_freq) in &ip_hands {
                // Skip if hands conflict
                if hands_conflict(oop_hand, ip_hand) { continue; }

                let dead_mask = board_mask | card_bit(oop_hand[0]) | card_bit(oop_hand[1])
                    | card_bit(ip_hand[0]) | card_bit(ip_hand[1]);

                // OOP traversal
                self.traverse(
                    self.game_tree.root,
                    Player::OOP,
                    oop_hand,
                    ip_hand,
                    1.0 * oop_freq,
                    1.0 * ip_freq,
                    &board,
                    dead_mask,
                    t,
                );

                // IP traversal
                self.traverse(
                    self.game_tree.root,
                    Player::IP,
                    oop_hand,
                    ip_hand,
                    1.0 * oop_freq,
                    1.0 * ip_freq,
                    &board,
                    dead_mask,
                    t,
                );
            }
        }

        // Apply DCFR discounting
        self.apply_discounting(t);
    }

    fn traverse(
        &mut self,
        node_id: usize,
        traverser: Player,
        oop_hand: [Card; 2],
        ip_hand: [Card; 2],
        reach_oop: f64,
        reach_ip: f64,
        board: &[Card],
        dead_mask: u64,
        t: f64,
    ) -> f64 {
        let node_kind = self.game_tree.nodes[node_id].kind.clone();

        match node_kind {
            NodeKind::Terminal { pot, winner, saw_flop, .. } => {
                self.compute_terminal_ev(
                    &winner, pot, traverser, oop_hand, ip_hand, board,
                    &self.game_tree.rake.clone(), saw_flop,
                )
            }

            NodeKind::Chance { children, .. } => {
                // For now, use the single subtree child (card dealing handled externally)
                if let Some((_, child_id)) = children.first() {
                    self.traverse(
                        *child_id, traverser, oop_hand, ip_hand,
                        reach_oop, reach_ip, board, dead_mask, t,
                    )
                } else {
                    0.0
                }
            }

            NodeKind::Action {
                player,
                actions,
                children,
                node_locked,
                locked_strategy,
                ..
            } => {
                let hand = if player == Player::OOP { oop_hand } else { ip_hand };
                let n_actions = actions.len();

                // Get or create info set
                let key: InfoKey = (node_id, hand);

                // Get current strategy
                let strategy = if node_locked {
                    locked_strategy
                        .as_ref()
                        .and_then(|map| map.get(&hand))
                        .cloned()
                        .unwrap_or_else(|| vec![1.0 / n_actions as f64; n_actions])
                } else {
                    if !self.info_sets.contains_key(&key) {
                        self.info_sets.insert(key, InfoSetData::new(n_actions));
                    }
                    self.info_sets[&key].get_strategy()
                };

                if player == traverser {
                    // Compute counterfactual values for each action
                    let opponent_reach = if traverser == Player::OOP { reach_ip } else { reach_oop };
                    let mut action_evs = vec![0.0f64; n_actions];

                    for (i, &child_id) in children.iter().enumerate() {
                        let (new_reach_oop, new_reach_ip) = if traverser == Player::OOP {
                            (reach_oop * strategy[i], reach_ip)
                        } else {
                            (reach_oop, reach_ip * strategy[i])
                        };
                        action_evs[i] = self.traverse(
                            child_id, traverser, oop_hand, ip_hand,
                            new_reach_oop, new_reach_ip, board, dead_mask, t,
                        );
                    }

                    let node_ev: f64 = action_evs.iter().zip(strategy.iter())
                        .map(|(&ev, &s)| ev * s)
                        .sum();

                    // Update regrets
                    if !node_locked {
                        let info = self.info_sets.entry(key).or_insert_with(|| InfoSetData::new(n_actions));
                        for i in 0..n_actions {
                            let regret = opponent_reach * (action_evs[i] - node_ev);
                            info.cumulative_regrets[i] += regret;
                        }

                        // Update cumulative strategy
                        let my_reach = if traverser == Player::OOP { reach_oop } else { reach_ip };
                        for i in 0..n_actions {
                            info.cumulative_strategy[i] += my_reach * strategy[i];
                        }
                    }

                    node_ev
                } else {
                    // Opponent node: weight by opponent strategy
                    let mut ev = 0.0f64;
                    for (i, &child_id) in children.iter().enumerate() {
                        let (new_reach_oop, new_reach_ip) = if player == Player::OOP {
                            (reach_oop * strategy[i], reach_ip)
                        } else {
                            (reach_oop, reach_ip * strategy[i])
                        };
                        ev += strategy[i] * self.traverse(
                            child_id, traverser, oop_hand, ip_hand,
                            new_reach_oop, new_reach_ip, board, dead_mask, t,
                        );
                    }

                    // Update cumulative strategy for opponent
                    if !node_locked {
                        let my_reach = if player == Player::OOP { reach_oop } else { reach_ip };
                        if let Some(info) = self.info_sets.get_mut(&key) {
                            for i in 0..n_actions {
                                info.cumulative_strategy[i] += my_reach * strategy[i];
                            }
                        }
                    }

                    ev
                }
            }
        }
    }

    fn compute_terminal_ev(
        &self,
        winner: &TerminalWinner,
        pot: f64,
        traverser: Player,
        oop_hand: [Card; 2],
        ip_hand: [Card; 2],
        board: &[Card],
        rake: &RakeConfig,
        saw_flop: bool,
    ) -> f64 {
        let rake_amount = if !saw_flop && rake.no_flop_no_drop {
            0.0
        } else {
            (pot * rake.percentage).min(rake.cap)
        };
        let net_pot = pot - rake_amount;

        match winner {
            TerminalWinner::OOP => {
                // IP folded, OOP wins
                match traverser {
                    Player::OOP => net_pot / 2.0,
                    Player::IP => -net_pot / 2.0,
                }
            }
            TerminalWinner::IP => {
                // OOP folded, IP wins
                match traverser {
                    Player::OOP => -net_pot / 2.0,
                    Player::IP => net_pot / 2.0,
                }
            }
            TerminalWinner::Showdown => {
                let score_oop = best_hand(oop_hand, board);
                let score_ip = best_hand(ip_hand, board);
                let half_pot = net_pot / 2.0;
                match traverser {
                    Player::OOP => {
                        if score_oop > score_ip { half_pot }
                        else if score_ip > score_oop { -half_pot }
                        else { 0.0 }
                    }
                    Player::IP => {
                        if score_ip > score_oop { half_pot }
                        else if score_oop > score_ip { -half_pot }
                        else { 0.0 }
                    }
                }
            }
        }
    }

    fn apply_discounting(&mut self, t: f64) {
        let pos_weight = t.powf(ALPHA) / (t.powf(ALPHA) + 1.0);
        let neg_weight = if BETA >= 0.0 {
            t.powf(BETA) / (t.powf(BETA) + 1.0)
        } else {
            0.0
        };
        let strat_weight = (t / (t + 1.0)).powf(GAMMA);

        for info in self.info_sets.values_mut() {
            for r in &mut info.cumulative_regrets {
                if *r >= 0.0 {
                    *r *= pos_weight;
                } else {
                    *r *= neg_weight;
                }
            }
            for s in &mut info.cumulative_strategy {
                *s *= strat_weight;
            }
        }
    }

    /// Compute exploitability via best response traversal.
    /// exploitability = (BR_OOP + BR_IP) / pot
    /// where BR_P is the max EV player P can achieve against the opponent's
    /// average strategy. Returns exploitability as a fraction of pot.
    /// At Nash equilibrium this equals 0.
    pub fn compute_exploitability(&self) -> f64 {
        let pot = self.game_tree.initial_pot;
        if pot <= 0.0 { return 0.0; }

        let board = &self.game_tree.board;
        let board_mask: u64 = board.iter().fold(0u64, |m, &c| card_bit(c) | m);

        let br_oop = self.best_response_value(Player::OOP, board, board_mask);
        let br_ip = self.best_response_value(Player::IP, board, board_mask);

        ((br_oop + br_ip) / pot).max(0.0)
    }

    /// Compute best response EV for one player against the opponent's
    /// average strategy, averaged over all valid hand matchups.
    fn best_response_value(&self, traverser: Player, board: &[Card], board_mask: u64) -> f64 {
        let (t_range, o_range) = match traverser {
            Player::OOP => (&self.oop_range, &self.ip_range),
            Player::IP => (&self.ip_range, &self.oop_range),
        };

        let t_hands: Vec<([Card; 2], f64)> = t_range.iter()
            .filter(|&(&h, _)| card_bit(h[0]) & board_mask == 0 && card_bit(h[1]) & board_mask == 0)
            .map(|(&h, &f)| (h, f))
            .collect();

        let o_hands: Vec<([Card; 2], f64)> = o_range.iter()
            .filter(|&(&h, _)| card_bit(h[0]) & board_mask == 0 && card_bit(h[1]) & board_mask == 0)
            .map(|(&h, &f)| (h, f))
            .collect();

        let mut total_value = 0.0;
        let mut total_weight = 0.0;

        for &(t_hand, t_freq) in &t_hands {
            // Build initial opponent reach probabilities
            let mut opp_reach: HashMap<[Card; 2], f64> = HashMap::new();
            for &(o_hand, o_freq) in &o_hands {
                if !hands_conflict(t_hand, o_hand) {
                    opp_reach.insert(o_hand, o_freq);
                }
            }
            if opp_reach.is_empty() { continue; }

            let opp_total: f64 = opp_reach.values().sum();
            let br_val = self.br_traverse(
                self.game_tree.root, traverser, t_hand, &opp_reach, board,
            );

            total_value += t_freq * br_val;
            total_weight += t_freq * opp_total;
        }

        if total_weight > 0.0 { total_value / total_weight } else { 0.0 }
    }

    /// Best response traversal for a single traverser hand.
    ///
    /// - Traverser's nodes: pick the max-EV action (best response).
    /// - Opponent's nodes: distribute opponent reach by their average strategy.
    /// - Terminals: sum payoffs weighted by opponent reach.
    fn br_traverse(
        &self,
        node_id: usize,
        traverser: Player,
        t_hand: [Card; 2],
        opp_reach: &HashMap<[Card; 2], f64>,
        board: &[Card],
    ) -> f64 {
        let node = &self.game_tree.nodes[node_id];
        match &node.kind {
            NodeKind::Terminal { winner, pot, saw_flop, .. } => {
                let rake = &self.game_tree.rake;
                let mut ev = 0.0;
                for (&opp_hand, &reach) in opp_reach {
                    if reach <= 0.0 { continue; }
                    let (oop_h, ip_h) = match traverser {
                        Player::OOP => (t_hand, opp_hand),
                        Player::IP => (opp_hand, t_hand),
                    };
                    ev += reach * self.compute_terminal_ev(
                        winner, *pot, traverser, oop_h, ip_h, board, rake, *saw_flop,
                    );
                }
                ev
            }

            NodeKind::Chance { children, .. } => {
                children.first()
                    .map(|(_, cid)| self.br_traverse(*cid, traverser, t_hand, opp_reach, board))
                    .unwrap_or(0.0)
            }

            NodeKind::Action { player, actions, children, node_locked, locked_strategy, .. } => {
                let n_actions = actions.len();

                if *player == traverser {
                    // Best response: pick the action with maximum EV
                    children.iter()
                        .map(|&cid| self.br_traverse(cid, traverser, t_hand, opp_reach, board))
                        .fold(f64::NEG_INFINITY, f64::max)
                } else {
                    // Opponent's node: split reach by opponent's per-hand strategy
                    let mut action_reach: Vec<HashMap<[Card; 2], f64>> =
                        vec![HashMap::new(); n_actions];

                    for (&opp_hand, &reach) in opp_reach {
                        if reach <= 0.0 { continue; }

                        let strat = if *node_locked {
                            locked_strategy.as_ref()
                                .and_then(|m| m.get(&opp_hand))
                                .cloned()
                                .unwrap_or_else(|| vec![1.0 / n_actions as f64; n_actions])
                        } else {
                            self.info_sets.get(&(node_id, opp_hand))
                                .map(|info| info.get_average_strategy())
                                .unwrap_or_else(|| vec![1.0 / n_actions as f64; n_actions])
                        };

                        for (a, &prob) in strat.iter().enumerate() {
                            if prob > 0.0 {
                                *action_reach[a].entry(opp_hand).or_insert(0.0) += reach * prob;
                            }
                        }
                    }

                    children.iter().enumerate()
                        .filter(|(a, _)| !action_reach[*a].is_empty())
                        .map(|(a, &cid)| self.br_traverse(cid, traverser, t_hand, &action_reach[a], board))
                        .sum()
                }
            }
        }
    }

    /// Extract solution: average strategies for all info sets
    pub fn extract_strategies(&self) -> HashMap<(usize, String), Vec<f64>> {
        let mut result = HashMap::new();
        for (&(node_id, hand), info) in &self.info_sets {
            let key = (node_id, crate::cards::hand_to_string(hand));
            result.insert(key, info.get_average_strategy());
        }
        result
    }

    // ─── Chained Nodelock Solve ──────────────────────────────────────────────

    /// Solve with chained nodelock across streets.
    ///
    /// Flow per round:
    ///   Phase 1: Unlock chain-locked nodes, solve full tree → Turn+ strategies develop
    ///   Phase 2: Lock all Turn+ strategies from Phase 1 results
    ///   Phase 3: Solve full tree with Turn+ locked → only Flop adjusts
    /// Repeat for `rounds` iterations.
    pub fn solve_with_chained_nodelock(
        &mut self,
        iters_per_phase: u32,
        rounds: u32,
    ) {
        let turn_roots = self.find_street_boundary_nodes(Street::Turn);
        if turn_roots.is_empty() {
            // No Turn subtrees, fall back to normal solve
            for _ in 0..(iters_per_phase * 2) {
                self.iterate();
            }
            return;
        }

        for _round in 0..rounds {
            // Phase 1: Solve full tree freely (user locks still apply)
            self.unlock_chained_locks(&turn_roots);
            for _ in 0..iters_per_phase {
                self.iterate();
            }

            // Phase 2: Lock Turn+ strategies from Phase 1 results
            self.lock_downstream_strategies(&turn_roots);

            // Phase 3: Solve with Turn+ locked → Flop strategies adjust
            for _ in 0..iters_per_phase {
                self.iterate();
            }
        }

        // Final unlock of chain-locked nodes (user locks remain)
        self.unlock_chained_locks(&turn_roots);
    }

    /// Find all Action nodes that are direct children of Chance nodes
    /// transitioning to `target_street`.
    pub fn find_street_boundary_nodes(&self, target_street: Street) -> Vec<usize> {
        let mut roots = Vec::new();
        for node in &self.game_tree.nodes {
            if let NodeKind::Chance { street, children, .. } = &node.kind {
                if *street == target_street {
                    for (_, child_id) in children {
                        roots.push(*child_id);
                    }
                }
            }
        }
        roots
    }

    /// Collect all Action node IDs within a subtree rooted at `root`.
    fn collect_subtree_action_nodes(&self, root: usize) -> Vec<usize> {
        let mut result = Vec::new();
        let mut stack = vec![root];
        while let Some(node_id) = stack.pop() {
            match &self.game_tree.nodes[node_id].kind {
                NodeKind::Action { children, .. } => {
                    result.push(node_id);
                    stack.extend(children);
                }
                NodeKind::Chance { children, .. } => {
                    for (_, child) in children {
                        stack.push(*child);
                    }
                }
                NodeKind::Terminal { .. } => {}
            }
        }
        result
    }

    /// Lock all Turn+ Action node strategies based on current average strategies.
    /// Skips user-locked nodes.
    fn lock_downstream_strategies(&mut self, subtree_roots: &[usize]) {
        for &root in subtree_roots {
            let action_nodes = self.collect_subtree_action_nodes(root);
            for node_id in action_nodes {
                if self.user_locked_nodes.contains(&node_id) {
                    continue;
                }
                let mut hand_strats: HashMap<[Card; 2], Vec<f64>> = HashMap::new();
                for (&(nid, hand), info) in &self.info_sets {
                    if nid == node_id {
                        hand_strats.insert(hand, info.get_average_strategy());
                    }
                }
                if !hand_strats.is_empty() {
                    if let NodeKind::Action {
                        ref mut node_locked,
                        ref mut locked_strategy,
                        ..
                    } = self.game_tree.nodes[node_id].kind {
                        *node_locked = true;
                        *locked_strategy = Some(hand_strats);
                    }
                }
            }
        }
    }

    /// Unlock chain-locked nodes. User-locked nodes are preserved.
    fn unlock_chained_locks(&mut self, subtree_roots: &[usize]) {
        for &root in subtree_roots {
            let action_nodes = self.collect_subtree_action_nodes(root);
            for node_id in action_nodes {
                if self.user_locked_nodes.contains(&node_id) {
                    continue;
                }
                if let NodeKind::Action {
                    ref mut node_locked,
                    ref mut locked_strategy,
                    ..
                } = self.game_tree.nodes[node_id].kind {
                    *node_locked = false;
                    *locked_strategy = None;
                }
            }
        }
    }
}

fn hands_conflict(h1: [Card; 2], h2: [Card; 2]) -> bool {
    h1[0] == h2[0] || h1[0] == h2[1] || h1[1] == h2[0] || h1[1] == h2[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::parse_card;
    use crate::game_tree::{GameTree, FullBetSizeConfig, RakeConfig};
    use crate::ranges::parse_range;

    #[test]
    fn test_solver_runs() {
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
        let oop_range = parse_range("AA,KK,QQ,AKs,AKo");
        let ip_range = parse_range("AA,KK,QQ,AKs,AKo");

        let mut solver = Solver::new(tree, oop_range, ip_range);
        for _ in 0..10 {
            solver.iterate();
        }
        assert_eq!(solver.iteration, 10);
    }

    #[test]
    fn test_chained_nodelock_solve() {
        use crate::game_tree::NodeLockEntry;

        let board = vec![
            parse_card("Qs").unwrap(),
            parse_card("8h").unwrap(),
            parse_card("4d").unwrap(),
        ];
        let mut tree = GameTree::build(
            100.0, 6.5, board,
            FullBetSizeConfig::default(),
            RakeConfig::default(),
        );

        // Lock Turn node after check-check: OOP always checks with AsAh
        let lock = NodeLockEntry {
            action_path: vec!["check".to_string(), "check".to_string()],
            street: "turn".to_string(),
            player: "oop".to_string(),
            hand_strategies: {
                let mut m = std::collections::HashMap::new();
                // 5 actions at Turn root: check + 3 bets + allin
                m.insert("AsAh".to_string(), vec![1.0, 0.0, 0.0, 0.0, 0.0]);
                m
            },
        };
        let locked_ids = tree.apply_node_locks(&[lock]);
        assert!(!locked_ids.is_empty(), "Should lock at least one node");

        let oop_range = parse_range("AA,KK");
        let ip_range = parse_range("AA,KK");

        let mut solver = Solver::new(tree, oop_range, ip_range);
        solver.set_user_locked_nodes(locked_ids.clone());

        // Run chained solve: 10 iters per phase, 2 rounds = 40 total iterations
        solver.solve_with_chained_nodelock(10, 2);

        assert!(solver.iteration > 0, "Solver should have run iterations");
        assert_eq!(solver.iteration, 40, "Should have run 10*2*2 = 40 iterations");

        // Verify Turn boundary nodes exist
        let turn_roots = solver.find_street_boundary_nodes(Street::Turn);
        assert!(!turn_roots.is_empty(), "Should find Turn boundary nodes");

        // Verify user-locked node retains its lock after chained solve
        let locked_node_id = locked_ids[0];
        match &solver.game_tree.nodes[locked_node_id].kind {
            NodeKind::Action { node_locked, locked_strategy, .. } => {
                assert!(*node_locked, "User-locked node should remain locked");
                assert!(locked_strategy.is_some(), "Locked strategy should exist");
            }
            _ => panic!("Expected Action node"),
        }

        // Verify non-user-locked Turn nodes are unlocked after solve
        for &turn_root in &turn_roots {
            if !locked_ids.contains(&turn_root) {
                if let NodeKind::Action { node_locked, .. } = &solver.game_tree.nodes[turn_root].kind {
                    assert!(!node_locked, "Non-user Turn nodes should be unlocked after solve");
                }
            }
        }
    }

    #[test]
    fn test_best_response_exploitability() {
        // River game with small ranges — fast to solve, verifiable properties.
        // Board: Ks 9h 5d 2c 3h
        // Rankings: KK(trips) > AA(pair A) > QQ(pair Q) > JJ(pair J)
        // OOP: AA(6),QQ(6)  IP: KK(3, Ks on board),JJ(6)
        let board = vec![
            parse_card("Ks").unwrap(),
            parse_card("9h").unwrap(),
            parse_card("5d").unwrap(),
            parse_card("2c").unwrap(),
            parse_card("3h").unwrap(),
        ];
        let tree = GameTree::build(
            100.0, 10.0, board,
            FullBetSizeConfig::default(),
            RakeConfig::default(),
        );
        let oop_range = parse_range("AA,QQ");
        let ip_range = parse_range("KK,JJ");

        let mut solver = Solver::new(tree, oop_range, ip_range);

        // Before any iterations: exploitability of uniform strategy should be non-negative
        let expl_0 = solver.compute_exploitability();
        assert!(expl_0 >= 0.0, "Exploitability at iter 0 should be >= 0, got {}", expl_0);

        // Run 20 iterations
        for _ in 0..20 {
            solver.iterate();
        }
        let expl_20 = solver.compute_exploitability();
        assert!(expl_20 >= 0.0, "Exploitability at iter 20 should be >= 0, got {}", expl_20);

        // Run 80 more (total 100)
        for _ in 0..80 {
            solver.iterate();
        }
        let expl_100 = solver.compute_exploitability();
        assert!(expl_100 >= 0.0, "Exploitability at iter 100 should be >= 0, got {}", expl_100);

        // Exploitability should decrease with more iterations
        assert!(expl_100 < expl_20,
            "Exploitability should decrease: {:.6} (100 iters) vs {:.6} (20 iters)",
            expl_100, expl_20);

        // After 100 DCFR iterations on a simple river game, should be well below 10% pot
        assert!(expl_100 < 0.10,
            "Exploitability should be < 10% pot after 100 iters, got {:.4}%",
            expl_100 * 100.0);

        eprintln!(
            "Exploitability: iter0={:.4}% iter20={:.4}% iter100={:.4}%",
            expl_0 * 100.0, expl_20 * 100.0, expl_100 * 100.0
        );
    }
}
