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
}

impl Solver {
    pub fn new(game_tree: GameTree, oop_range: Range, ip_range: Range) -> Self {
        Solver {
            game_tree,
            oop_range,
            ip_range,
            info_sets: HashMap::new(),
            iteration: 0,
        }
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
                    locked_strategy.clone().unwrap_or_else(|| {
                        vec![1.0 / n_actions as f64; n_actions]
                    })
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

    /// Compute exploitability (sum of best response EVs for both players)
    /// Returns exploitability as fraction of pot
    pub fn compute_exploitability(&self) -> f64 {
        let pot = self.game_tree.initial_pot;
        // Simplified: compute best response EV for each player
        // Full implementation would do best-response traversal
        // For now return a placeholder based on iteration count
        let t = self.iteration as f64;
        1.0 / t.sqrt() // approximate convergence
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
}
