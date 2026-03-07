/// Card representation: 0..51
/// rank = card / 4, suit = card % 4
/// rank: 0=2, 1=3, ..., 12=A
/// suit: 0=s, 1=h, 2=d, 3=c
pub type Card = u8;

pub const DECK_SIZE: usize = 52;

pub const RANK_2: u8 = 0;
pub const RANK_A: u8 = 12;

pub fn make_card(rank: u8, suit: u8) -> Card {
    rank * 4 + suit
}

pub fn rank(card: Card) -> u8 {
    card / 4
}

pub fn suit(card: Card) -> u8 {
    card % 4
}

/// Parse card string like "Ah", "Ks", "2c", "Td"
pub fn parse_card(s: &str) -> Option<Card> {
    let mut chars = s.chars();
    let rank_ch = chars.next()?;
    let suit_ch = chars.next()?;
    let rank = match rank_ch {
        '2' => 0, '3' => 1, '4' => 2, '5' => 3, '6' => 4,
        '7' => 5, '8' => 6, '9' => 7, 'T' | 't' => 8,
        'J' | 'j' => 9, 'Q' | 'q' => 10, 'K' | 'k' => 11,
        'A' | 'a' => 12,
        _ => return None,
    };
    let suit = match suit_ch {
        's' | 'S' => 0,
        'h' | 'H' => 1,
        'd' | 'D' => 2,
        'c' | 'C' => 3,
        _ => return None,
    };
    Some(make_card(rank, suit))
}

pub fn card_to_string(card: Card) -> String {
    let rank_ch = match rank(card) {
        0 => '2', 1 => '3', 2 => '4', 3 => '5', 4 => '6',
        5 => '7', 6 => '8', 7 => '9', 8 => 'T',
        9 => 'J', 10 => 'Q', 11 => 'K', 12 => 'A',
        _ => '?',
    };
    let suit_ch = match suit(card) {
        0 => 's', 1 => 'h', 2 => 'd', 3 => 'c',
        _ => '?',
    };
    format!("{}{}", rank_ch, suit_ch)
}

/// Parse board string like "Qs8h4d" or ["Qs","8h","4d"]
pub fn parse_board_str(s: &str) -> Option<Vec<Card>> {
    if s.len() % 2 != 0 {
        return None;
    }
    let mut cards = Vec::new();
    let mut i = 0;
    let bytes = s.as_bytes();
    while i + 1 < s.len() {
        let chunk = &s[i..i + 2];
        cards.push(parse_card(chunk)?);
        i += 2;
    }
    Some(cards)
}

/// 52-card deck as bitmask (u64)
pub type CardSet = u64;

pub fn card_bit(card: Card) -> CardSet {
    1u64 << card
}

pub fn has_card(set: CardSet, card: Card) -> bool {
    set & card_bit(card) != 0
}

/// All hole-hand combinations (C(52,2) = 1326)
pub fn all_hands() -> Vec<[Card; 2]> {
    let mut hands = Vec::with_capacity(1326);
    for c1 in 0..52u8 {
        for c2 in (c1 + 1)..52u8 {
            hands.push([c1, c2]);
        }
    }
    hands
}

/// Canonical hand (lower card first)
pub fn canonical_hand(h: [Card; 2]) -> [Card; 2] {
    if h[0] <= h[1] { h } else { [h[1], h[0]] }
}

/// Parse hand string like "AhKs"
pub fn parse_hand(s: &str) -> Option<[Card; 2]> {
    if s.len() != 4 { return None; }
    let c1 = parse_card(&s[0..2])?;
    let c2 = parse_card(&s[2..4])?;
    Some(canonical_hand([c1, c2]))
}

pub fn hand_to_string(hand: [Card; 2]) -> String {
    format!("{}{}", card_to_string(hand[0]), card_to_string(hand[1]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_card() {
        assert_eq!(parse_card("As"), Some(make_card(12, 0)));
        assert_eq!(parse_card("2c"), Some(make_card(0, 3)));
        assert_eq!(parse_card("Td"), Some(make_card(8, 2)));
    }

    #[test]
    fn test_card_roundtrip() {
        for c in 0..52u8 {
            let s = card_to_string(c);
            assert_eq!(parse_card(&s), Some(c));
        }
    }

    #[test]
    fn test_all_hands() {
        let hands = all_hands();
        assert_eq!(hands.len(), 1326);
    }
}
