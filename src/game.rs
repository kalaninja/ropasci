use frame_support::pallet_prelude::*;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum GameStage {
    Betting { participating_players: u64 },
    Revealing { anticipated_players: u64 },
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct Game<BlockNumber, Balance> {
    pub start: BlockNumber,
    pub round_length: BlockNumber,
    pub bet: Balance,
    pub stage: GameStage,
}

impl<BlockNumber, Balance> Game<BlockNumber, Balance> {
    pub fn start(start: BlockNumber, round_length: BlockNumber, bet: Balance) -> Self {
        Self {
            start,
            round_length,
            bet,
            stage: GameStage::Betting { participating_players: 1 },
        }
    }

    pub fn join(&mut self) {
        match self.stage {
            GameStage::Betting { participating_players } =>
                self.stage = GameStage::Betting { participating_players: participating_players + 1 },
            _ => unreachable!("Joining a game that is not in betting stage"),
        }
    }

    pub fn start_revealing(&mut self) {
        match self.stage {
            GameStage::Betting { participating_players } =>
                self.stage = GameStage::Revealing { anticipated_players: participating_players },
            _ => unreachable!("Start revealing a game that is not in betting stage"),
        }
    }

    pub fn reveal(&mut self) {
        match self.stage {
            GameStage::Revealing { anticipated_players } =>
                self.stage = GameStage::Revealing { anticipated_players: anticipated_players - 1 },
            _ => unreachable!("Revealing a game that is not in revealing stage"),
        }
    }

    pub fn last_revealing(&self) -> bool {
        matches!(self.stage, GameStage::Revealing{anticipated_players: 1})
    }
}

#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum Hand {
    Rock,
    Paper,
    Scissors,
}

impl Hand {
    pub fn new(value: u8) -> Option<Self> {
        match value {
            0 => Some(Hand::Rock),
            1 => Some(Hand::Paper),
            2 => Some(Hand::Scissors),
            _ => None,
        }
    }

    pub fn beats(&self, other: &Self) -> bool {
        match (self, other) {
            (Hand::Rock, Hand::Scissors) => true,
            (Hand::Paper, Hand::Rock) => true,
            (Hand::Scissors, Hand::Paper) => true,
            _ => false,
        }
    }

    pub fn beaten_by(&self) -> Self {
        match self {
            Hand::Rock => Hand::Paper,
            Hand::Paper => Hand::Scissors,
            Hand::Scissors => Hand::Rock,
        }
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub struct Move<MoveHash> {
    pub hash: MoveHash,
    pub hand: Option<Hand>,
}

impl<MoveHash> Move<MoveHash> {
    pub fn new(hash: MoveHash) -> Self {
        Self {
            hash,
            hand: None,
        }
    }

    pub fn reveal(&mut self, move_reveal: &[u8]) -> Result<(), ()> {
        move_reveal.first()
            .and_then(|&value| Hand::new(value))
            .map(|hand| {
                self.hand = Some(hand);
            })
            .ok_or(())
    }
}
