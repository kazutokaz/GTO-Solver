mod cards;
mod hand_eval;
mod ranges;
mod game_tree;
mod cfr;
mod io;

use std::io::Read;
use std::time::Instant;

use cards::parse_card;
use game_tree::{GameTree, FullBetSizeConfig, RakeConfig};
use ranges::parse_range;
use cfr::Solver;
use io::{SolveInput, SolveOutput, SolutionNode, convert_bet_sizes, convert_rake, convert_node_locks};

fn main() {
    // Read JSON from stdin
    let mut input_str = String::new();
    std::io::stdin().read_to_string(&mut input_str)
        .expect("Failed to read stdin");

    let input: SolveInput = match serde_json::from_str(&input_str) {
        Ok(v) => v,
        Err(e) => {
            let err = serde_json::json!({
                "status": "failed",
                "error": format!("Invalid input JSON: {}", e)
            });
            println!("{}", serde_json::to_string_pretty(&err).unwrap());
            std::process::exit(1);
        }
    };

    let start = Instant::now();

    // Parse board
    let board: Vec<cards::Card> = input.game.board.iter()
        .filter_map(|s| parse_card(s))
        .collect();

    if board.len() < 3 {
        let err = serde_json::json!({
            "job_id": input.job_id,
            "status": "failed",
            "error": "Board must have at least 3 cards"
        });
        println!("{}", serde_json::to_string_pretty(&err).unwrap());
        std::process::exit(1);
    }

    // Parse ranges
    let oop_range = parse_range(&input.game.players.oop.range);
    let ip_range = parse_range(&input.game.players.ip.range);

    // Build game tree
    let bet_sizes = convert_bet_sizes(input.bet_sizes);
    let rake = convert_rake(input.rake);
    let node_locks = convert_node_locks(input.node_locks);

    let mut tree = GameTree::build(
        input.game.stack_size,
        input.game.pot_size,
        board,
        bet_sizes,
        rake,
    );

    tree.apply_node_locks(&node_locks);

    // Solve config
    let solve_cfg = input.solve_config.unwrap_or(io::SolveConfigInput {
        max_iterations: None,
        target_exploitability: None,
        timeout_seconds: None,
    });
    let max_iterations = solve_cfg.max_iterations.unwrap_or(1000);
    let target_exploitability = solve_cfg.target_exploitability.unwrap_or(0.003);
    let timeout_secs = solve_cfg.timeout_seconds.unwrap_or(300) as u64;

    // Run solver
    let mut solver = Solver::new(tree, oop_range, ip_range);
    let check_interval = 100u32;

    loop {
        solver.iterate();

        let elapsed = start.elapsed().as_secs();
        if elapsed >= timeout_secs {
            eprintln!("Timeout reached after {} iterations", solver.iteration);
            break;
        }

        if solver.iteration >= max_iterations {
            break;
        }

        if solver.iteration % check_interval == 0 {
            let exploitability = solver.compute_exploitability();
            eprintln!(
                "Iteration {}: exploitability = {:.6}",
                solver.iteration, exploitability
            );
            if exploitability <= target_exploitability {
                eprintln!("Converged!");
                break;
            }
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    let exploitability = solver.compute_exploitability();

    // Build output
    let solution = build_solution_node(&solver, solver.game_tree.root);

    let output = SolveOutput {
        job_id: input.job_id,
        status: "completed".to_string(),
        exploitability,
        iterations: solver.iteration,
        elapsed_seconds: elapsed,
        solution,
    };

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

fn build_solution_node(solver: &Solver, node_id: usize) -> SolutionNode {
    use game_tree::NodeKind;
    use cards::hand_to_string;

    let node = &solver.game_tree.nodes[node_id];

    match &node.kind {
        NodeKind::Action { player, actions, children, .. } => {
            let player_str = match player {
                game_tree::Player::OOP => "oop",
                game_tree::Player::IP => "ip",
            }.to_string();

            let action_names: Vec<String> = actions.iter().map(|a| a.to_string()).collect();

            let range = match player {
                game_tree::Player::OOP => &solver.oop_range,
                game_tree::Player::IP => &solver.ip_range,
            };

            let board = &solver.game_tree.board;
            let board_mask: u64 = board.iter().fold(0u64, |m, &c| cards::card_bit(c) | m);

            let mut strategy_map = std::collections::HashMap::new();
            let mut ev_map = std::collections::HashMap::new();

            for (&hand, &freq) in range {
                if freq <= 0.0 { continue; }
                if cards::card_bit(hand[0]) & board_mask != 0 { continue; }
                if cards::card_bit(hand[1]) & board_mask != 0 { continue; }

                let key = (node_id, hand);
                let strat = solver.info_sets.get(&key)
                    .map(|info| info.get_average_strategy())
                    .unwrap_or_else(|| vec![1.0 / actions.len() as f64; actions.len()]);

                strategy_map.insert(hand_to_string(hand), strat);
                ev_map.insert(hand_to_string(hand), 0.0);
            }

            // Build children (limit depth for output size)
            let mut children_map = std::collections::HashMap::new();
            for (action, &child_id) in actions.iter().zip(children.iter()).take(3) {
                let child_node = build_solution_node(solver, child_id);
                children_map.insert(action.to_string(), child_node);
            }

            SolutionNode {
                player: player_str,
                actions: action_names,
                strategy: strategy_map,
                ev: ev_map,
                children: children_map,
            }
        }
        NodeKind::Terminal { winner, .. } => {
            let player_str = match winner {
                game_tree::TerminalWinner::OOP => "terminal:oop_wins",
                game_tree::TerminalWinner::IP => "terminal:ip_wins",
                game_tree::TerminalWinner::Showdown => "terminal:showdown",
            };
            SolutionNode::empty(player_str)
        }
        NodeKind::Chance { .. } => {
            SolutionNode::empty("chance")
        }
    }
}
