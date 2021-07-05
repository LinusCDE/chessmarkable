use super::Scene;
use crate::canvas::*;
use crate::CLI_OPTS;
use anyhow::Result;
use chessmarkable::proto::*;
use chessmarkable::{Player, Square};
use fxhash::{FxHashMap, FxHashSet};
use libremarkable::image;
use libremarkable::input::{multitouch, InputEvent};
use pleco::bot_prelude::*;
use pleco::{BitMove, Board, Piece};
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc::{channel, Receiver, Sender};
use chess_pgn_parser::Game;
use chessmarkable::replay::Replay;
use crate::scene::game_scene::ALL_PIECES;
use crate::scene::piece_images::get_orig_piece_img;
use crate::scene::game_scene::IMG_PIECE_MOVED_TO;
use crate::scene::game_scene::IMG_PIECE_SELECTED;
use crate::scene::game_scene::IMG_PIECE_MOVEHINT;
use crate::scene::game_scene::IMG_PIECE_MOVED_FROM;
use crate::pgns::Pgn;


#[inline]
fn to_square(x: usize, y: usize) -> Square {
    Square::new(x, y).expect("to_square() failed")
}

pub struct ReplayScene {
    board: Board,
    first_draw: bool,
    back_button_hitbox: Option<mxcfb_rect>,
    undo_button_hitbox: Option<mxcfb_rect>,
    next_move_button_hitbox: Option<mxcfb_rect>,
    reset_button_hitbox: Option<mxcfb_rect>,
    full_refresh_button_hitbox: Option<mxcfb_rect>,
    piece_hitboxes: Vec<Vec<mxcfb_rect>>,
    /// The squared that were visually affected and should be redrawn
    redraw_squares: FxHashSet<Square>,
    /// If the amount of changes squares cannot be easily decided this
    /// is a easy way to update everything. Has a performance hit though.
    redraw_all_squares: bool,
    /// Resized to fit selected_square
    square_size: u32,
    img_piece_moved_from: image::DynamicImage,
    img_piece_moved_to: image::DynamicImage,
    piece_padding: u32,
    img_pieces: FxHashMap</* Piece */ char, image::DynamicImage>,
    overlay_padding: u32,
    img_piece_selected: image::DynamicImage,
    img_piece_movehint: image::DynamicImage,
    selected_square: Option<Square>,
    move_hints: FxHashSet<Square>,
    last_move_from: Option<Square>,
    last_move_to: Option<Square>,
    finger_down_square: Option<Square>,
    pub return_to_main_menu: bool,
    /// Do a full screen refresh on next draw
    force_full_refresh: Option<SystemTime>,
    move_comment: Option<String>,
    move_comment_last_rect: Option<mxcfb_rect>,
    is_game_over: bool,
    possible_moves: Vec<(Square, Square)>,
    replay: Replay,
    pub selected_pgn: Option<Pgn>,
}

impl ReplayScene {
    pub fn new(
        replay_info: Option<Game>,
        selected_pgn: Option<Pgn>,
    ) -> Self {
        // Size of board
        let square_size = DISPLAYWIDTH as u32 / 8;
        let piece_padding = square_size / 10;
        let overlay_padding = square_size / 20;

        // Calculate hitboxes
        let mut piece_hitboxes = Vec::new();
        for x in 0..8 {
            let mut y_axis = Vec::new();
            for y in 0..8 {
                y_axis.push(mxcfb_rect {
                    left: ((DISPLAYWIDTH as u32 - square_size * 8) / 2) + square_size * x,
                    top: ((DISPLAYHEIGHT as u32 - square_size * 8) / 2) + square_size * (7 - y),
                    width: square_size,
                    height: square_size,
                });
            }
            piece_hitboxes.push(y_axis);
        }

        // Create resized images
        let mut img_pieces: FxHashMap<char, image::DynamicImage> = Default::default();
        for piece in ALL_PIECES.iter() {
            img_pieces.insert(
                piece.character_lossy(),
                get_orig_piece_img(piece).resize(
                    square_size - piece_padding * 2,
                    square_size - piece_padding * 2,
                    image::FilterType::Lanczos3,
                ),
            );
        }
        let img_piece_selected = IMG_PIECE_SELECTED.resize(
            square_size - overlay_padding * 2,
            square_size - overlay_padding * 2,
            image::FilterType::Lanczos3,
        );
        let img_piece_movehint = IMG_PIECE_MOVEHINT.resize(
            square_size - overlay_padding * 2,
            square_size - overlay_padding * 2,
            image::FilterType::Lanczos3,
        );
        let img_piece_moved_from =
            IMG_PIECE_MOVED_FROM.resize(square_size, square_size, image::FilterType::Lanczos3);
        let img_piece_moved_to =
            IMG_PIECE_MOVED_TO.resize(square_size, square_size, image::FilterType::Lanczos3);

        //Replay Info
        Self {
            board: Board::default(), // Temporary default (usually stays that but will change when having a custom fen)
            first_draw: true,
            piece_hitboxes,
            square_size,
            piece_padding,
            overlay_padding,
            selected_square: None,
            move_hints: Default::default(),
            last_move_from: None,
            last_move_to: None,
            finger_down_square: None,
            img_pieces,
            img_piece_selected,
            img_piece_movehint,
            img_piece_moved_from,
            img_piece_moved_to,
            redraw_squares: Default::default(),
            redraw_all_squares: false,
            back_button_hitbox: None,
            undo_button_hitbox: None,
            next_move_button_hitbox: None,
            reset_button_hitbox: None,
            full_refresh_button_hitbox: None,
            move_comment: None,
            return_to_main_menu: false,
            force_full_refresh: None,
            is_game_over: false,
            possible_moves: vec![],
            replay: Replay::new(replay_info.expect("Couldn't read Replay Info")),
            move_comment_last_rect: None,
            selected_pgn,
        }
    }

    fn draw_board(&mut self, canvas: &mut Canvas) -> Vec<mxcfb_rect> {
        let start = SystemTime::now();
        let mut updated_regions = vec![];
        for x in 0..8 {
            for y in 0..8 {
                let square = to_square(x, y); // Flip board so white is at the bottom
                if !self.redraw_all_squares && !self.redraw_squares.contains(&square) {
                    continue;
                }

                //
                // Square background color
                //
                let is_bright_bg = x % 2 == y % 2;
                let bounds = &self.piece_hitboxes[x][y];
                canvas.fill_rect(
                    Point2 {
                        x: Some(bounds.left as i32),
                        y: Some(bounds.top as i32),
                    },
                    self.piece_hitboxes[x][y].size().cast().unwrap(),
                    if is_bright_bg {
                        color::GRAY(100)
                    } else {
                        color::GRAY(50)
                    },
                );

                //
                // Underlay / Background layers
                //
                // Also highlight squares from previous move
                if self.last_move_from.is_some() && self.last_move_from.unwrap() == square {
                    canvas.draw_image(
                        bounds.top_left().cast().unwrap(),
                        &self.img_piece_moved_from,
                        true,
                    );
                }
                if self.last_move_to.is_some() && self.last_move_to.unwrap() == square {
                    canvas.draw_image(
                        bounds.top_left().cast().unwrap(),
                        &self.img_piece_moved_to,
                        true,
                    );
                }

                //
                // Piece
                //
                let piece = self.board.piece_at_sq(*square);
                if piece != Piece::None {
                    // Actual piece here
                    let piece_img = &self.img_pieces
                        .get(&piece.character_lossy())
                        .expect("Failed to find resized piece img!");
                    canvas.draw_image(
                        Point2 {
                            x: (bounds.left + self.piece_padding) as i32,
                            y: (bounds.top + self.piece_padding) as i32,
                        },
                        &piece_img,
                        true,
                    );
                }

                //
                // Overlay
                //
                // Overlay image if square is selected
                if piece != Piece::None
                    && self.selected_square.is_some()
                    && self.selected_square.unwrap() == square
                {
                    canvas.draw_image(
                        Point2 {
                            x: (bounds.left + self.overlay_padding) as i32,
                            y: (bounds.top + self.overlay_padding) as i32,
                        },
                        &self.img_piece_selected,
                        true,
                    );
                }

                // Display postions a selected chess piece could move to
                if self.move_hints.contains(&square) {
                    canvas.draw_image(
                        Point2 {
                            x: (bounds.left + self.overlay_padding) as i32,
                            y: (bounds.top + self.overlay_padding) as i32,
                        },
                        &self.img_piece_movehint,
                        true,
                    );
                }

                updated_regions.push(bounds.clone());
            }
        }

        if self.redraw_all_squares || !CLI_OPTS.no_merge {
            // Update full board instead of every single position
            updated_regions.clear();
            updated_regions.push(self.full_board_rect());
        }

        let squares = if self.redraw_all_squares {
            8 * 8
        } else {
            self.redraw_squares.len()
        };
        let dur = start.elapsed().unwrap();
        debug!(
            "{} squares redrawn in {:?} ({:?} per square)",
            squares,
            dur,
            dur / squares as u32
        );

        self.redraw_squares.clear();
        self.redraw_all_squares = false;

        updated_regions
    }

    fn full_board_rect(&self) -> mxcfb_rect {
        let left = self.piece_hitboxes[0][7].left;
        let top = self.piece_hitboxes[0][7].top;
        let right = self.piece_hitboxes[7][0].left + self.piece_hitboxes[7][0].width;
        let bottom = self.piece_hitboxes[7][0].top + self.piece_hitboxes[7][0].height;
        mxcfb_rect {
            left,
            top,
            width: right - left,
            height: bottom - top,
        }
    }

    fn clear_move_hints(&mut self) {
        for last_move_hint in &self.move_hints {
            self.redraw_squares.insert(last_move_hint.clone());
        }
        self.move_hints.clear();
    }

    fn set_move_hints(&mut self, square: Square) {
        self.clear_move_hints();

        for (src, dest) in self.possible_moves.iter() {
            if *src == square {
                self.move_hints.insert(*dest);
                self.redraw_squares.insert(*dest);
            }
        }
    }

    fn on_user_move(&mut self, src: Square, dest: Square) {
        let response = self.replay.player_move(src, dest);
        self.update_board(&*response.fen);
        self.clear_state_post_move();
    }

    fn clear_state_post_move(&mut self) {
        self.selected_square = None;
        self.finger_down_square = None;
        self.clear_move_hints();
        self.clear_last_moved_hints();
        self.possible_moves = self.replay.possible_moves().iter()
            .map(|bit_move| (bit_move.get_src().into(), bit_move.get_dest().into()))
            .collect();
    }

    fn clear_last_moved_hints(&mut self) {
        for last_move_hint in self.last_move_from.iter().chain(self.last_move_to.iter()) {
            self.redraw_squares.insert(last_move_hint.clone());
        }
        self.last_move_from = None;
        self.last_move_to = None;
    }

    fn update_board(&mut self, fen: &str) {
        if self.board.fen() == fen {
            debug!("Ignored unchanged board");
        }
        info!("Updated FEN: {}", fen);

        let new_board = match Board::from_fen(fen) {
            Ok(board) => board,
            Err(e) => {
                warn!("Failed to parse fen \"{}\". Error: {:?}", fen, e);
                return;
            }
        };

        // Find updated squares
        for x in 0..8 {
            for y in 0..8 {
                let sq = to_square(x, y);
                let old_piece = self.board.piece_at_sq(*sq);
                let new_piece = new_board.piece_at_sq(*sq);

                if old_piece != new_piece {
                    self.redraw_squares.insert(sq);
                }
            }
        }

        self.board = new_board;
    }
}

impl Scene for ReplayScene {
    fn on_input(&mut self, event: InputEvent) {
        match event {
            InputEvent::MultitouchEvent { event } => {
                // Taps and buttons
                match event {
                    multitouch::MultitouchEvent::Press { finger } => {
                        for x in 0..8 {
                            for y in 0..8 {
                                if Canvas::is_hitting(finger.pos, self.piece_hitboxes[x][y]) {
                                    self.finger_down_square = Some(to_square(x, y));
                                }
                            }
                        }
                        if self.back_button_hitbox.is_some()
                            && Canvas::is_hitting(finger.pos, self.back_button_hitbox.unwrap())
                        {
                            self.return_to_main_menu = true;
                        } else if self.full_refresh_button_hitbox.is_some()
                            && Canvas::is_hitting(
                            finger.pos,
                            self.full_refresh_button_hitbox.unwrap(),
                        )
                        {
                            self.force_full_refresh = Some(SystemTime::now());
                        } else if self.next_move_button_hitbox.is_some()
                            && Canvas::is_hitting(
                            finger.pos,
                            self.next_move_button_hitbox.unwrap(),
                        )
                        {
                            let replay_response = self.replay.play_replay_move();
                            self.update_board(&*replay_response.fen);
                            self.clear_state_post_move();
                            self.move_comment = replay_response.comment;
                        } else if self.reset_button_hitbox.is_some()
                            && Canvas::is_hitting(
                            finger.pos,
                            self.reset_button_hitbox.unwrap(),
                        ) {
                            let replay_response = self.replay.reset();
                            self.update_board(&*replay_response.fen);
                            self.clear_state_post_move();
                            self.move_comment = replay_response.comment;
                        } else if self.undo_button_hitbox.is_some()
                            && Canvas::is_hitting(
                            finger.pos,
                            self.undo_button_hitbox.unwrap(),
                        ) {
                            let replay_response = self.replay.undo_move();
                            self.update_board(&*replay_response.fen);
                            self.clear_state_post_move();
                            self.move_comment = replay_response.comment;
                        }
                    }
                    multitouch::MultitouchEvent::Release { finger } => {
                        if !self.is_game_over {
                            for x in 0..8 {
                                for y in 0..8 {
                                    if Canvas::is_hitting(finger.pos, self.piece_hitboxes[x][y]) {
                                        let new_square = to_square(x, y);
                                        if let Some(last_selected_square) = self.selected_square {
                                            self.redraw_squares
                                                .insert(last_selected_square.clone());

                                            if last_selected_square == new_square {
                                                // Cancel move
                                                self.selected_square = None;
                                                self.clear_move_hints();
                                            } else {
                                                // Attempt to move from last_selected_square to new_square if move is
                                                // in self.possible_moves. Otherwise just select the piece on new_square.
                                                // See https://github.com/LinusCDE/chessmarkable/issues/14
                                                let is_possible_move = self
                                                    .possible_moves
                                                    .iter()
                                                    .any(|(possible_src, possible_dest)| {
                                                        possible_src == &last_selected_square
                                                            && possible_dest == &new_square
                                                    });
                                                if is_possible_move {
                                                    // Move
                                                    self.redraw_squares.insert(new_square.clone());
                                                    self.on_user_move(
                                                        last_selected_square,
                                                        new_square,
                                                    );
                                                } else {
                                                    // Select new_square as new selected piece
                                                    if self.board.piece_at_sq(*new_square)
                                                        != Piece::None
                                                    {
                                                        self.selected_square = Some(new_square);
                                                        self.redraw_squares
                                                            .insert(new_square.clone());
                                                        self.set_move_hints(new_square);
                                                    } else {
                                                        // Clear selection
                                                        self.selected_square = None;
                                                        self.clear_move_hints();
                                                    }
                                                }
                                            }
                                        } else {
                                            let finger_down_square = self
                                                .finger_down_square
                                                .unwrap_or(new_square.clone());
                                            if finger_down_square.0 != new_square.0 {
                                                // Do immeate move (swiped) without highlighting

                                                self.redraw_squares
                                                    .insert(finger_down_square.clone());
                                                self.on_user_move(finger_down_square, new_square);
                                            } else {
                                                // Mark square
                                                if self.board.piece_at_sq(*new_square)
                                                    != Piece::None
                                                {
                                                    self.selected_square = Some(new_square);
                                                    self.redraw_squares.insert(new_square.clone());
                                                    self.set_move_hints(new_square);
                                                }
                                            }
                                        };
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn draw(&mut self, canvas: &mut Canvas) {
        if self.first_draw {
            // First frame
            canvas.clear();
            self.back_button_hitbox = Some(canvas.draw_button(
                Point2 {
                    x: Some(50),
                    y: Some(1770),
                },
                "Quit",
                75.0,
                10,
                20,
            ));
            self.full_refresh_button_hitbox = Some(canvas.draw_button(
                Point2 {
                    x: Some((DISPLAYWIDTH - 260) as i32),
                    y: Some(1770),
                },
                "Refresh",
                75.0,
                10,
                20,
            ));
            self.reset_button_hitbox = Some(canvas.draw_button(
                Point2 {
                    x: Some(
                        self.back_button_hitbox.unwrap().left as i32 +
                            self.back_button_hitbox.unwrap().width as i32
                            + 405
                    ),
                    y: Some(1770),
                },
                "Reset",
                60.0,
                50,
                50,
            ));
            self.undo_button_hitbox = Some(canvas.draw_button(
                Point2 {
                    x: Some(
                        self.reset_button_hitbox.unwrap().left as i32
                            - 200
                    ),
                    y: Some(1780),
                },
                "<",
                125.0,
                60,
                100,
            ));
            self.next_move_button_hitbox = Some(canvas.draw_button(
                Point2 {
                    x: Some(
                        self.reset_button_hitbox.unwrap().left as i32
                            + self.reset_button_hitbox.unwrap().width as i32
                            + 150
                    ),
                    y: Some(1780),
                },
                ">",
                125.0,
                60,
                100,
            ));
            self.redraw_all_squares = true;
            self.draw_board(canvas);
            canvas.update_full();
            self.first_draw = false;
            // Refresh again after 500ms
            self.force_full_refresh = Some(SystemTime::now() + Duration::from_millis(250));
        }


        // Update board
        if self.redraw_all_squares || self.redraw_squares.len() > 0 {
            self.draw_board(canvas).iter().for_each(|r| {
                canvas.update_partial(r);
            });
            self.redraw_all_squares = false;
        }

        // Do forced refresh on request
        if self.force_full_refresh.is_some() && self.force_full_refresh.unwrap() < SystemTime::now()
        {
            canvas.update_full();
            self.force_full_refresh = None;
        }

        // Clear previous text when changed or expired
        if self.move_comment_last_rect.is_some()
            && self.move_comment.is_some()
        {
            // Clear any previous text
            if let Some(ref last_rect) = self.move_comment_last_rect {
                canvas.fill_rect(
                    Point2 {
                        x: Some(last_rect.left as i32),
                        y: Some(last_rect.top as i32),
                    },
                    Vector2 {
                        x: last_rect.width,
                        y: last_rect.height,
                    },
                    color::WHITE,
                );
                canvas.update_partial(last_rect);
                self.move_comment_last_rect = None;
            }
        }

        // Draw a requested text once
        if self.move_comment.is_some() {
            if let Some(ref comment) = self.move_comment {
                // Old text was cleared above already

                let rect = canvas.draw_multi_line_text(
                    None,
                    50,
                    comment,
                    100,
                    35.0,
                );
                canvas.update_partial(&rect);
                self.move_comment_last_rect = Some(rect);
                self.move_comment = None;
            }
        }
    }
}
