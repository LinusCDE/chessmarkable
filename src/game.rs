pub use crate::{Player, Square};
use anyhow::Result;
use serde::{Deserialize, Serialize};
pub use tanton::{BitMove, Board, File, Piece, PieceType, Player as TantonPlayer, Rank, SQ};

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ChessOutcome {
    Checkmate { winner: Player },
    Stalemate,
    Aborted { who: Option<Player> },
}

/// Wrapper around tantons board.
/// Aims to be panic safe and not synchronize any internal data, meaning:
///  - no background tasks
///  - no changes without a mut access
pub struct ChessGame {
    board: tanton::Board,
    board_moves_played_offset: u16,
    outcome: Option<ChessOutcome>,
}

impl Default for ChessGame {
    fn default() -> Self {
        Self {
            board: Board::default(),
            board_moves_played_offset: 0,
            outcome: None,
        }
    }
}

impl ChessGame {
    pub fn from_fen(fen: &str) -> Result<ChessGame> {
        let board = match Board::from_fen(fen) {
            Ok(board) => Ok(board),
            Err(e) => Err(anyhow!(
                "Failed to create game board from FEN. Reason: {:?}",
                e,
            )),
        }?;
        Ok(Self {
            board_moves_played_offset: board.moves_played(),
            board,
            ..Default::default()
        })
    }

    pub fn board(&self) -> Board {
        self.board.shallow_clone()
    }

    pub fn fen(&self) -> String {
        self.board.fen()
    }

    pub fn turn(&self) -> Player {
        self.board.turn().into()
    }

    pub fn outcome(&self) -> Option<ChessOutcome> {
        self.outcome
    }

    pub fn total_moves(&self) -> u16 {
        self.board.moves_played()
    }

    pub fn total_undoable_moves(&self) -> u16 {
        self.total_moves() - self.board_moves_played_offset
    }

    pub fn possible_moves(&self) -> tanton::MoveList {
        self.board.generate_moves()
    }

    pub fn player_left(&mut self, player: Player) {
        if self.outcome.is_none() {
            self.outcome = Some(ChessOutcome::Aborted { who: Some(player) });
        }
    }

    pub fn undo(&mut self, count: u16) -> Result<()> {
        if count > self.board.moves_played() {
            return Err(anyhow!(
                "Can't undo {} moves as that rewinds to before the game started.",
                count
            ));
        }
        if count > self.total_undoable_moves() {
            return Err(anyhow!(
                "Can't undo {} moves. (Starting from a FEN/savestate doesn't keep moves).",
                count
            ));
        }

        for _ in 0..count {
            self.board.undo_move();
        }
        self.update_game_outcome();
        Ok(())
    }

    fn piece_on_square(&self, player: Player, square: Square) -> bool {
        self.board
            .get_occupied_player(player.into())
            .into_iter()
            .any(|sq| sq == *square)
    }

    fn update_game_outcome(&mut self) {
        if self.board.checkmate() {
            self.outcome = Some(ChessOutcome::Checkmate {
                winner: self.turn().other_player(),
            });
        } else if self.board.stalemate() {
            self.outcome = Some(ChessOutcome::Stalemate);
        } else if let Some(outcome) = self.outcome {
            match outcome {
                ChessOutcome::Aborted { .. } => {} // Abort is irreversible
                _ => self.outcome = None,
            };
        }
    }

    pub fn move_piece_by_type(
        &mut self,
        piece: Piece,
        destination: Square,
        src_col: Option<File>,
        src_row: Option<Rank>,
    ) -> Result<(Square, Square)> {
        ensure!(
            self.outcome.is_none(),
            "Can't do move since the game has already ended."
        );
        let piece_locations = self.board.get_piece_locations();
        let mut piece_type_locations = vec![];
        for loc in piece_locations {
            if loc.1 == piece {
                piece_type_locations.push(loc.0 .0)
            }
        }
        // Find a legal move for `source` and `destination`
        // (i.e. including promotions or other special data)
        let mut candidate_moves: Vec<BitMove> = Vec::new();
        for legal_move in self.board.generate_moves().iter() {
            if piece_type_locations.contains(&legal_move.get_src_u8())
                && legal_move.get_dest_u8() == destination.0
            {
                candidate_moves.push(legal_move.clone());
            }
        }
        if candidate_moves.is_empty() {
            return Err(anyhow!("Move not found as possibility"));
        }
        let mut selected_move: Option<&BitMove> = None;
        if candidate_moves.len() == 1 {
            selected_move = candidate_moves.first();
        }
        if selected_move.is_none() && src_col.is_some() {
            selected_move = candidate_moves
                .iter()
                .find(|bmove| bmove.src_col() == src_col.unwrap());
        }
        if selected_move.is_none() && src_row.is_some() {
            selected_move = candidate_moves
                .iter()
                .find(|bmove| bmove.src_row() == src_row.unwrap());
        }
        if selected_move.is_none() {
            return Err(anyhow!("Move not found as possibility"));
        }
        let selected_move = selected_move.unwrap();

        self.board.apply_move(selected_move.to_owned());
        if let Err(e) = self.board.is_okay() {
            self.undo(1)?;
            return Err(anyhow!(
                "Board got into illegal state after move. Reason: \"{:?}\"",
                e
            ));
        }

        self.update_game_outcome();
        Ok((Square::from(SQ(selected_move.get_src_u8())), destination))
    }

    pub fn move_piece(&mut self, source: Square, destination: Square) -> Result<()> {
        ensure!(
            self.piece_on_square(self.turn(), source),
            "The playing player has no piece on the source square!"
        );
        ensure!(source != destination, "Move does not actually move");
        ensure!(
            self.outcome.is_none(),
            "Can't do move since the game has already ended."
        );

        // Find a legal move for `source` and `destination`
        // (i.e. including promotions or other special data)
        let mut selected_move: Option<BitMove> = None;
        for legal_move in self.board.generate_moves().iter() {
            if legal_move.get_src_u8() == source.0 && legal_move.get_dest_u8() == destination.0 {
                selected_move = Some(legal_move.clone());
            }
        }
        if selected_move.is_none() {
            return Err(anyhow!("Move not found as possibility"));
        }
        let selected_move = selected_move.unwrap();

        self.board.apply_move(selected_move);
        if let Err(e) = self.board.is_okay() {
            self.undo(1)?;
            return Err(anyhow!(
                "Board got into illegal state after move. Reason: \"{:?}\"",
                e
            ));
        }

        self.update_game_outcome();
        Ok(())
    }
}
