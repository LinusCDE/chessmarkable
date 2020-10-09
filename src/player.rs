use anyhow::Result;
use pleco::Player as PlecoPlayer;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug, Serialize, Deserialize)]
#[repr(u8)]
pub enum Player {
    White = 0,
    Black = 1,
}

impl Player {
    pub fn other_player(&self) -> Self {
        match self {
            Player::Black => Player::White,
            Player::White => Player::Black,
        }
    }
}

impl From<PlecoPlayer> for Player {
    fn from(pleco_player: PlecoPlayer) -> Self {
        match pleco_player {
            PlecoPlayer::Black => Player::Black,
            PlecoPlayer::White => Player::White,
        }
    }
}

impl Into<PlecoPlayer> for Player {
    fn into(self) -> PlecoPlayer {
        match self {
            Player::Black => PlecoPlayer::Black,
            Player::White => PlecoPlayer::White,
        }
    }
}

impl std::str::FromStr for Player {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "black" => Ok(Player::Black),
            "white" => Ok(Player::White),
            _ => Err(anyhow!(
                "Specified player is neither \"Black\" nor \"White\""
            )),
        }
    }
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
