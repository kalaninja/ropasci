use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite};
use frame_support::assert_ok;
use frame_system::RawOrigin;
use sp_runtime::traits::Bounded;

use crate::game::Hand;
use crate::Pallet as RoPaSci;

use super::*;

const SEED: u32 = 0;

fn get_player<T: Config>(index: u32) -> T::AccountId {
    let player = account("player", index, SEED);
    T::Currency::make_free_balance_be(&player, BalanceOf::<T>::max_value());
    player
}

fn start_new_game<T: Config>(seed: &[u8]) -> GameId<T> {
    let owner = get_player::<T>(1);
    let round_length = 20u32.into();
    let bet = 10u32.into();
    let move_hash = T::MoveHasher::hash(seed);

    assert_ok!(RoPaSci::<T>::start(RawOrigin::Signed(owner).into(), round_length, bet, move_hash));
    move_hash
}

benchmarks! {
    start_game {
        let caller = get_player::<T>(1);
		let round_length = 20u32.into();
        let bet = 10u32.into();
        let move_hash = T::MoveHasher::hash(b"move");
    }: start(RawOrigin::Signed(caller), round_length, bet, move_hash)
    verify {
        assert!(Games::<T>::contains_key(move_hash));
    }

    join_game {
        let game_id = start_new_game::<T>(b"game");
        let caller = get_player::<T>(2);
        let move_hash = T::MoveHasher::hash(b"move");
    }: join(RawOrigin::Signed(caller.clone()), game_id, move_hash)
    verify {
        assert!(Moves::<T>::contains_key(game_id, caller));
    }

    reveal_move {
        let r in 1 .. 10_000_000;

        let game_id = start_new_game::<T>(b"game");
        let caller = get_player::<T>(2);
        let move_reveal = vec![0u8; r as usize];
        let move_hash = <T as Config>::MoveHasher::hash(&move_reveal);
        assert_ok!(RoPaSci::<T>::join(RawOrigin::Signed(caller.clone()).into(), game_id, move_hash));

        // end betting
        frame_system::Pallet::<T>::set_block_number(20u32.into());
        RoPaSci::<T>::on_initialize(20u32.into());
    }: reveal(RawOrigin::Signed(caller.clone()), game_id, move_reveal)
    verify {
        assert!(matches!(
            Moves::<T>::get(game_id, caller),
            Some(Move {
                hash,
                hand: Some(Hand::Rock),
            }) if hash == move_hash
        ))
    }

    on_initialize_betting {
        let g in 1 .. 10_000;

        for i in 1..=g {
            let seed = vec![1u8; i as usize];
            let _game_id = start_new_game::<T>(&seed);
        }

        let block_number = 20u32.into();
        frame_system::Pallet::<T>::set_block_number(block_number);
    }: {
        RoPaSci::<T>::on_initialize(block_number);
    }
    verify {
        assert!(!BettingGamesIndex::<T>::contains_key(block_number));
    }

    on_initialize_revealing {
        let g in 1 .. 10_000;

        for i in 1..=g {
            let seed = vec![1u8; i as usize];
            let _game_id = start_new_game::<T>(&seed);
        }

        // end betting
        frame_system::Pallet::<T>::set_block_number(20u32.into());
        RoPaSci::<T>::on_initialize(20u32.into());

        let block_number = 40u32.into();
        frame_system::Pallet::<T>::set_block_number(block_number);
    }: {
        RoPaSci::<T>::on_initialize(block_number);
    }
    verify {
        assert!(!RevealingGamesIndex::<T>::contains_key(block_number));
    }
}

impl_benchmark_test_suite!(RoPaSci, crate::mock::new_test_ext(), crate::mock::Test);
