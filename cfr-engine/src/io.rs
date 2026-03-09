/// JSON I/O structures matching the CLI interface spec from the design doc

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

// ─── Input ────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SolveInput {
    pub job_id: String,
    pub game: GameConfig,
    pub bet_sizes: Option<BetSizesInput>,
    pub rake: Option<RakeInput>,
    pub node_locks: Option<Vec<NodeLockInput>>,
    pub solve_config: Option<SolveConfigInput>,
}

#[derive(Debug, Deserialize)]
pub struct GameConfig {
    pub stack_size: f64,
    pub pot_size: f64,
    pub board: Vec<String>,
    pub street: Option<String>,
    pub turn_cards: Option<Vec<String>>,
    pub river_cards: Option<Vec<String>>,
    pub players: PlayersConfig,
}

#[derive(Debug, Deserialize)]
pub struct PlayersConfig {
    pub oop: PlayerConfig,
    pub ip: PlayerConfig,
}

#[derive(Debug, Deserialize)]
pub struct PlayerConfig {
    pub range: String,
}

#[derive(Debug, Deserialize)]
pub struct BetSizesInput {
    pub flop: Option<StreetBetSizesInput>,
    pub turn: Option<StreetBetSizesInput>,
    pub river: Option<StreetBetSizesInput>,
}

#[derive(Debug, Deserialize)]
pub struct StreetBetSizesInput {
    pub ip_bet: Option<Vec<f64>>,
    pub oop_bet: Option<Vec<f64>>,
    pub ip_raise: Option<Vec<f64>>,
    pub oop_raise: Option<Vec<f64>>,
    pub oop_donk: Option<Vec<f64>>,
}

#[derive(Debug, Deserialize)]
pub struct RakeInput {
    pub percentage: f64,
    pub cap: f64,
    pub no_flop_no_drop: bool,
}

#[derive(Debug, Deserialize)]
pub struct NodeLockInput {
    pub action_path: Vec<String>,
    pub street: String,
    pub player: String,
    pub hand_strategies: HashMap<String, Vec<f64>>,
}

#[derive(Debug, Deserialize)]
pub struct SolveConfigInput {
    pub max_iterations: Option<u32>,
    pub target_exploitability: Option<f64>,
    pub timeout_seconds: Option<u32>,
}

// ─── Output ───────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct SolveOutput {
    pub job_id: String,
    pub status: String,
    pub exploitability: f64,
    pub iterations: u32,
    pub elapsed_seconds: f64,
    pub solution: SolutionNode,
}

#[derive(Debug, Serialize)]
pub struct SolutionNode {
    pub player: String,
    pub actions: Vec<String>,
    /// hand -> action probabilities
    pub strategy: HashMap<String, Vec<f64>>,
    /// hand -> EV
    pub ev: HashMap<String, f64>,
    pub children: HashMap<String, SolutionNode>,
}

impl SolutionNode {
    pub fn empty(player: &str) -> Self {
        SolutionNode {
            player: player.to_string(),
            actions: vec![],
            strategy: HashMap::new(),
            ev: HashMap::new(),
            children: HashMap::new(),
        }
    }
}

// ─── Conversion helpers ───────────────────────────────────────────────────────

use crate::game_tree::{FullBetSizeConfig, BetSizeConfig, RakeConfig, NodeLockEntry};

pub fn convert_bet_sizes(input: Option<BetSizesInput>) -> FullBetSizeConfig {
    let mut cfg = FullBetSizeConfig::default();
    if let Some(bs) = input {
        if let Some(f) = bs.flop { apply_street(&mut cfg.flop, f); }
        if let Some(t) = bs.turn { apply_street(&mut cfg.turn, t); }
        if let Some(r) = bs.river { apply_street(&mut cfg.river, r); }
    }
    cfg
}

fn apply_street(cfg: &mut BetSizeConfig, input: StreetBetSizesInput) {
    if let Some(v) = input.ip_bet { cfg.ip_bet = v; }
    if let Some(v) = input.oop_bet { cfg.oop_bet = v; }
    if let Some(v) = input.ip_raise { cfg.ip_raise = v; }
    if let Some(v) = input.oop_raise { cfg.oop_raise = v; }
    if let Some(v) = input.oop_donk { cfg.oop_donk = v; }
}

pub fn convert_rake(input: Option<RakeInput>) -> RakeConfig {
    match input {
        None => RakeConfig::default(),
        Some(r) => RakeConfig {
            percentage: r.percentage,
            cap: r.cap,
            no_flop_no_drop: r.no_flop_no_drop,
        },
    }
}

pub fn convert_node_locks(input: Option<Vec<NodeLockInput>>) -> Vec<NodeLockEntry> {
    input.unwrap_or_default().into_iter().map(|n| NodeLockEntry {
        action_path: n.action_path,
        street: n.street,
        player: n.player,
        hand_strategies: n.hand_strategies,
    }).collect()
}
