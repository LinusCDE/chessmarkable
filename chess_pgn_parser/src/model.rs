use num::FromPrimitive;

enum_from_primitive! {
    #[derive(PartialEq, Eq, Debug, Copy, Clone)]
    pub enum File {
        A,
        B,
        C,
        D,
        E,
        F,
        G,
        H,
    }
}

enum_from_primitive! {
    #[derive(PartialEq, Eq, Debug, Copy, Clone)]
    pub enum Rank {
        R1,
        R2,
        R3,
        R4,
        R5,
        R6,
        R7,
        R8,
    }
}

enum_from_primitive! {
    //Unknowns are represented as X
    //e.g BX means B file and unknown rank
    #[derive(PartialEq, Eq, Debug, Clone)]
    pub enum Square {
        A1, A2, A3, A4, A5, A6, A7, A8, AX,
        B1, B2, B3, B4, B5, B6, B7, B8, BX,
        C1, C2, C3, C4, C5, C6, C7, C8, CX,
        D1, D2, D3, D4, D5, D6, D7, D8, DX,
        E1, E2, E3, E4, E5, E6, E7, E8, EX,
        F1, F2, F3, F4, F5, F6, F7, F8, FX,
        G1, G2, G3, G4, G5, G6, G7, G8, GX,
        H1, H2, H3, H4, H5, H6, H7, H8, HX,
        X1, X2, X3, X4, X5, X6, X7, X8, XX,
    }
}

impl Square {
    pub fn new(file: Option<File>, rank: Option<Rank>) -> Square {
        let f = match file {
            Some(value) => value as u32,
            None => 8,
        };

        let r = match rank {
            Some(value) => value as u32,
            None => 8,
        };

        Square::new_u32(f, r)
    }

    pub fn new_known(file: File, rank: Rank) -> Square {
        Square::new_u32(file as u32, rank as u32)
    }

    pub fn new_file(file: File) -> Square {
        Square::new_u32(file as u32, 8)
    }

    pub fn new_rank(rank: Rank) -> Square {
        Square::new_u32(8, rank as u32)
    }

    fn new_u32(file: u32, rank: u32) -> Square {
        Square::from_u32(9 * file + rank).unwrap()
    }

    pub fn file(self: &Square) -> Option<File> {
        File::from_u32(self.clone() as u32 / 9)
    }

    pub fn rank(self: &Square) -> Option<Rank> {
        Rank::from_u32(self.clone() as u32 % 9)
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum Piece {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Move {
    BasicMove {
        piece: Piece,
        to: Square,
        from: Square,
        is_capture: bool,
        promoted_to: Option<Piece>,
    },
    CastleKingside,
    CastleQueenside,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum AnnotationSymbol {
    Blunder,
    Mistake,
    Dubious,
    Interesting,
    Good,
    Brilliant,
}

impl Move {
    pub fn new(piece: Piece, to: Square) -> Move {
        Move::BasicMove {
            piece: piece,
            to: to,
            from: Square::XX,
            is_capture: false,
            promoted_to: None,
        }
    }

    pub fn from(&self, square: Square) -> Move {
        match *self {
            Move::BasicMove {
                ref piece,
                ref to,
                from: _,
                is_capture,
                ref promoted_to,
            } => Move::BasicMove {
                piece: piece.clone(),
                to: to.clone(),
                from: square,
                is_capture: is_capture,
                promoted_to: promoted_to.clone(),
            },
            _ => self.clone(),
        }
    }

    pub fn capture(&self) -> Move {
        match *self {
            Move::BasicMove {
                ref piece,
                ref to,
                ref from,
                is_capture: _,
                promoted_to,
            } => Move::BasicMove {
                piece: piece.clone(),
                to: to.clone(),
                from: from.clone(),
                is_capture: true,
                promoted_to: promoted_to,
            },
            _ => self.clone(),
        }
    }

    pub fn with_promotion(&self, piece: Piece) -> Move {
        match *self {
            Move::BasicMove {
                piece: ref piece_,
                ref to,
                ref from,
                is_capture,
                promoted_to: _,
            } => Move::BasicMove {
                piece: piece_.clone(),
                to: to.clone(),
                from: from.clone(),
                is_capture: is_capture,
                promoted_to: Some(piece),
            },
            _ => self.clone(),
        }
    }

    pub fn no_mark(&self) -> MarkedMove {
        MarkedMove {
            move_: self.clone(),
            is_check: false,
            is_checkmate: false,
            annotation_symbol: None,
        }
    }

    pub fn check(&self) -> MarkedMove {
        MarkedMove {
            move_: self.clone(),
            is_check: true,
            is_checkmate: false,
            annotation_symbol: None,
        }
    }

    pub fn checkmate(&self) -> MarkedMove {
        MarkedMove {
            move_: self.clone(),
            is_check: false,
            is_checkmate: true,
            annotation_symbol: None,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct MarkedMove {
    pub move_: Move,
    pub is_check: bool,
    pub is_checkmate: bool,
    pub annotation_symbol: Option<AnnotationSymbol>,
}

impl MarkedMove {
    pub fn annotated(&self, symbol: AnnotationSymbol) -> MarkedMove {
        MarkedMove {
            move_: self.move_.clone(),
            is_check: self.is_check,
            is_checkmate: self.is_checkmate,
            annotation_symbol: Some(symbol),
        }
    }

    pub fn numbered(&self, number: Option<MoveNumber>) -> GameMove {
        GameMove {
            number: number,
            move_: self.clone(),
            nag: None,
            comment: None,
            variations: vec![],
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum MoveNumber {
    White(u32),
    Black(u32),
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct NAG(pub u32);

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct GameMove {
    pub number: Option<MoveNumber>,
    pub move_: MarkedMove,
    pub nag: Option<NAG>,
    pub comment: Option<String>,
    pub variations: Vec<MoveSequence>,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct MoveSequence {
    pub comment: Option<String>,
    pub moves: Vec<GameMove>,
}

impl GameMove {
    pub fn nag(&self, value: NAG) -> GameMove {
        GameMove {
            number: self.number.clone(),
            move_: self.move_.clone(),
            nag: Some(value),
            comment: self.comment.clone(),
            variations: self.variations.clone(),
        }
    }

    pub fn comment(&self, value: String) -> GameMove {
        GameMove {
            number: self.number.clone(),
            move_: self.move_.clone(),
            nag: self.nag.clone(),
            comment: Some(value),
            variations: self.variations.clone(),
        }
    }

    pub fn with_variations(&self, variations: Vec<MoveSequence>) -> GameMove {
        GameMove {
            number: self.number.clone(),
            move_: self.move_.clone(),
            nag: self.nag.clone(),
            comment: self.comment.clone(),
            variations: variations,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum GameTermination {
    WhiteWins,
    BlackWins,
    DrawnGame,
    Unknown,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Game {
    pub tags: Vec<(String, String)>,
    pub comment: Option<String>,
    pub moves: Vec<GameMove>,
    pub termination: GameTermination,
}

#[cfg(test)]
mod tests {
    use super::*;
    use File::*;
    use Rank::*;
    use Square::*;
    use Piece::*;
    use Move::*;
    use MoveNumber::*;
    use AnnotationSymbol::*;

    #[test]
    fn square_new_known() {
        assert_eq!(Square::new_known(C, R3), C3);
    }

    #[test]
    fn square_new() {
        assert_eq!(Square::new(Some(D), Some(R4)), D4);
    }

    #[test]
    fn square_new_with_unknown_rank() {
        assert_eq!(Square::new(Some(E), None), EX);
    }

    #[test]
    fn square_new_with_unknown_file() {
        assert_eq!(Square::new(None, Some(R6)), X6);
    }

    #[test]
    fn square_new_with_both_unknown() {
        assert_eq!(Square::new(None, None), XX);
    }

    #[test]
    fn square_new_file() {
        assert_eq!(Square::new_file(F), FX);
    }

    #[test]
    fn square_new_rank() {
        assert_eq!(Square::new_rank(R7), X7);
    }

    #[test]
    fn square_file_known() {
        assert_eq!(G1.file(), Some(G));
    }

    #[test]
    fn square_file_unknown() {
        assert_eq!(X1.file(), None);
    }

    #[test]
    fn square_rank_known() {
        assert_eq!(H2.rank(), Some(R2));
    }

    #[test]
    fn square_rank_unknown() {
        assert_eq!(HX.rank(), None);
    }


    #[test]
    fn move_new() {
        assert_eq!(
            Move::new(Queen, A2),
            BasicMove {
                piece: Queen,
                to: A2,
                from: XX,
                is_capture: false,
                promoted_to: None,
            }
        );
    }

    #[test]
    fn move_from() {
        assert_eq!(
            Move::new(Queen, A2).from(B1),
            BasicMove {
                piece: Queen,
                to: A2,
                from: B1,
                is_capture: false,
                promoted_to: None,
            }
        );
    }

    #[test]
    fn move_with_promotion() {
        assert_eq!(
            Move::new(Pawn, H8).with_promotion(Queen),
            BasicMove {
                piece: Pawn,
                to: H8,
                from: XX,
                is_capture: false,
                promoted_to: Some(Queen),
            }
        );
    }

    #[test]
    fn marked_move_annotated() {
        assert_eq!(
            Move::new(Queen, A2).no_mark().annotated(Brilliant),
            MarkedMove {
                move_: Move::new(Queen, A2),
                is_check: false,
                is_checkmate: false,
                annotation_symbol: Some(Brilliant),
            }
        );
    }

    #[test]
    fn marked_move_numbered_some() {
        let marked_move = Move::new(Queen, A2).no_mark();

        assert_eq!(
            marked_move.numbered(Some(White(1))),
            GameMove {
                number: Some(White(1)),
                move_: marked_move,
                nag: None,
                comment: None,
                variations: vec![],
            }
        );
    }

    #[test]
    fn marked_move_numbered_none() {
        let marked_move = Move::new(Queen, A2).no_mark();

        assert_eq!(
            marked_move.numbered(None),
            GameMove {
                number: None,
                move_: marked_move,
                nag: None,
                comment: None,
                variations: vec![],
            }
        );
    }

    #[test]
    fn game_move_nag() {
        let marked_move = Move::new(Queen, A2).no_mark();

        let game_move = GameMove {
            number: Some(White(1)),
            move_: marked_move.clone(),
            nag: None,
            comment: Some("Comment".to_string()),
            variations: vec![],
        };

        assert_eq!(
            game_move.nag(NAG(1)),
            GameMove {
                number: Some(White(1)),
                move_: marked_move.clone(),
                nag: Some(NAG(1)),
                comment: Some("Comment".to_string()),
                variations: vec![],
            }
        );
    }

    #[test]
    fn game_move_comment() {
        let marked_move = Move::new(Queen, A2).no_mark();

        let game_move = GameMove {
            number: Some(White(1)),
            move_: marked_move.clone(),
            nag: Some(NAG(1)),
            comment: None,
            variations: vec![],
        };

        assert_eq!(
            game_move.comment("Comment".to_string()),
            GameMove {
                number: Some(White(1)),
                move_: marked_move.clone(),
                nag: Some(NAG(1)),
                comment: Some("Comment".to_string()),
                variations: vec![],
            }
        );
    }


    #[test]
    fn game_move_with_variations() {
        let marked_move = Move::new(Queen, A2).no_mark();

        let game_move = GameMove {
            number: Some(White(1)),
            move_: marked_move.clone(),
            nag: Some(NAG(1)),
            comment: Some("Comment".to_string()),
            variations: vec![],
        };

        let alternative = Move::new(Queen, A1).no_mark().numbered(Some(White(1)));

        assert_eq!(
            game_move.with_variations(vec![
                MoveSequence {
                    comment: None,
                    moves: vec![alternative.clone()],
                },
            ]),
            GameMove {
                number: Some(White(1)),
                move_: marked_move.clone(),
                nag: Some(NAG(1)),
                comment: Some("Comment".to_string()),
                variations: vec![
                    MoveSequence {
                        comment: None,
                        moves: vec![alternative.clone()],
                    },
                ],
            }
        );
    }
}
