pub use crate::game::SQ;
use crate::game::{File, Rank};
use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_string_derive::SerdeDisplayFromStr;
use std::fmt;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SquareFormatError {
    #[error("The square in not within the acceptable range (found: {found}, expected: between A1 and H8)")]
    OutOfRange { found: String },
    #[error("The square has to contain two chars (found: {found}, expected: a value with two chars in range A1 to H8)")]
    InvalidLength { found: String },
}

#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug, SerdeDisplayFromStr)]
#[expected_data_description = "a chess square donated with a letter and a number (e.g. A1)"]
pub struct Square {
    sq: SQ,
}

impl Square {
    pub fn new(x: usize, y: usize) -> Result<Self> {
        let x = x as u8;
        let y = y as u8;

        let file = match x {
            x if x == File::A as u8 => File::A,
            x if x == File::B as u8 => File::B,
            x if x == File::C as u8 => File::C,
            x if x == File::D as u8 => File::D,
            x if x == File::E as u8 => File::E,
            x if x == File::F as u8 => File::F,
            x if x == File::G as u8 => File::G,
            x if x == File::H as u8 => File::H,
            _ => bail!("Invalid File index for pos"),
        };
        let rank = match y {
            y if y == Rank::R1 as u8 => Rank::R1,
            y if y == Rank::R2 as u8 => Rank::R2,
            y if y == Rank::R3 as u8 => Rank::R3,
            y if y == Rank::R4 as u8 => Rank::R4,
            y if y == Rank::R5 as u8 => Rank::R5,
            y if y == Rank::R6 as u8 => Rank::R6,
            y if y == Rank::R7 as u8 => Rank::R7,
            y if y == Rank::R8 as u8 => Rank::R8,
            _ => bail!("Invalid Rank index for pos"),
        };
        Ok(Square {
            sq: SQ::make(file, rank),
        })
    }

    pub fn x(&self) -> u8 {
        self.file() as u8
    }

    pub fn y(&self) -> u8 {
        self.rank() as u8
    }
}

impl From<SQ> for Square {
    fn from(sq: SQ) -> Self {
        Square { sq }
    }
}

impl Into<SQ> for Square {
    fn into(self) -> SQ {
        *self.clone()
    }
}

impl std::str::FromStr for Square {
    type Err = SquareFormatError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        const FILES: &[char] = &['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H'];
        const RANKS: &[char] = &['1', '2', '3', '4', '5', '6', '7', '8'];

        let chars: Vec<_> = s.chars().collect();
        if (chars.len() != 2) {
            return Err(SquareFormatError::InvalidLength {
                found: s.to_owned(),
            });
        }

        let file_index = match FILES.iter().position(|f| f == &chars[0]) {
            Some(pos) => pos,
            None => {
                return Err(SquareFormatError::OutOfRange {
                    found: s.to_owned(),
                })
            }
        };

        let rank_index = match RANKS.iter().position(|f| f == &chars[1]) {
            Some(pos) => pos,
            None => {
                return Err(SquareFormatError::OutOfRange {
                    found: s.to_owned(),
                })
            }
        };

        Ok(Square::new(file_index, rank_index).unwrap())
    }
}

impl std::ops::Deref for Square {
    type Target = SQ;

    fn deref(&self) -> &Self::Target {
        &self.sq
    }
}

impl std::ops::DerefMut for Square {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sq
    }
}

/*
impl Serialize for Square {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> serde::de::Visitor<'de> for Square {
    // The type that our Visitor is going to produce.
    type Value = Square;

    // Format a message stating what data this Visitor expects to receive.
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a chess square donated with a letter and a number (e.g. A1)")
    }

    fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        v.parse()
            .map_err(|e| serde::de::Error::custom::<<Square as std::str::FromStr>::Err>(e))
    }
}
*/
