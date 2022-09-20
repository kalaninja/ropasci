//! # RoPaSci Pallet
//! A pallet allowing to play Rock-Paper-Scissors game.
//!
//! A game can be created by a `start` call with a bet amount, a round length and a move hash. The
//! move hash of the game creator becomes the game id. The game is created in the "betting" stage
//! and will remain in this stage until the end of the round.
//!
//! While in the "betting" stage the game can be joined by other players with a `join` call. After
//! the "betting" stage the game is moved to the "revealing" stage and will remain in this stage
//! until the end.
//!
//! In the "revealing" stage all the game participants can reveal their moves with a `reveal` call
//! providing the actual move of the player with the salt used to hash the move. The first byte of
//! the move reveal is the move itself and the rest of the bytes are the salt. The actual move
//! should be one of the following:
//! - 0x00: Rock
//! - 0x01: Paper
//! - 0x02: Scissors
//!
//! The game ends when the last player reveals their move or when the round length is reached.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    pallet_prelude::*,
    traits::{Currency, ExistenceRequirement, WithdrawReasons},
};
use frame_system::pallet_prelude::*;
use sp_runtime::{
    SaturatedConversion,
    traits::{CheckedDiv, CheckEqual, Hash, MaybeDisplay, MaybeMallocSizeOf, Saturating, SimpleBitOps},
};
use sp_std::{
    fmt::Debug,
    prelude::*,
};

pub use pallet::*;

use crate::game::{Game, GameStage, Move};

mod validation;
mod game;
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
type GameId<T> = <T as Config>::MoveHash;
type GameOf<T> = Game<<T as frame_system::Config>::BlockNumber, BalanceOf<T>>;
type MoveOf<T> = Move<<T as Config>::MoveHash>;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The output of the `MoveHasher` function.
        type MoveHash: Parameter
        + Member
        + MaybeSerializeDeserialize
        + Debug
        + MaybeDisplay
        + SimpleBitOps
        + Ord
        + Default
        + Copy
        + CheckEqual
        + sp_std::hash::Hash
        + AsRef<[u8]>
        + AsMut<[u8]>
        + MaybeMallocSizeOf
        + MaxEncodedLen;

        /// The type of hash used for hashing moves.
        type MoveHasher: Hash<Output=Self::MoveHash>;

        /// The currency trait.
        type Currency: Currency<Self::AccountId>;

        /// Minimal round length.
        #[pallet::constant]
        type MinRoundLength: Get<u32>;

        /// Maximum round length.
        #[pallet::constant]
        type MaxRoundLength: Get<u32>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Game started. \[game_id, owner, round_length, bet\]
        GameStarted { game_id: GameId<T>, owner: T::AccountId, round_length: T::BlockNumber, bet: BalanceOf<T> },
        /// Bet placed. \[game_id, player\]
        BetPlaced { game_id: GameId<T>, player: T::AccountId },
        /// Move revealed. \[game_id, player\]
        MoveRevealed { game_id: GameId<T>, player: T::AccountId },
        /// Game ended. \[game_id, winners, reward\]
        GameEnded { game_id: GameId<T>, winners: Vec<T::AccountId>, reward: BalanceOf<T> },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Round length is invalid.
        RoundLengthInvalid,
        /// Money is not enough
        MoneyNotEnough,
        /// Game already exists
        GameExists,
        /// Game does not exist
        GameMissing,
        /// Game stage is wrong
        GameWrongStage,
        /// Player already made a move
        PlayerMoveMade,
        /// Player did not make a move
        PlayerMoveMissing,
        /// Player move is invalid
        PlayerMoveInvalid,
        /// Player move reveal does not match with the move hash
        PlayerRevealMismatch,
    }

    /// The games currently in prob"nonplayer move"gress.
    #[pallet::storage]
    pub type Games<T> = StorageMap<_, Blake2_128Concat, GameId<T>, GameOf<T>, OptionQuery>;

    /// Index of all the active games in "betting" stage by their expiration block number.
    #[pallet::storage]
    pub type BettingGamesIndex<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::BlockNumber,
        Vec<GameId<T>>,
        OptionQuery
    >;

    /// Index of all the active games in "revealing" stage by their expiration block number.
    #[pallet::storage]
    pub type RevealingGamesIndex<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::BlockNumber,
        Vec<GameId<T>>,
        OptionQuery
    >;

    /// The moves made by the players in all the active games.
    #[pallet::storage]
    pub type Moves<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        GameId<T>,
        Twox64Concat,
        T::AccountId,
        MoveOf<T>,
        OptionQuery
    >;

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(now: T::BlockNumber) -> Weight {
            BettingGamesIndex::<T>::mutate_exists(now, |maybe_game_ids| {
                if let Some(game_ids) = maybe_game_ids.take() {
                    for game_id in game_ids {
                        Self::end_betting(&game_id);
                    }
                }
            });

            RevealingGamesIndex::<T>::mutate_exists(now, |maybe_game_ids| {
                if let Some(game_ids) = maybe_game_ids.take() {
                    for game_id in game_ids {
                        Self::end_game(&game_id);
                    }
                }
            });

            T::DbWeight::get().reads(2)
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Start a new game. The game will be created in "betting" stage. A creator needs to
        /// provide a round length, a bet amount and a move hash. The move hash becomes an game id.
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn start(
            origin: OriginFor<T>,
            #[pallet::compact] round_length: T::BlockNumber,
            #[pallet::compact] bet: BalanceOf<T>,
            move_hash: T::MoveHash,
        ) -> DispatchResult {
            let owner = ensure_signed(origin)?;
            ensure!(Self::valid_round_length(round_length), Error::<T>::RoundLengthInvalid);
            ensure!(Self::can_create_game(&move_hash), Error::<T>::GameExists);

            Self::deposit_bet(&owner, bet)?;
            Self::start_game(&owner, move_hash, round_length, bet);

            Self::deposit_event(Event::<T>::GameStarted { game_id: move_hash, owner, round_length, bet });
            Ok(())
        }

        /// Place a bet on an existing game. The game must be in "betting" stage. A player needs to
        /// provide a game id and a move hash.
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn join(
            origin: OriginFor<T>,
            game_id: GameId<T>,
            move_hash: T::MoveHash,
        ) -> DispatchResult {
            let joiner = ensure_signed(origin)?;
            let game = Games::<T>::get(&game_id).ok_or(Error::<T>::GameMissing)?;
            ensure!(Self::can_join_game(&game), Error::<T>::GameWrongStage);
            ensure!(Self::can_make_move(&game_id, &joiner), Error::<T>::PlayerMoveMade);

            Self::deposit_bet(&joiner, game.bet)?;
            Self::join_game(&game_id, &joiner, move_hash);

            Self::deposit_event(Event::<T>::BetPlaced { game_id, player: joiner });
            Ok(())
        }

        /// Reveal a move. The game must be in "revealing" stage. A player needs to provide a game id
        /// and a move reveal. The move reveal will be hashed and compared with the move hash.
        /// The first byte of the reveal is the move itself. The rest of the reveal is the salt.
        /// The actual move should be one of the following: 0 - Rock, 1 - Paper, 2 - Scissors.
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn reveal(
            origin: OriginFor<T>,
            game_id: GameId<T>,
            move_reveal: Vec<u8>,
        ) -> DispatchResult {
            let player = ensure_signed(origin)?;
            let game = Games::<T>::get(&game_id).ok_or(Error::<T>::GameMissing)?;
            ensure!(Self::can_reveal_move(&game), Error::<T>::GameWrongStage);

            let is_last = Self::try_reveal_move(&game_id, &player, &move_reveal)?;
            Self::deposit_event(Event::<T>::MoveRevealed { game_id, player });

            if is_last {
                Self::end_game(&game_id);
            }
            Ok(())
        }
    }
}

impl<T: Config> Pallet<T> {
    fn now() -> T::BlockNumber {
        frame_system::Pallet::<T>::block_number()
    }

    fn deposit_bet(player: &T::AccountId, bet: BalanceOf<T>) -> Result<(), Error<T>> {
        T::Currency::withdraw(
            player,
            bet,
            WithdrawReasons::RESERVE,
            ExistenceRequirement::KeepAlive)
            .map(|_| ())
            .map_err(|_| Error::<T>::MoneyNotEnough)
    }

    fn start_game(owner: &T::AccountId, move_hash: T::MoveHash, round_length: T::BlockNumber, bet: BalanceOf<T>) {
        let now = Self::now();
        let game = Game::start(
            now,
            round_length,
            bet,
        );

        Games::<T>::insert(move_hash, &game);
        BettingGamesIndex::<T>::append(now.saturating_add(game.round_length), move_hash);
        Moves::<T>::insert(move_hash, owner, Move::new(move_hash));
    }

    fn join_game(game_id: &GameId<T>, joiner: &T::AccountId, move_hash: T::MoveHash) {
        Games::<T>::mutate(game_id, |maybe_game| {
            maybe_game.as_mut().map(|game| game.join())
        });
        Moves::<T>::insert(&game_id, joiner, Move::new(move_hash));
    }

    fn end_betting(game_id: &GameId<T>) {
        Games::<T>::mutate(game_id, |maybe_game| {
            maybe_game.as_mut().map(|game| {
                game.start_revealing();
                let timeout = Self::now().saturating_add(game.round_length);
                RevealingGamesIndex::<T>::append(timeout, game_id);
            })
        });
    }

    fn try_reveal_move(
        game_id: &GameId<T>,
        player: &T::AccountId,
        move_reveal: &Vec<u8>,
    ) -> Result<bool, Error<T>> {
        Moves::<T>::try_mutate(
            game_id,
            player,
            |maybe_move| -> Result<(), Error<T>>{
                let player_move = maybe_move.as_mut().ok_or(Error::<T>::PlayerMoveMissing)?;
                ensure!(Self::reveal_match(move_reveal, &player_move.hash), Error::<T>::PlayerRevealMismatch);

                player_move.reveal(move_reveal).map_err(|_| Error::<T>::PlayerMoveInvalid)?;
                Ok(())
            })?;

        let mut is_last = false;
        Games::<T>::mutate(game_id, |maybe_game| {
            maybe_game.as_mut().map(|game| {
                if game.last_revealing() {
                    is_last = true
                } else {
                    game.reveal()
                }
            })
        });

        Ok(is_last)
    }

    fn end_game(game_id: &GameId<T>) {
        Games::<T>::mutate_exists(game_id, |maybe_game| {
            if let Some(game) = maybe_game.take() {
                let moves = Moves::<T>::drain_prefix(game_id).collect::<Vec<_>>();
                let players_count = BalanceOf::<T>::saturated_from(moves.len());
                let money_pool = game.bet.saturating_mul(players_count);
                let mut winners = Self::find_winners(&moves);
                let winners_count = BalanceOf::<T>::saturated_from(winners.len());
                let reward = money_pool.checked_div(&winners_count)
                    .unwrap_or_else(|| {
                        // return all bets if something went wrong
                        winners = moves.into_iter().map(|(player, _)| player).collect();
                        game.bet
                    });

                for winner in &winners {
                    T::Currency::deposit_creating(winner, reward);
                }

                Self::deposit_event(Event::<T>::GameEnded { game_id: *game_id, winners, reward });
            }
        });
    }

    fn find_winners(moves: &[(T::AccountId, MoveOf<T>)]) -> Vec<T::AccountId> {
        let mut winners = Vec::new();

        let mut hands_lookup = [false; 3];
        for (_, player_move) in moves {
            if let Some(hand) = player_move.hand {
                hands_lookup[hand as usize] = true;
            }
        }

        for (player, player_move) in moves {
            if player_move.hand
                .map(|hand| hand.beaten_by())
                .map(|beaten_by| !hands_lookup[beaten_by as usize])
                .unwrap_or(false) {
                winners.push(player.clone());
            }
        }

        winners
    }
}