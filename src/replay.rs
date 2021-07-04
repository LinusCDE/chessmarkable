use crate::game::ChessGame;
use crate::game::Player as PlecoPlayer;
pub use crate::game::{ChessOutcome, SQ};
use crate::{Player, Square};
use anyhow::{Context, Result};
use pleco::tools::Searcher;
use serde::{Deserialize, Serialize};
use std::thread;
use std::time::{Duration, SystemTime};
use tokio::stream::StreamExt;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::task;
use chess_pgn_parser::{Game, GameMove, Move};
use chess_pgn_parser::Piece as LocalPiece;
use chess_pgn_parser::Rank as LocalRank;
use chess_pgn_parser::File as LocalFile;
use pleco::{Piece, Rank, File};
use pleco::core::Piece::{WhitePawn, WhiteKnight, WhiteBishop, WhiteRook, WhiteQueen, WhiteKing, BlackPawn, BlackKing, BlackKnight, BlackBishop, BlackRook, BlackQueen};

const FEN_TAG: &str = "FEN";

pub struct ReplayResponse {
    pub fen: String,
    pub comment: Option<String>
}

pub struct Replay {
    active_game_state: ChessGame,
    replay_info: Game,
    replay_moves_played_offset: usize,
    game_started: bool,
    is_white_turn: bool
}

impl Replay {
    pub fn new(
        replay_info: Game
    ) -> Self {
        let starting_fen = replay_info.tags.iter().find(|tag| tag.to_owned().0 == FEN_TAG);
        let active_game_state = match starting_fen {
            Some(fen) =>
                ChessGame::from_fen(&*fen.1).unwrap(),
            _ => {
                ChessGame::default()
            }
        };
        Self {
            active_game_state,
            replay_info,
            replay_moves_played_offset: 0,
            game_started: false,
            is_white_turn: true
        }
    }

    pub fn play_replay_move(&mut self) -> ReplayResponse {
        if self.replay_moves_played_offset + 1 < self.replay_info.moves.len() {
            let played_move: GameMove = self.replay_info.moves[self.replay_moves_played_offset].clone();
            let played_move = played_move.move_.move_;
            let played_piece = match played_move {
                Move::BasicMove { piece, .. } => to_pleco_piece(piece, self.is_white_turn),
                Move::CastleKingside => if self.is_white_turn { WhiteKing } else { BlackKing }
                Move::CastleQueenside => if self.is_white_turn { WhiteQueen } else { BlackQueen }
            };
            let destination = match played_move.clone() {
                Move::BasicMove { to, .. } => SQ::make(to_pleco_file(to.file()).unwrap(), to_pleco_rank(to.rank()).unwrap()),
                Move::CastleKingside => if self.is_white_turn { SQ::make(File::H, Rank::R1) } else { SQ::make(File::H, Rank::R8) }
                Move::CastleQueenside => if self.is_white_turn { SQ::make(File::A, Rank::R1) } else { SQ::make(File::A, Rank::R8) }
            };
            let (src_col, src_row) = match played_move.clone() {
                Move::BasicMove { from, .. } => (to_pleco_file(from.file()), to_pleco_rank(from.rank())),
                Move::CastleKingside => (Some(File::E), if self.is_white_turn { Some(Rank::R1) } else { Some(Rank::R8) }),
                Move::CastleQueenside => (Some(File::E), if self.is_white_turn { Some(Rank::R1) } else { Some(Rank::R8) })
            };
            self.active_game_state.move_piece_by_type(played_piece, Square::from(destination), src_col, src_row);
            self.is_white_turn = !self.is_white_turn;
            self.replay_moves_played_offset = self.replay_moves_played_offset + 1;
        }
        return ReplayResponse{fen: self.active_game_state.fen(), comment: None }
    }
    //
    // pub fn player_move(&mut self, source: Square, destination: Square) -> Result<String> {
    //
    // }
    //
    // pub fn undo_move(&mut self) -> Result<String> {
    //
    // }
    //
    pub fn reset(&mut self) -> ReplayResponse {
        self.active_game_state = ChessGame::default();
        self.replay_moves_played_offset = 0;
        self.is_white_turn = true;
        return ReplayResponse{fen: self.active_game_state.fen(), comment: None }
    }
}

fn to_pleco_piece(piece: LocalPiece, is_white_turn: bool) -> Piece {
    if is_white_turn {
        match piece {
            LocalPiece::Pawn => WhitePawn,
            LocalPiece::Knight => WhiteKnight,
            LocalPiece::Bishop => WhiteBishop,
            LocalPiece::Rook => WhiteRook,
            LocalPiece::Queen => WhiteQueen,
            LocalPiece::King => WhiteKing
        }
    } else {
        match piece {
            LocalPiece::Pawn => BlackPawn,
            LocalPiece::Knight => BlackKnight,
            LocalPiece::Bishop => BlackBishop,
            LocalPiece::Rook => BlackRook,
            LocalPiece::Queen => BlackQueen,
            LocalPiece::King => BlackKing
        }
    }
}

fn to_pleco_rank(rank: Option<LocalRank>) -> Option<Rank> {
    match rank {
        Some(LocalRank::R1) => Some(Rank::R1),
        Some(LocalRank::R2) => Some(Rank::R2),
        Some(LocalRank::R3) => Some(Rank::R3),
        Some(LocalRank::R4) => Some(Rank::R4),
        Some(LocalRank::R5) => Some(Rank::R5),
        Some(LocalRank::R6) => Some(Rank::R6),
        Some(LocalRank::R7) => Some(Rank::R7),
        Some(LocalRank::R8) => Some(Rank::R8),
        _ => None
    }
}

fn to_pleco_file(file: Option<LocalFile>) -> Option<File> {
    if file.is_some() {
        match file {
            Some(LocalFile::A) => Some(File::A),
            Some(LocalFile::B) => Some(File::B),
            Some(LocalFile::C) => Some(File::C),
            Some(LocalFile::D) => Some(File::D),
            Some(LocalFile::E) => Some(File::E),
            Some(LocalFile::F) => Some(File::F),
            Some(LocalFile::G) => Some(File::G),
            Some(LocalFile::H) => Some(File::H),
            _ => None
        }
    } else {
        None
    }
}
