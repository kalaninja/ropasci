use sp_runtime::traits::{Get, Hash};

use crate::*;

impl<T: Config> Pallet<T> {
    pub(crate) fn valid_round_length(round_length: T::BlockNumber) -> bool {
        round_length >= T::MinRoundLength::get().into() &&
            round_length <= T::MaxRoundLength::get().into()
    }

    pub(crate) fn can_create_game(game_id: &GameId<T>) -> bool {
        !Games::<T>::contains_key(game_id)
    }

    pub(crate) fn can_join_game(game: &GameOf<T>) -> bool {
        matches!(game.stage, GameStage::Betting{..})
    }

    pub(crate) fn can_make_move(game_id: &GameId<T>, player: &T::AccountId) -> bool {
        !Moves::<T>::contains_key(game_id, player)
    }

    pub(crate) fn can_reveal_move(game: &GameOf<T>) -> bool {
        matches!(game.stage, GameStage::Revealing{..})
    }

    pub(crate) fn reveal_match(move_reveal: &Vec<u8>, move_hash: &T::MoveHash) -> bool {
        *move_hash == T::MoveHasher::hash(move_reveal)
    }
}