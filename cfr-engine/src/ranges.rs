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

    // Specific combo like "AhKh"
    if combo_str.len() == 4 {
        if let Some(hand) = parse_specific_combo(combo_str) {
            range.insert(hand, freq);
            return;
        }
    }

    let bytes = combo_str.as_bytes();
    if bytes.len() < 2 { return; }

    let r1 = char_to_rank(bytes[0] as char);
    let r2 = char_to_rank(bytes[1] as char);
    if r1.is_none() || r2.is_none() { return; }
    let r1 = r1.unwrap();
    let r2 = r2.unwrap();

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
                        if s1 == s2 {
                            let h = canonical_hand([make_card(high, s1), make_card(low, s2)]);
                            range.insert(h, freq);
                        } else {
                            let h = canonical_hand([make_card(high, s1), make_card(low, s2)]);
                            range.insert(h, freq);
                        }
                    }
                }
            }
        }
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
}
