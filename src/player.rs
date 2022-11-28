use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt;
use tanton::Player as TantonPlayer;

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

impl From<TantonPlayer> for Player {
    fn from(tanton_player: TantonPlayer) -> Self {
        match tanton_player {
            TantonPlayer::Black => Player::Black,
            TantonPlayer::White => Player::White,
        }
    }
}

impl Into<TantonPlayer> for Player {
    fn into(self) -> TantonPlayer {
        match self {
            Player::Black => TantonPlayer::Black,
            Player::White => TantonPlayer::White,
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
