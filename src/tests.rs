use frame_support::{assert_noop, assert_ok};
use sp_runtime::traits::Hash;

use crate::game::Hand;
use crate::mock::*;

use super::*;

#[test]
fn can_create_game() {
    new_test_ext().execute_with(|| {
        let round_length = 20;
        let bet = 10;
        let move_hash = <Test as Config>::MoveHasher::hash(b"move");
        let balance = Balances::free_balance(1);

        assert_ok!(RoPaSci::start(Origin::signed(1), round_length, bet, move_hash));
        assert_eq!(balance - bet, Balances::free_balance(1));

        assert!(matches!(
            Games::<Test>::get(move_hash),
            Some(Game {
                round_length: game_round_length,
                bet: game_bet,
                stage: GameStage::Betting { participating_players: 1 },
                ..
            }) if round_length == game_round_length && bet == game_bet
        ));

        assert!(matches!(
            BettingGamesIndex::<Test>::get(20),
            Some(games) if games.len() == 1 && games.contains(&move_hash)
        ));

        assert!(!RevealingGamesIndex::<Test>::contains_key(20));

        assert!(matches!(
            Moves::<Test>::get(move_hash, 1),
            Some(Move{ hash, hand: None }) if hash == move_hash
        ));
    });
}

#[test]
fn fail_duplicate_game() {
    new_test_ext().execute_with(|| {
        let round_length = 20;
        let bet = 10;
        let move_hash = <Test as Config>::MoveHasher::hash(b"move");

        assert_ok!(RoPaSci::start(Origin::signed(1), round_length, bet, move_hash));
        assert_noop!(
            RoPaSci::start(Origin::signed(1), round_length, bet, move_hash),
            Error::<Test>::GameExists,
        );
    });
}

#[test]
fn fail_wrong_round_length() {
    new_test_ext().execute_with(|| {
        let round_length = 0;
        let bet = 10;
        let move_hash = <Test as Config>::MoveHasher::hash(b"move");

        assert_noop!(
            RoPaSci::start(Origin::signed(1), round_length, bet, move_hash),
            Error::<Test>::RoundLengthInvalid,
        );
    });
}

#[test]
fn fail_not_enough_money() {
    new_test_ext().execute_with(|| {
        let round_length = 20;
        let bet = Balances::free_balance(1) + 1;
        let move_hash = <Test as Config>::MoveHasher::hash(b"move");

        assert_noop!(
            RoPaSci::start(Origin::signed(1), round_length, bet, move_hash),
            Error::<Test>::MoneyNotEnough,
        );
    });
}

#[test]
fn can_join_game() {
    new_test_ext().execute_with(|| {
        let round_length = 20;
        let bet = 10;
        let move_hash = <Test as Config>::MoveHasher::hash(b"move");
        let balance = Balances::free_balance(2);

        assert_ok!(RoPaSci::start(Origin::signed(1), round_length, bet, move_hash));
        assert_ok!(RoPaSci::join(Origin::signed(2),  move_hash, move_hash));
        assert_ok!(RoPaSci::join(Origin::signed(3),  move_hash, move_hash));

        assert_eq!(balance - bet, Balances::free_balance(2));
        assert_eq!(3, Moves::<Test>::iter_prefix(move_hash).count());
        assert!(Moves::<Test>::contains_key(move_hash, 2));
        assert!(Moves::<Test>::contains_key(move_hash, 3));
        assert!(matches!(
        Games::<Test>::get(move_hash),
            Some(Game {
                stage: GameStage::Betting { participating_players: 3 },
                ..
            })
        ));
    });
}

#[test]
fn fail_join_twice() {
    new_test_ext().execute_with(|| {
        let round_length = 20;
        let bet = 10;
        let move_hash = <Test as Config>::MoveHasher::hash(b"move");

        assert_ok!(RoPaSci::start(Origin::signed(1), round_length, bet, move_hash));
        assert_ok!(RoPaSci::join(Origin::signed(2),  move_hash, move_hash));
        assert_noop!(
            RoPaSci::join(Origin::signed(2),  move_hash, move_hash),
            Error::<Test>::PlayerMoveMade,
        );
    });
}

#[test]
fn moves_to_reveal() {
    new_test_ext().execute_with(|| {
        let round_length = 20;
        let bet = 10;
        let move_hash = <Test as Config>::MoveHasher::hash(b"move");

        assert_ok!(RoPaSci::start(Origin::signed(1), round_length, bet, move_hash));
        assert_ok!(RoPaSci::join(Origin::signed(2),  move_hash, move_hash));

        // end betting
        run_to_block(20);

        assert!(matches!(
            Games::<Test>::get(move_hash),
            Some(Game {
                stage: GameStage::Revealing { anticipated_players : 2 },
                ..
            })
        ));

        assert!(!BettingGamesIndex::<Test>::contains_key(20));

        assert!(matches!(
            RevealingGamesIndex::<Test>::get(40),
            Some(games) if games.len() == 1 && games.contains(&move_hash)
        ));
    });
}

#[test]
fn fail_join_at_revealing() {
    new_test_ext().execute_with(|| {
        let round_length = 20;
        let bet = 10;
        let move_hash = <Test as Config>::MoveHasher::hash(b"move");

        assert_ok!(RoPaSci::start(Origin::signed(1), round_length, bet, move_hash));
        assert_ok!(RoPaSci::join(Origin::signed(2),  move_hash, move_hash));

        // end betting
        run_to_block(20);

        assert_noop!(
            RoPaSci::join(Origin::signed(3),  move_hash, move_hash),
            Error::<Test>::GameWrongStage,
        );
    });
}

#[test]
fn can_reveal() {
    new_test_ext().execute_with(|| {
        let round_length = 20;
        let bet = 10;
        let move_1 = vec![Hand::Rock as u8, 1, 2, 3];
        let move_1_hash = <Test as Config>::MoveHasher::hash(&move_1);
        let move_2 = vec![Hand::Paper as u8, 1, 2, 3, 4];
        let move_2_hash = <Test as Config>::MoveHasher::hash(&move_2);

        assert_ok!(RoPaSci::start(Origin::signed(1), round_length, bet, move_1_hash));
        assert_ok!(RoPaSci::join(Origin::signed(2),  move_1_hash, move_2_hash));

        // end betting
        run_to_block(20);

        assert_ok!(RoPaSci::reveal(Origin::signed(2),  move_1_hash, move_2));

        assert!(matches!(
            Games::<Test>::get(move_1_hash),
            Some(Game {
                stage: GameStage::Revealing { anticipated_players : 1 },
                ..
            })
        ));

        assert!(matches!(
            Moves::<Test>::get(move_1_hash, 2),
            Some(Move {
                hash,
                hand: Some(Hand::Paper),
            }) if hash == move_2_hash
        ))
    });
}

#[test]
fn fail_reveal() {
    new_test_ext().execute_with(|| {
        let round_length = 20;
        let bet = 10;
        let move_1 = vec![100, 1, 2, 3]; // invalid hand
        let move_1_hash = <Test as Config>::MoveHasher::hash(&move_1);
        let move_2 = vec![Hand::Paper as u8, 1, 2, 3, 4];
        let move_2_hash = <Test as Config>::MoveHasher::hash(&move_2);

        assert_ok!(RoPaSci::start(Origin::signed(1), round_length, bet, move_1_hash));
        assert_ok!(RoPaSci::join(Origin::signed(2),  move_1_hash, move_2_hash));

        // end betting
        run_to_block(20);

        assert_noop!(
            RoPaSci::reveal(Origin::signed(1),  move_1_hash, move_1),
            Error::<Test>::PlayerMoveInvalid,
        );

        assert_noop!(
            RoPaSci::reveal(Origin::signed(2),  move_1_hash, vec![1, 2, 3]),
            Error::<Test>::PlayerRevealMismatch,
        );

        assert_noop!(
            RoPaSci::reveal(Origin::signed(3),  move_1_hash, vec![1, 2, 3]),
            Error::<Test>::PlayerMoveMissing,
        );
    });
}

#[test]
fn can_end_game() {
    new_test_ext().execute_with(|| {
        let round_length = 20;
        let bet = 10;
        let move_hash = <Test as Config>::MoveHasher::hash(b"move");

        assert_ok!(RoPaSci::start(Origin::signed(1), round_length, bet, move_hash));
        run_to_block(40);

        assert!(!Games::<Test>::contains_key(move_hash));
        assert_eq!(0, Moves::<Test>::iter_prefix(move_hash).count());
        assert!(!RevealingGamesIndex::<Test>::contains_key(40));
    });
}

#[test]
fn can_end_game_none_revealed() {
    new_test_ext().execute_with(|| {
        let round_length = 20;
        let bet = 10;
        let move_hash = <Test as Config>::MoveHasher::hash(b"move");
        let (balance_1, balance_2, balance_3) =
            (Balances::free_balance(1), Balances::free_balance(2), Balances::free_balance(3));

        assert_ok!(RoPaSci::start(Origin::signed(1), round_length, bet, move_hash));
        assert_ok!(RoPaSci::join(Origin::signed(2),  move_hash, move_hash));
        assert_ok!(RoPaSci::join(Origin::signed(3),  move_hash, move_hash));

        assert!(matches!(
            (Balances::free_balance(1), Balances::free_balance(2), Balances::free_balance(3)),
            (b1, b2, b3) if b1 == balance_1 - bet && b2 == balance_2 - bet && b3 == balance_3 - bet
        ));

        // end game
        run_to_block(40);

        // no one revealed, so no one wins. all bets are returned
        assert!(matches!(
            (Balances::free_balance(1), Balances::free_balance(2), Balances::free_balance(3)),
            (b1, b2, b3) if b1 == balance_1 && b2 == balance_2 && b3 == balance_3
        ));
    });
}

#[test]
fn can_end_game_draw() {
    new_test_ext().execute_with(|| {
        let round_length = 20;
        let bet = 10;
        let same_move = vec![Hand::Rock as u8, 1, 2, 3];
        let move_hash = <Test as Config>::MoveHasher::hash(&same_move);
        let (balance_1, balance_2, balance_3) =
            (Balances::free_balance(1), Balances::free_balance(2), Balances::free_balance(3));

        assert_ok!(RoPaSci::start(Origin::signed(1), round_length, bet, move_hash));
        assert_ok!(RoPaSci::join(Origin::signed(2),  move_hash, move_hash));
        assert_ok!(RoPaSci::join(Origin::signed(3),  move_hash, move_hash));

        // end betting
        run_to_block(20);

        assert_ok!(RoPaSci::reveal(Origin::signed(1), move_hash, same_move.clone()));
        assert_ok!(RoPaSci::reveal(Origin::signed(2),  move_hash, same_move.clone()));
        assert_ok!(RoPaSci::reveal(Origin::signed(3),  move_hash, same_move));

        // game ends with last move revealed. all bets are returned
        assert!(matches!(
            (Balances::free_balance(1), Balances::free_balance(2), Balances::free_balance(3)),
            (b1, b2, b3) if b1 == balance_1 && b2 == balance_2 && b3 == balance_3
        ));
    });
}

#[test]
fn can_end_game_no_win() {
    new_test_ext().execute_with(|| {
        let round_length = 20;
        let bet = 10;
        let (move_1, move_2, move_3) =
            (vec![Hand::Rock as u8], vec![Hand::Paper as u8], vec![Hand::Scissors as u8]);
        let move_1_hash = <Test as Config>::MoveHasher::hash(&move_1);
        let move_2_hash = <Test as Config>::MoveHasher::hash(&move_2);
        let move_3_hash = <Test as Config>::MoveHasher::hash(&move_3);
        let (balance_1, balance_2, balance_3) =
            (Balances::free_balance(1), Balances::free_balance(2), Balances::free_balance(3));

        assert_ok!(RoPaSci::start(Origin::signed(1), round_length, bet, move_1_hash));
        assert_ok!(RoPaSci::join(Origin::signed(2),  move_1_hash, move_2_hash));
        assert_ok!(RoPaSci::join(Origin::signed(3),  move_1_hash, move_3_hash));

        // end betting
        run_to_block(20);

        assert_ok!(RoPaSci::reveal(Origin::signed(1), move_1_hash, move_1));
        assert_ok!(RoPaSci::reveal(Origin::signed(2),  move_1_hash, move_2));
        assert_ok!(RoPaSci::reveal(Origin::signed(3),  move_1_hash, move_3));

        // game ends with last move revealed and no one wins. all bets are returned
        assert!(matches!(
            (Balances::free_balance(1), Balances::free_balance(2), Balances::free_balance(3)),
            (b1, b2, b3) if b1 == balance_1 && b2 == balance_2 && b3 == balance_3
        ));
    });
}

#[test]
fn can_end_game_win() {
    new_test_ext().execute_with(|| {
        let round_length = 20;
        let bet = 10;
        let (move_1, move_2, move_3) =
            (vec![Hand::Rock as u8], vec![Hand::Paper as u8], vec![Hand::Paper as u8]);
        let move_1_hash = <Test as Config>::MoveHasher::hash(&move_1);
        let move_2_hash = <Test as Config>::MoveHasher::hash(&move_2);
        let move_3_hash = <Test as Config>::MoveHasher::hash(&move_3);
        let move_4_hash = <Test as Config>::MoveHasher::hash(b"misses reveal");
        let (balance_1, balance_2, balance_3, balance_4) = (
            Balances::free_balance(1),
            Balances::free_balance(2),
            Balances::free_balance(3),
            Balances::free_balance(4)
        );

        assert_ok!(RoPaSci::start(Origin::signed(1), round_length, bet, move_1_hash));
        assert_ok!(RoPaSci::join(Origin::signed(2),  move_1_hash, move_2_hash));
        assert_ok!(RoPaSci::join(Origin::signed(3),  move_1_hash, move_3_hash));
        assert_ok!(RoPaSci::join(Origin::signed(4),  move_1_hash, move_4_hash));

        // end betting
        run_to_block(20);

        assert_ok!(RoPaSci::reveal(Origin::signed(1), move_1_hash, move_1));
        assert_ok!(RoPaSci::reveal(Origin::signed(2),  move_1_hash, move_2));
        assert_ok!(RoPaSci::reveal(Origin::signed(3),  move_1_hash, move_3));

        // end game
        run_to_block(40);

        println!("{:?}", Balances::free_balance(1));
        println!("{:?}", Balances::free_balance(2));
        println!("{:?}", Balances::free_balance(3));
        println!("{:?}", Balances::free_balance(4));

        // players 2 and 3 win. player 1 loses. player 4 misses reveal and loses
        assert!(matches!(
            (Balances::free_balance(1), Balances::free_balance(2), Balances::free_balance(3), Balances::free_balance(4)),
            (b1, b2, b3, b4) if b1 == balance_1 - bet && b2 == balance_2 + bet && b3 == balance_3 + bet && b4 == balance_4 - bet
        ));
    });
}