/// 5-card hand evaluator (for showdown equity calculation)
/// Returns a score: higher = better hand
/// Category (bits 20-22): 0=high card, 1=pair, 2=two pair, 3=trips,
///                        4=straight, 5=flush, 6=full house, 7=quads, 8=straight flush

use crate::cards::{Card, rank, suit};

pub fn evaluate_5(cards: &[Card; 5]) -> u32 {
    let ranks: [u8; 5] = [rank(cards[0]), rank(cards[1]), rank(cards[2]), rank(cards[3]), rank(cards[4])];
    let suits: [u8; 5] = [suit(cards[0]), suit(cards[1]), suit(cards[2]), suit(cards[3]), suit(cards[4])];

    let is_flush = suits[0] == suits[1] && suits[1] == suits[2]
        && suits[2] == suits[3] && suits[3] == suits[4];

    // Sort ranks descending
    let mut sorted = ranks;
    sorted.sort_unstable_by(|a, b| b.cmp(a));

    // Check straight
    let is_straight = {
        let normal = sorted[0] == sorted[1] + 1
            && sorted[1] == sorted[2] + 1
            && sorted[2] == sorted[3] + 1
            && sorted[3] == sorted[4] + 1;
        // A-2-3-4-5 wheel
        let wheel = sorted[0] == 12 && sorted[1] == 3 && sorted[2] == 2
            && sorted[3] == 1 && sorted[4] == 0;
        (normal, wheel)
    };

    // Count rank frequencies
    let mut freq = [0u8; 13];
    for &r in &ranks {
        freq[r as usize] += 1;
    }
    let mut quads = 0u8;
    let mut trips = 0u8;
    let mut pairs = 0u8;
    let mut quad_rank = 0u8;
    let mut trip_rank = 0u8;
    let mut pair_ranks = [0u8; 2];
    for r in (0..13u8).rev() {
        match freq[r as usize] {
            4 => { quads += 1; quad_rank = r; }
            3 => { trips += 1; trip_rank = r; }
            2 => { pair_ranks[pairs as usize] = r; pairs += 1; }
            _ => {}
        }
    }

    // Kickers (ranks not in a made hand)
    let kickers: Vec<u8> = sorted.iter().copied().collect();

    if is_flush && (is_straight.0 || is_straight.1) {
        // Straight flush
        let high = if is_straight.1 { 3 } else { sorted[0] };
        return (8 << 20) | (high as u32);
    }
    if quads > 0 {
        let kicker = kickers.iter().find(|&&r| r != quad_rank).copied().unwrap_or(0);
        return (7 << 20) | ((quad_rank as u32) << 4) | (kicker as u32);
    }
    if trips > 0 && pairs > 0 {
        return (6 << 20) | ((trip_rank as u32) << 4) | (pair_ranks[0] as u32);
    }
    if is_flush {
        let val: u32 = kickers.iter().enumerate()
            .fold(0u32, |acc, (i, &r)| acc | ((r as u32) << (16 - i * 4)));
        return (5 << 20) | val;
    }
    if is_straight.0 {
        return (4 << 20) | (sorted[0] as u32);
    }
    if is_straight.1 {
        return (4 << 20) | 3u32; // wheel, high=5 (index 3)
    }
    if trips > 0 {
        let kk: Vec<u8> = kickers.iter().copied().filter(|&r| r != trip_rank).collect();
        return (3 << 20) | ((trip_rank as u32) << 8) | ((kk[0] as u32) << 4) | (kk[1] as u32);
    }
    if pairs == 2 {
        let kicker = kickers.iter().copied().find(|&r| r != pair_ranks[0] && r != pair_ranks[1]).unwrap_or(0);
        return (2 << 20) | ((pair_ranks[0] as u32) << 8) | ((pair_ranks[1] as u32) << 4) | (kicker as u32);
    }
    if pairs == 1 {
        let kk: Vec<u8> = kickers.iter().copied().filter(|&r| r != pair_ranks[0]).collect();
        return (1 << 20) | ((pair_ranks[0] as u32) << 12) | ((kk[0] as u32) << 8)
            | ((kk[1] as u32) << 4) | (kk[2] as u32);
    }
    // High card
    kickers.iter().enumerate()
        .fold(0u32, |acc, (i, &r)| acc | ((r as u32) << (16 - i * 4)))
}

/// Best 5 out of 7 cards
pub fn evaluate_7(cards: &[Card; 7]) -> u32 {
    let mut best = 0u32;
    // C(7,5) = 21 combinations
    for i in 0..7 {
        for j in (i + 1)..7 {
            // Use the 5 cards that are NOT i and j
            let five: Vec<Card> = (0..7).filter(|&k| k != i && k != j).map(|k| cards[k]).collect();
            let score = evaluate_5(&[five[0], five[1], five[2], five[3], five[4]]);
            if score > best {
                best = score;
            }
        }
    }
    best
}

/// Best 5 from hole cards (2) + board (3-5)
pub fn best_hand(hole: [crate::cards::Card; 2], board: &[crate::cards::Card]) -> u32 {
    let all: Vec<Card> = [hole[0], hole[1]].iter().chain(board.iter()).copied().collect();
    let n = all.len();
    assert!(n >= 5 && n <= 7);
    if n == 7 {
        return evaluate_7(&[all[0], all[1], all[2], all[3], all[4], all[5], all[6]]);
    }
    if n == 6 {
        let mut best = 0u32;
        for skip in 0..6 {
            let five: Vec<Card> = (0..6).filter(|&k| k != skip).map(|k| all[k]).collect();
            let s = evaluate_5(&[five[0], five[1], five[2], five[3], five[4]]);
            if s > best { best = s; }
        }
        return best;
    }
    evaluate_5(&[all[0], all[1], all[2], all[3], all[4]])
}

/// Compute equity for two ranges over a given board using Monte Carlo or exact enumeration
/// Returns (oop_equity, ip_equity)
pub fn compute_equity(
    oop_hand: [Card; 2],
    ip_hand: [Card; 2],
    board: &[Card],
) -> (f64, f64) {
    let dead: Vec<Card> = [oop_hand[0], oop_hand[1], ip_hand[0], ip_hand[1]]
        .iter().chain(board.iter()).copied().collect();

    let remaining_cards: Vec<Card> = (0u8..52)
        .filter(|c| !dead.contains(c))
        .collect();

    let need = 5 - board.len();

    if need == 0 {
        let s_oop = best_hand(oop_hand, board);
        let s_ip = best_hand(ip_hand, board);
        return if s_oop > s_ip { (1.0, 0.0) }
               else if s_ip > s_oop { (0.0, 1.0) }
               else { (0.5, 0.5) };
    }

    let n = remaining_cards.len();
    let mut wins_oop = 0.0f64;
    let mut total = 0.0f64;

    // Enumerate run-outs
    enumerate_runouts(&remaining_cards, need, &mut |runout: &[Card]| {
        let mut full_board = board.to_vec();
        full_board.extend_from_slice(runout);
        let s_oop = best_hand(oop_hand, &full_board);
        let s_ip = best_hand(ip_hand, &full_board);
        if s_oop > s_ip { wins_oop += 1.0; }
        else if s_oop == s_ip { wins_oop += 0.5; }
        total += 1.0;
    });

    if total == 0.0 { return (0.5, 0.5); }
    let eq_oop = wins_oop / total;
    (eq_oop, 1.0 - eq_oop)
}

fn enumerate_runouts(cards: &[Card], need: usize, callback: &mut impl FnMut(&[Card])) {
    if need == 0 {
        callback(&[]);
        return;
    }
    let n = cards.len();
    match need {
        1 => {
            for i in 0..n {
                callback(&[cards[i]]);
            }
        }
        2 => {
            for i in 0..n {
                for j in (i+1)..n {
                    callback(&[cards[i], cards[j]]);
                }
            }
        }
        _ => {
            // Generic recursion for need=3+ (rare, only on flop)
            enumerate_runouts_rec(cards, need, &mut vec![], callback);
        }
    }
}

fn enumerate_runouts_rec(cards: &[Card], need: usize, current: &mut Vec<Card>, callback: &mut impl FnMut(&[Card])) {
    if need == 0 {
        callback(current);
        return;
    }
    let n = cards.len();
    let start = if current.is_empty() { 0 } else {
        let last = *current.last().unwrap();
        cards.iter().position(|&c| c == last).map(|i| i + 1).unwrap_or(0)
    };
    for i in start..n {
        current.push(cards[i]);
        let remaining: Vec<Card> = cards[i+1..].to_vec();
        enumerate_runouts_rec(&remaining, need - 1, current, callback);
        current.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::parse_card;

    #[test]
    fn test_royal_flush() {
        let cards = [
            parse_card("As").unwrap(),
            parse_card("Ks").unwrap(),
            parse_card("Qs").unwrap(),
            parse_card("Js").unwrap(),
            parse_card("Ts").unwrap(),
        ];
        let score = evaluate_5(&cards);
        assert_eq!(score >> 20, 8); // straight flush
    }

    #[test]
    fn test_equity_aces_vs_kings() {
        let oo = [parse_card("Ah").unwrap(), parse_card("As").unwrap()];
        let ip = [parse_card("Kh").unwrap(), parse_card("Ks").unwrap()];
        let board = vec![
            parse_card("2h").unwrap(),
            parse_card("7d").unwrap(),
            parse_card("Tc").unwrap(),
        ];
        let (eq_oo, _eq_ip) = compute_equity(oo, ip, &board);
        // AA should be ~80% vs KK on low board
        assert!(eq_oo > 0.75, "AA equity was {}", eq_oo);
    }

    #[test]
    fn test_hand_rankings_order() {
        let p = |s: &str| parse_card(s).unwrap();
        // High card < Pair < Two pair < Trips < Straight < Flush < Full house < Quads < Straight flush
        let high_card = evaluate_5(&[p("2s"), p("4h"), p("6d"), p("8c"), p("Ts")]);
        let pair = evaluate_5(&[p("2s"), p("2h"), p("6d"), p("8c"), p("Ts")]);
        let two_pair = evaluate_5(&[p("2s"), p("2h"), p("6d"), p("6c"), p("Ts")]);
        let trips = evaluate_5(&[p("2s"), p("2h"), p("2d"), p("8c"), p("Ts")]);
        let straight = evaluate_5(&[p("5s"), p("6h"), p("7d"), p("8c"), p("9s")]);
        let flush = evaluate_5(&[p("2s"), p("4s"), p("6s"), p("8s"), p("Ts")]);
        let full_house = evaluate_5(&[p("2s"), p("2h"), p("2d"), p("8c"), p("8s")]);
        let quads = evaluate_5(&[p("2s"), p("2h"), p("2d"), p("2c"), p("Ts")]);
        let str_flush = evaluate_5(&[p("5s"), p("6s"), p("7s"), p("8s"), p("9s")]);

        assert!(high_card < pair);
        assert!(pair < two_pair);
        assert!(two_pair < trips);
        assert!(trips < straight);
        assert!(straight < flush);
        assert!(flush < full_house);
        assert!(full_house < quads);
        assert!(quads < str_flush);
    }

    #[test]
    fn test_wheel_straight() {
        let p = |s: &str| parse_card(s).unwrap();
        let wheel = evaluate_5(&[p("As"), p("2h"), p("3d"), p("4c"), p("5s")]);
        assert_eq!(wheel >> 20, 4); // straight category
        // Wheel should be lower than 6-high straight
        let six_high = evaluate_5(&[p("2s"), p("3h"), p("4d"), p("5c"), p("6s")]);
        assert!(wheel < six_high);
    }

    #[test]
    fn test_best_hand_7_cards() {
        let p = |s: &str| parse_card(s).unwrap();
        // Full board: hole = AA, board has A + two pair → should make full house
        let hole = [p("Ah"), p("As")];
        let board = vec![p("Ad"), p("Kh"), p("Kd"), p("7c"), p("2s")];
        let score = best_hand(hole, &board);
        assert_eq!(score >> 20, 6); // full house
    }

    #[test]
    fn test_showdown_river_exact() {
        let p = |s: &str| parse_card(s).unwrap();
        let h1 = [p("Ah"), p("Kh")];
        let h2 = [p("Qh"), p("Jh")];
        let board = vec![p("2s"), p("5d"), p("9c"), p("Th"), p("3h")];
        // Both have flush draws but only with hearts on board
        // h1: Ah Kh + Th 3h = A-high flush
        // h2: Qh Jh + Th 3h = Q-high flush
        let (eq1, eq2) = compute_equity(h1, h2, &board);
        assert_eq!(eq1, 1.0); // AK flush beats QJ flush
        assert_eq!(eq2, 0.0);
    }
}
