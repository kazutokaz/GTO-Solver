/// Discounted CFR (DCFR) implementation
/// Parameters: α=1.5, β=0.0, γ=2.0
///
/// The solver operates on a postflop game tree with two players (OOP/IP).
/// Each player has a range of hands. The strategy is stored per hand per node.

use std::collections::HashMap;
use dashmap::DashMap;
use rayon::prelude::*;
use crate::cards::{Card, card_bit};
use crate::game_tree::{GameTree, NodeKind, Player, Street, TerminalWinner};
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
}

impl InfoSetData {
    pub fn new(n_actions: usize) -> Self {
        InfoSetData {
            cumulative_regrets: vec![0.0; n_actions],
            cumulative_strategy: vec![0.0; n_actions],
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

/// Compact board key for score cache lookup (7 bytes, 0xFF-padded)
fn board_to_key(board: &[Card]) -> [Card; 7] {
    let mut key = [0xFF; 7];
    key[..board.len()].copy_from_slice(board);
    key
}

/// O(n log n) showdown EV for a single traverser hand against opponent reach vector.
/// Sort opponents by hand strength, then use cumulative sums to compute EV.
fn showdown_ev_sorted(
    t_score: u32,
    opp_reach: &[([Card; 2], f64)],
    half_pot: f64,
    scores: Option<&HashMap<[Card; 2], u32>>,
    board: &[Card],
) -> f64 {
    let mut opp_scored: Vec<(u32, f64)> = Vec::with_capacity(opp_reach.len());
    for &(hand, reach) in opp_reach {
        if reach <= 0.0 { continue; }
        let score = scores.and_then(|s| s.get(&hand).copied())
            .unwrap_or_else(|| best_hand(hand, board));
        opp_scored.push((score, reach));
    }

    if opp_scored.is_empty() { return 0.0; }

    opp_scored.sort_unstable_by_key(|&(s, _)| s);

    let lo = opp_scored.partition_point(|&(s, _)| s < t_score);
    let hi = opp_scored.partition_point(|&(s, _)| s <= t_score);

    let reach_below: f64 = opp_scored[..lo].iter().map(|&(_, r)| r).sum();
    let reach_above: f64 = opp_scored[hi..].iter().map(|&(_, r)| r).sum();

    half_pot * (reach_below - reach_above)
}

/// Collect all unique boards that have showdown terminals
fn collect_showdown_boards(tree: &GameTree, node_id: usize, board: &[Card], out: &mut Vec<Vec<Card>>) {
    match &tree.nodes[node_id].kind {
        NodeKind::Terminal { winner: TerminalWinner::Showdown, .. } => {
            out.push(board.to_vec());
        }
        NodeKind::Terminal { .. } => {}
        NodeKind::Action { children, .. } => {
            for &cid in children { collect_showdown_boards(tree, cid, board, out); }
        }
        NodeKind::Chance { children, .. } => {
            for &(card, cid) in children {
                let mut nb = board.to_vec();
                nb.push(card);
                collect_showdown_boards(tree, cid, &nb, out);
            }
        }
    }
}

/// Precompute best_hand scores for all hands on all unique boards
fn precompute_hand_scores(
    tree: &GameTree, oop_range: &Range, ip_range: &Range,
) -> HashMap<[Card; 7], HashMap<[Card; 2], u32>> {
    let mut boards = Vec::new();
    collect_showdown_boards(tree, tree.root, &tree.board, &mut boards);
    boards.sort();
    boards.dedup();

    let mut cache = HashMap::new();
    for board in &boards {
        let board_mask: u64 = board.iter().fold(0u64, |m, &c| card_bit(c) | m);
        let mut scores: HashMap<[Card; 2], u32> = HashMap::new();
        for (&hand, _) in oop_range.iter().chain(ip_range.iter()) {
            if scores.contains_key(&hand) { continue; }
            if card_bit(hand[0]) & board_mask != 0 { continue; }
            if card_bit(hand[1]) & board_mask != 0 { continue; }
            scores.insert(hand, best_hand(hand, board));
        }
        cache.insert(board_to_key(board), scores);
    }
    cache
}

pub struct Solver {
    pub game_tree: GameTree,
    pub oop_range: Range,
    pub ip_range: Range,
    pub info_sets: DashMap<InfoKey, InfoSetData>,
    pub iteration: u32,
    pub user_locked_nodes: Vec<usize>,
    score_cache: HashMap<[Card; 7], HashMap<[Card; 2], u32>>,
}

impl Solver {
    pub fn new(game_tree: GameTree, oop_range: Range, ip_range: Range) -> Self {
        let score_cache = precompute_hand_scores(&game_tree, &oop_range, &ip_range);
        Solver {
            game_tree,
            oop_range,
            ip_range,
            info_sets: DashMap::new(),
            iteration: 0,
            user_locked_nodes: Vec::new(),
            score_cache,
        }
    }

    pub fn set_user_locked_nodes(&mut self, nodes: Vec<usize>) {
        self.user_locked_nodes = nodes;
    }

    /// Run a single DCFR iteration using parallel reach-vector traversal.
    /// OOP and IP hands are each parallelized via rayon.
    pub fn iterate(&mut self) {
        self.iteration += 1;
        let t = self.iteration as f64;

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

        // Parallel traversal — shared borrow of self via DashMap interior mutability
        {
            let this = &*self;

            // OOP traversal: each OOP hand in parallel
            oop_hands.par_iter().for_each(|&(oop_hand, oop_freq)| {
                let oop_mask = card_bit(oop_hand[0]) | card_bit(oop_hand[1]);
                let ip_reach: Vec<([Card; 2], f64)> = ip_hands.iter()
                    .filter(|&&(h, _)| (card_bit(h[0]) | card_bit(h[1])) & oop_mask == 0)
                    .cloned()
                    .collect();
                if ip_reach.is_empty() { return; }

                this.traverse_reach(
                    this.game_tree.root, Player::OOP, oop_hand, oop_freq,
                    &ip_reach, &board, t,
                );
            });

            // IP traversal: each IP hand in parallel
            ip_hands.par_iter().for_each(|&(ip_hand, ip_freq)| {
                let ip_mask = card_bit(ip_hand[0]) | card_bit(ip_hand[1]);
                let oop_reach: Vec<([Card; 2], f64)> = oop_hands.iter()
                    .filter(|&&(h, _)| (card_bit(h[0]) | card_bit(h[1])) & ip_mask == 0)
                    .cloned()
                    .collect();
                if oop_reach.is_empty() { return; }

                this.traverse_reach(
                    this.game_tree.root, Player::IP, ip_hand, ip_freq,
                    &oop_reach, &board, t,
                );
            });
        }

        // Apply DCFR discounting
        self.apply_discounting(t);
    }

    /// Reach-vector traversal: traverse the tree once per traverser hand,
    /// processing all opponent hands simultaneously via their reach vector.
    /// Uses &self with DashMap for thread-safe concurrent access.
    fn traverse_reach(
        &self,
        node_id: usize,
        traverser: Player,
        t_hand: [Card; 2],
        t_reach: f64,
        opp_reach: &[([Card; 2], f64)],
        board: &[Card],
        t: f64,
    ) -> f64 {
        let node = &self.game_tree.nodes[node_id];
        match &node.kind {
            NodeKind::Terminal { pot, winner, saw_flop, .. } => {
                let rake = &self.game_tree.rake;
                let rake_amount = if !*saw_flop && rake.no_flop_no_drop { 0.0 }
                                  else { (*pot * rake.percentage).min(rake.cap) };
                let net_pot = *pot - rake_amount;
                let half_pot = net_pot / 2.0;

                match winner {
                    TerminalWinner::Showdown => {
                        let bkey = board_to_key(board);
                        let scores = self.score_cache.get(&bkey);
                        let t_score = scores.and_then(|s| s.get(&t_hand).copied())
                            .unwrap_or_else(|| best_hand(t_hand, board));
                        showdown_ev_sorted(t_score, opp_reach, half_pot, scores, board)
                    }
                    _ => {
                        let total_reach: f64 = opp_reach.iter().map(|&(_, r)| r).sum();
                        let payoff = match (winner, traverser) {
                            (TerminalWinner::OOP, Player::OOP) => half_pot,
                            (TerminalWinner::OOP, Player::IP) => -half_pot,
                            (TerminalWinner::IP, Player::IP) => half_pot,
                            (TerminalWinner::IP, Player::OOP) => -half_pot,
                            _ => 0.0,
                        };
                        payoff * total_reach
                    }
                }
            }

            NodeKind::Chance { children, .. } => {
                let t_mask = card_bit(t_hand[0]) | card_bit(t_hand[1]);
                let mut total_ev = 0.0;
                let mut valid_count = 0u32;
                for &(card, child_id) in children {
                    let cmask = card_bit(card);
                    if cmask & t_mask != 0 { continue; }
                    let filtered: Vec<([Card; 2], f64)> = opp_reach.iter()
                        .filter(|&&(h, _)| (card_bit(h[0]) | card_bit(h[1])) & cmask == 0)
                        .cloned()
                        .collect();
                    if filtered.is_empty() { continue; }
                    let blen = board.len();
                    let mut eb = [0u8; 7];
                    eb[..blen].copy_from_slice(board);
                    eb[blen] = card;
                    total_ev += self.traverse_reach(
                        child_id, traverser, t_hand, t_reach,
                        &filtered, &eb[..blen+1], t,
                    );
                    valid_count += 1;
                }
                if valid_count > 0 { total_ev / valid_count as f64 } else { 0.0 }
            }

            NodeKind::Action {
                player,
                actions,
                children,
                node_locked,
                locked_strategy,
                ..
            } => {
                let n_actions = actions.len();

                if *player == traverser {
                    // Traverser's node: compute counterfactual values
                    let key: InfoKey = (node_id, t_hand);

                    // Read strategy (DashMap guard dropped before recursion)
                    let strategy = if *node_locked {
                        locked_strategy
                            .as_ref()
                            .and_then(|map| map.get(&t_hand))
                            .cloned()
                            .unwrap_or_else(|| vec![1.0 / n_actions as f64; n_actions])
                    } else {
                        self.info_sets.entry(key)
                            .or_insert_with(|| InfoSetData::new(n_actions))
                            .get_strategy()
                    };

                    // Recurse (no DashMap guards held)
                    let mut action_evs = vec![0.0f64; n_actions];
                    for (i, &child_id) in children.iter().enumerate() {
                        action_evs[i] = self.traverse_reach(
                            child_id, traverser, t_hand, t_reach * strategy[i],
                            opp_reach, board, t,
                        );
                    }

                    let node_ev: f64 = action_evs.iter().zip(strategy.iter())
                        .map(|(&ev, &s)| ev * s)
                        .sum();

                    // Write back regrets and cumulative strategy
                    if !*node_locked {
                        let mut info = self.info_sets.entry(key)
                            .or_insert_with(|| InfoSetData::new(n_actions));
                        for i in 0..n_actions {
                            info.cumulative_regrets[i] += action_evs[i] - node_ev;
                        }
                        for i in 0..n_actions {
                            info.cumulative_strategy[i] += t_reach * strategy[i];
                        }
                    }

                    node_ev
                } else {
                    // Opponent's node: split reach by per-hand strategy
                    let mut action_opp_reach: Vec<Vec<([Card; 2], f64)>> =
                        vec![Vec::new(); n_actions];

                    for &(opp_hand, reach) in opp_reach {
                        if reach <= 0.0 { continue; }

                        let opp_key: InfoKey = (node_id, opp_hand);

                        // Read strategy and update cumulative in one lock acquisition
                        let strat = if *node_locked {
                            locked_strategy
                                .as_ref()
                                .and_then(|m| m.get(&opp_hand))
                                .cloned()
                                .unwrap_or_else(|| vec![1.0 / n_actions as f64; n_actions])
                        } else {
                            let mut entry = self.info_sets.entry(opp_key)
                                .or_insert_with(|| InfoSetData::new(n_actions));
                            let s = entry.get_strategy();
                            for a in 0..n_actions {
                                entry.cumulative_strategy[a] += reach * s[a];
                            }
                            s
                            // guard dropped here
                        };

                        for (a, &prob) in strat.iter().enumerate() {
                            if prob > 0.0 {
                                action_opp_reach[a].push((opp_hand, reach * prob));
                            }
                        }
                    }

                    // Recurse (no DashMap guards held)
                    let mut ev = 0.0;
                    for (a, &child_id) in children.iter().enumerate() {
                        if action_opp_reach[a].is_empty() { continue; }
                        ev += self.traverse_reach(
                            child_id, traverser, t_hand, t_reach,
                            &action_opp_reach[a], board, t,
                        );
                    }

                    ev
                }
            }
        }
    }

    fn apply_discounting(&self, t: f64) {
        let pos_weight = t.powf(ALPHA) / (t.powf(ALPHA) + 1.0);
        let neg_weight = if BETA >= 0.0 {
            t.powf(BETA) / (t.powf(BETA) + 1.0)
        } else {
            0.0
        };
        let strat_weight = (t / (t + 1.0)).powf(GAMMA);

        self.info_sets.iter_mut().for_each(|mut entry| {
            for r in &mut entry.cumulative_regrets {
                if *r >= 0.0 {
                    *r *= pos_weight;
                } else {
                    *r *= neg_weight;
                }
            }
            for s in &mut entry.cumulative_strategy {
                *s *= strat_weight;
            }
        });
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
            let t_mask = card_bit(t_hand[0]) | card_bit(t_hand[1]);
            let opp_reach: Vec<([Card; 2], f64)> = o_hands.iter()
                .filter(|&&(h, _)| (card_bit(h[0]) | card_bit(h[1])) & t_mask == 0)
                .cloned()
                .collect();
            if opp_reach.is_empty() { continue; }

            let opp_total: f64 = opp_reach.iter().map(|&(_, r)| r).sum();
            let br_val = self.br_traverse(
                self.game_tree.root, traverser, t_hand, &opp_reach, board,
            );

            total_value += t_freq * br_val;
            total_weight += t_freq * opp_total;
        }

        if total_weight > 0.0 { total_value / total_weight } else { 0.0 }
    }

    /// Best response traversal for a single traverser hand.
    /// Uses Vec-based opponent reach for cache-friendly processing.
    fn br_traverse(
        &self,
        node_id: usize,
        traverser: Player,
        t_hand: [Card; 2],
        opp_reach: &[([Card; 2], f64)],
        board: &[Card],
    ) -> f64 {
        let node = &self.game_tree.nodes[node_id];
        match &node.kind {
            NodeKind::Terminal { winner, pot, saw_flop, .. } => {
                let rake = &self.game_tree.rake;
                let rake_amount = if !saw_flop && rake.no_flop_no_drop { 0.0 }
                                  else { (*pot * rake.percentage).min(rake.cap) };
                let net_pot = *pot - rake_amount;
                let half_pot = net_pot / 2.0;

                match winner {
                    TerminalWinner::Showdown => {
                        let bkey = board_to_key(board);
                        let scores = self.score_cache.get(&bkey);
                        let t_score = scores.and_then(|s| s.get(&t_hand).copied())
                            .unwrap_or_else(|| best_hand(t_hand, board));
                        showdown_ev_sorted(t_score, opp_reach, half_pot, scores, board)
                    }
                    _ => {
                        let total_reach: f64 = opp_reach.iter().map(|&(_, r)| r).sum();
                        let payoff = match (winner, traverser) {
                            (TerminalWinner::OOP, Player::OOP) => half_pot,
                            (TerminalWinner::OOP, Player::IP) => -half_pot,
                            (TerminalWinner::IP, Player::IP) => half_pot,
                            (TerminalWinner::IP, Player::OOP) => -half_pot,
                            _ => 0.0,
                        };
                        payoff * total_reach
                    }
                }
            }

            NodeKind::Chance { children, .. } => {
                let t_mask = card_bit(t_hand[0]) | card_bit(t_hand[1]);
                let mut total_ev = 0.0;
                let mut valid_count = 0u32;
                for &(card, child_id) in children {
                    let cmask = card_bit(card);
                    if cmask & t_mask != 0 { continue; }
                    let filtered: Vec<([Card; 2], f64)> = opp_reach.iter()
                        .filter(|&&(h, _)| (card_bit(h[0]) | card_bit(h[1])) & cmask == 0)
                        .cloned()
                        .collect();
                    if filtered.is_empty() { continue; }
                    let blen = board.len();
                    let mut eb = [0u8; 7];
                    eb[..blen].copy_from_slice(board);
                    eb[blen] = card;
                    total_ev += self.br_traverse(child_id, traverser, t_hand, &filtered, &eb[..blen+1]);
                    valid_count += 1;
                }
                if valid_count > 0 { total_ev / valid_count as f64 } else { 0.0 }
            }

            NodeKind::Action { player, actions, children, node_locked, locked_strategy, .. } => {
                let n_actions = actions.len();

                if *player == traverser {
                    children.iter()
                        .map(|&cid| self.br_traverse(cid, traverser, t_hand, opp_reach, board))
                        .fold(f64::NEG_INFINITY, f64::max)
                } else {
                    let mut action_reach: Vec<Vec<([Card; 2], f64)>> =
                        vec![Vec::new(); n_actions];

                    for &(opp_hand, reach) in opp_reach {
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
                                action_reach[a].push((opp_hand, reach * prob));
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
        for entry in self.info_sets.iter() {
            let &(node_id, hand) = entry.key();
            result.insert(
                (node_id, crate::cards::hand_to_string(hand)),
                entry.value().get_average_strategy(),
            );
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
                for entry in self.info_sets.iter() {
                    let &(nid, hand) = entry.key();
                    if nid == node_id {
                        hand_strats.insert(hand, entry.value().get_average_strategy());
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
            vec![parse_card("Ah").unwrap()],
            vec![parse_card("2s").unwrap()],
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
            vec![parse_card("Jc").unwrap()],
            vec![parse_card("2s").unwrap()],
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
            vec![], vec![],
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
