use internal_iterator::InternalIterator;

use board_game::games::scrabble::basic::Deck;

#[test]
fn empty_sub_deck() {
    assert_eq!(vec![Deck::default()], Deck::default().sub_decks().collect::<Vec<_>>());
}

#[test]
fn simple_sub_deck() {
    assert_eq!(
        vec![
            Deck::from_letters("").unwrap(),
            Deck::from_letters("X").unwrap(),
            Deck::from_letters("D").unwrap(),
            Deck::from_letters("DX").unwrap(),
            Deck::from_letters("A").unwrap(),
            Deck::from_letters("AX").unwrap(),
            Deck::from_letters("AD").unwrap(),
            Deck::from_letters("ADX").unwrap(),
        ],
        Deck::from_letters("ADX").unwrap().sub_decks().collect::<Vec<_>>()
    );
}

#[test]
fn multi_sub_deck() {
    assert_eq!(
        vec![
            Deck::from_letters("").unwrap(),
            Deck::from_letters("D").unwrap(),
            Deck::from_letters("A").unwrap(),
            Deck::from_letters("AD").unwrap(),
            Deck::from_letters("AA").unwrap(),
            Deck::from_letters("AAD").unwrap(),
        ],
        Deck::from_letters("AAD").unwrap().sub_decks().collect::<Vec<_>>()
    );
}
