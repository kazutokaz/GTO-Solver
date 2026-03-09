/// Range parsing and representation
/// Supports formats:
///   AA, KK, QQ         - pocket pairs
///   AKs, AQs           - suited hands
///   AKo, AQo           - offsuit hands
///   AK (= AKs + AKo)   - any combo
///   AKs:0.5            - 50% frequency
///   AA:0.75            - 75% frequency
///   Specific combos: AhKh, AsKs

use std::collections::HashMap;
use crate::cards::{Card, rank, suit, make_card, all_hands, canonical_hand};

/// Frequency for each canonical hand [c1,c2] where c1 < c2
pub type Range = HashMap<[Card; 2], f64>;

pub fn parse_range(s: &str) -> Range {
    let mut range = Range::new();
    for token in s.split(',') {
        let token = token.trim();
        if token.is_empty() { continue; }
        parse_token(token, &mut range);
    }
    range
}

fn parse_token(token: &str, range: &mut Range) {
    // Split off frequency suffix ":0.5"
    let (combo_str, freq) = if let Some(pos) = token.rfind(':') {
        let f: f64 = token[pos+1..].parse().unwrap_or(1.0);
        (&token[..pos], f)
    } else {
        (token, 1.0)
    };

    let freq = freq.clamp(0.0, 1.0);

    // Check for dash range: "22-TT", "A2s-A9s", "A2o-A9o"
    if let Some(dash_pos) = combo_str.find('-') {
        let left = &combo_str[..dash_pos];
        let right = &combo_str[dash_pos+1..];
        parse_dash_range(left, right, freq, range);
        return;
    }

    // Check for plus suffix: "22+", "A2s+", "ATo+"
    if combo_str.ends_with('+') {
        let base = &combo_str[..combo_str.len()-1];
        parse_plus_range(base, freq, range);
        return;
    }

    // Specific combo like "AhKh"
    if combo_str.len() == 4 {
        if let Some(hand) = parse_specific_combo(combo_str) {
            range.insert(hand, freq);
            return;
        }
    }

    add_single_combo(combo_str, freq, range);
}

/// Parse "22+", "A2s+", "ATo+" etc.
fn parse_plus_range(base: &str, freq: f64, range: &mut Range) {
    let bytes = base.as_bytes();
    if bytes.len() < 2 { return; }

    let r1 = match char_to_rank(bytes[0] as char) { Some(r) => r, None => return };
    let r2 = match char_to_rank(bytes[1] as char) { Some(r) => r, None => return };

    if r1 == r2 {
        // Pocket pair plus: "22+" = 22,33,...,AA
        for rank in r1..=12 {
            let combo = format!("{}{}", rank_to_char(rank), rank_to_char(rank));
            add_single_combo(&combo, freq, range);
        }
    } else {
        // Non-pair plus: "A2s+" = A2s,A3s,...,AKs
        let suited_flag = if bytes.len() >= 3 {
            match bytes[2] as char {
                's' | 'S' => "s",
                'o' | 'O' => "o",
                _ => "",
            }
        } else { "" };

        let (high, low) = if r1 > r2 { (r1, r2) } else { (r2, r1) };
        // Increment the lower card up to high-1
        for low_rank in low..high {
            let combo = format!("{}{}{}", rank_to_char(high), rank_to_char(low_rank), suited_flag);
            add_single_combo(&combo, freq, range);
        }
    }
}

/// Parse "22-TT", "A2s-A9s", "K2s-K9s" etc.
fn parse_dash_range(left: &str, right: &str, freq: f64, range: &mut Range) {
    let lb = left.as_bytes();
    let rb = right.as_bytes();
    if lb.len() < 2 || rb.len() < 2 { return; }

    let l1 = match char_to_rank(lb[0] as char) { Some(r) => r, None => return };
    let l2 = match char_to_rank(lb[1] as char) { Some(r) => r, None => return };
    let r1 = match char_to_rank(rb[0] as char) { Some(r) => r, None => return };
    let r2 = match char_to_rank(rb[1] as char) { Some(r) => r, None => return };

    if l1 == l2 && r1 == r2 {
        // Pair range: "22-TT"
        let lo = l1.min(r1);
        let hi = l1.max(r1);
        for rank in lo..=hi {
            let combo = format!("{}{}", rank_to_char(rank), rank_to_char(rank));
            add_single_combo(&combo, freq, range);
        }
    } else {
        // Non-pair range: "A2s-A9s" or "K2s-K9s"
        // The high card should be the same; we iterate the low card
        let suited_flag = if lb.len() >= 3 {
            match lb[2] as char {
                's' | 'S' => "s",
                'o' | 'O' => "o",
                _ => "",
            }
        } else { "" };

        let (high_l, low_l) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
        let (high_r, low_r) = if r1 > r2 { (r1, r2) } else { (r2, r1) };
        let high = high_l; // should be same for both
        if high_l != high_r { return; } // malformed

        let lo = low_l.min(low_r);
        let hi = low_l.max(low_r);
        for low_rank in lo..=hi {
            let combo = format!("{}{}{}", rank_to_char(high), rank_to_char(low_rank), suited_flag);
            add_single_combo(&combo, freq, range);
        }
    }
}

/// Add all combos for a single hand notation like "AA", "AKs", "AKo", "AK"
fn add_single_combo(combo_str: &str, freq: f64, range: &mut Range) {
    let bytes = combo_str.as_bytes();
    if bytes.len() < 2 { return; }

    let r1 = match char_to_rank(bytes[0] as char) { Some(r) => r, None => return };
    let r2 = match char_to_rank(bytes[1] as char) { Some(r) => r, None => return };

    let suited_flag: Option<bool> = if bytes.len() >= 3 {
        match bytes[2] as char {
            's' | 'S' => Some(true),
            'o' | 'O' => Some(false),
            _ => None,
        }
    } else {
        None
    };

    if r1 == r2 {
        // Pocket pair
        for s1 in 0..4u8 {
            for s2 in (s1+1)..4u8 {
                let h = canonical_hand([make_card(r1, s1), make_card(r1, s2)]);
                range.insert(h, freq);
            }
        }
    } else {
        let (high, low) = if r1 > r2 { (r1, r2) } else { (r2, r1) };
        match suited_flag {
            Some(true) => {
                for s in 0..4u8 {
                    let h = canonical_hand([make_card(high, s), make_card(low, s)]);
                    range.insert(h, freq);
                }
            }
            Some(false) => {
                for s1 in 0..4u8 {
                    for s2 in 0..4u8 {
                        if s1 == s2 { continue; }
                        let h = canonical_hand([make_card(high, s1), make_card(low, s2)]);
                        range.insert(h, freq);
                    }
                }
            }
            None => {
                // Both suited and offsuit
                for s1 in 0..4u8 {
                    for s2 in 0..4u8 {
                        let h = canonical_hand([make_card(high, s1), make_card(low, s2)]);
                        range.insert(h, freq);
                    }
                }
            }
        }
    }
}

fn rank_to_char(r: u8) -> char {
    match r {
        0 => '2', 1 => '3', 2 => '4', 3 => '5',
        4 => '6', 5 => '7', 6 => '8', 7 => '9',
        8 => 'T', 9 => 'J', 10 => 'Q', 11 => 'K', 12 => 'A',
        _ => '?',
    }
}

fn parse_specific_combo(s: &str) -> Option<[Card; 2]> {
    if s.len() != 4 { return None; }
    let c1 = parse_card_str(&s[0..2])?;
    let c2 = parse_card_str(&s[2..4])?;
    if c1 == c2 { return None; }
    Some(canonical_hand([c1, c2]))
}

fn parse_card_str(s: &str) -> Option<Card> {
    crate::cards::parse_card(s)
}

fn char_to_rank(c: char) -> Option<u8> {
    match c {
        '2' => Some(0), '3' => Some(1), '4' => Some(2), '5' => Some(3),
        '6' => Some(4), '7' => Some(5), '8' => Some(6), '9' => Some(7),
        'T' | 't' => Some(8), 'J' | 'j' => Some(9),
        'Q' | 'q' => Some(10), 'K' | 'k' => Some(11), 'A' | 'a' => Some(12),
        _ => None,
    }
}

/// Filter a range to remove hands conflicting with the board
pub fn filter_range_for_board(range: &Range, board: &[Card]) -> Range {
    use crate::cards::card_bit;
    let board_mask: u64 = board.iter().fold(0u64, |m, &c| m | card_bit(c));
    range.iter()
        .filter(|&(&hand, _)| {
            card_bit(hand[0]) & board_mask == 0 && card_bit(hand[1]) & board_mask == 0
        })
        .map(|(&h, &f)| (h, f))
        .collect()
}

/// Normalize range so sum of frequencies = 1.0
/// Used for strategy representation (convert weights to probabilities)
pub fn normalize(weights: &[f64]) -> Vec<f64> {
    let sum: f64 = weights.iter().sum();
    if sum <= 0.0 {
        let n = weights.len();
        return vec![1.0 / n as f64; n];
    }
    weights.iter().map(|&w| w / sum).collect()
}

/// Total combos in a range (weighted)
pub fn range_total(range: &Range) -> f64 {
    range.values().sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_aa() {
        let r = parse_range("AA");
        assert_eq!(r.len(), 6); // C(4,2)
        for (_, &freq) in &r {
            assert_eq!(freq, 1.0);
        }
    }

    #[test]
    fn test_parse_aks() {
        let r = parse_range("AKs");
        assert_eq!(r.len(), 4); // 4 suits
    }

    #[test]
    fn test_parse_ako() {
        let r = parse_range("AKo");
        assert_eq!(r.len(), 12); // 4*3
    }

    #[test]
    fn test_parse_ak() {
        let r = parse_range("AK");
        assert_eq!(r.len(), 16); // 4+12
    }

    #[test]
    fn test_parse_freq() {
        let r = parse_range("AA:0.5");
        for (_, &f) in &r {
            assert_eq!(f, 0.5);
        }
    }

    #[test]
    fn test_filter_board() {
        use crate::cards::parse_card;
        let r = parse_range("AA");
        let board = vec![parse_card("Ah").unwrap(), parse_card("Kd").unwrap()];
        let filtered = filter_range_for_board(&r, &board);
        // Ah is on board, so no hand containing Ah
        for (hand, _) in &filtered {
            assert_ne!(hand[0], parse_card("Ah").unwrap());
            assert_ne!(hand[1], parse_card("Ah").unwrap());
        }
    }

    #[test]
    fn test_parse_complex_range() {
        // Multiple components separated by commas
        let r = parse_range("AA,KK,QQ");
        assert_eq!(r.len(), 18); // 6+6+6
    }

    #[test]
    fn test_parse_specific_hand() {
        // Specific suited combo like AhKh
        let r = parse_range("AhKh");
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn test_parse_mixed_freq() {
        // Different frequencies for different parts
        let r = parse_range("AA:1.0,KK:0.5");
        let aa_count = r.iter().filter(|&(&h, _)| {
            crate::cards::rank(h[0]) == 12 && crate::cards::rank(h[1]) == 12
        }).count();
        let kk_freqs: Vec<f64> = r.iter().filter(|&(&h, _)| {
            crate::cards::rank(h[0]) == 11 && crate::cards::rank(h[1]) == 11
        }).map(|(_, &f)| f).collect();
        assert_eq!(aa_count, 6);
        assert_eq!(kk_freqs.len(), 6);
        for f in kk_freqs {
            assert_eq!(f, 0.5);
        }
    }

    #[test]
    fn test_empty_range() {
        let r = parse_range("");
        assert_eq!(r.len(), 0);
    }

    #[test]
    fn test_filter_removes_conflicting() {
        use crate::cards::parse_card;
        // KK with Kd on board should lose combos containing Kd
        let r = parse_range("KK");
        assert_eq!(r.len(), 6);
        let board = vec![
            parse_card("Kd").unwrap(),
            parse_card("7s").unwrap(),
            parse_card("2h").unwrap(),
        ];
        let filtered = filter_range_for_board(&r, &board);
        // 3 combos remain (KhKs, KhKc, KsKc)
        assert_eq!(filtered.len(), 3);
    }

    #[test]
    fn test_parse_pair_plus() {
        // "TT+" = TT,JJ,QQ,KK,AA = 5 pairs * 6 combos = 30
        let r = parse_range("TT+");
        assert_eq!(r.len(), 30);
    }

    #[test]
    fn test_parse_suited_plus() {
        // "ATs+" = ATs,AJs,AQs,AKs = 4 hands * 4 combos = 16
        let r = parse_range("ATs+");
        assert_eq!(r.len(), 16);
    }

    #[test]
    fn test_parse_offsuit_plus() {
        // "ATo+" = ATo,AJo,AQo,AKo = 4 hands * 12 combos = 48
        let r = parse_range("ATo+");
        assert_eq!(r.len(), 48);
    }

    #[test]
    fn test_parse_pair_dash_range() {
        // "22-55" = 22,33,44,55 = 4 * 6 = 24
        let r = parse_range("22-55");
        assert_eq!(r.len(), 24);
    }

    #[test]
    fn test_parse_suited_dash_range() {
        // "A2s-A5s" = A2s,A3s,A4s,A5s = 4 * 4 = 16
        let r = parse_range("A2s-A5s");
        assert_eq!(r.len(), 16);
    }

    #[test]
    fn test_parse_offsuit_dash_range() {
        // "K9o-KQo" = K9o,KTo,KJo,KQo = 4 * 12 = 48
        let r = parse_range("K9o-KQo");
        assert_eq!(r.len(), 48);
    }

    #[test]
    fn test_parse_realistic_range() {
        // Typical OOP range
        let r = parse_range("22+,A2s+,K9s+,Q9s+,J9s+,T9s,ATo+,KTo+,QTo+,JTo");
        // 22+ = 13*6 = 78 pairs
        // A2s+ = 12*4 = 48 suited
        // K9s+ = 4*4 = 16
        // Q9s+ = 3*4 = 12
        // J9s+ = 2*4 = 8
        // T9s = 1*4 = 4
        // ATo+ = 4*12 = 48
        // KTo+ = 3*12 = 36
        // QTo+ = 2*12 = 24
        // JTo = 1*12 = 12
        // Total unique = 78+48+16+12+8+4+48+36+24+12 = 286 (with possible overlap)
        assert!(r.len() > 200, "Expected > 200 combos, got {}", r.len());
    }

    #[test]
    fn test_range_total() {
        let r = parse_range("AA:0.5");
        let total = range_total(&r);
        assert!((total - 3.0).abs() < 0.001); // 6 combos * 0.5 = 3.0
    }
}
