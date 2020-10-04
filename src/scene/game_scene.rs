use super::Scene;
use crate::canvas::*;
use crate::CLI_OPTS;
use fxhash::{FxHashMap, FxHashSet};
use libremarkable::image;
use libremarkable::input::{gpio, multitouch, multitouch::Finger, InputEvent};
use pleco::bot_prelude::*;
use pleco::{BitMove, Board, File, Piece, Rank, SQ};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::{Duration, SystemTime};

lazy_static! {
    // Black set
    static ref IMG_KING_BLACK: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../res/king-black.png"))
            .expect("Failed to load resource as image!");
    static ref IMG_QUEEN_BLACK: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../res/queen-black.png"))
            .expect("Failed to load resource as image!");
    static ref IMG_BISHOP_BLACK: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../res/bishop-black.png"))
            .expect("Failed to load resource as image!");
    static ref IMG_ROOK_BLACK: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../res/rook-black.png"))
            .expect("Failed to load resource as image!");
    static ref IMG_KNIGHT_BLACK: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../res/knight-black.png"))
            .expect("Failed to load resource as image!");
    static ref IMG_PAWN_BLACK: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../res/pawn-black.png"))
            .expect("Failed to load resource as image!");

    // White set
    static ref IMG_KING_WHITE: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../res/king-white.png"))
            .expect("Failed to load resource as image!");
    static ref IMG_QUEEN_WHITE: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../res/queen-white.png"))
            .expect("Failed to load resource as image!");
    static ref IMG_BISHOP_WHITE: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../res/bishop-white.png"))
            .expect("Failed to load resource as image!");
    static ref IMG_ROOK_WHITE: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../res/rook-white.png"))
            .expect("Failed to load resource as image!");
    static ref IMG_KNIGHT_WHITE: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../res/knight-white.png"))
            .expect("Failed to load resource as image!");
    static ref IMG_PAWN_WHITE: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../res/pawn-white.png"))
            .expect("Failed to load resource as image!");

    // Additional overlays
    static ref IMG_PIECE_SELECTED: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../res/piece-selected.png"))
            .expect("Failed to load resource as image!");
    static ref IMG_PIECE_MOVEHINT: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../res/piece-move-hint.png"))
            .expect("Failed to load resource as image!");
}

const ALL_PIECES: &[Piece] = &[
    Piece::BlackKing,
    Piece::BlackQueen,
    Piece::BlackBishop,
    Piece::BlackRook,
    Piece::BlackKnight,
    Piece::BlackPawn,
    Piece::WhiteKing,
    Piece::WhiteQueen,
    Piece::WhiteBishop,
    Piece::WhiteRook,
    Piece::WhiteKnight,
    Piece::WhitePawn,
];

fn to_square(x: usize, y: usize) -> SQ {
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
        _ => panic!("Invalid file for pos"),
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
        _ => panic!("Invalid rank for pos"),
    };
    SQ::make(file, rank)
}

#[derive(Clone, Copy, PartialEq)]
pub enum GameMode {
    PvP = 0,
    EasyBot = 1,
    NormalBot = 2,
    HardBot = 4,
    // Could go up to about 8-10 (depending on the algo) before getting too slow. But probably fairly unbeatable then.
}

pub struct GameScene {
    board: Board,
    game_mode: GameMode,
    first_draw: bool,
    /// Likely because it's currently the turn of the bot
    ignore_user_moves: bool,
    bot_job: Sender<Option<(Board, u16)>>,
    bot_move: Receiver<BitMove>,
    back_button_hitbox: Option<mxcfb_rect>,
    undo_button_hitbox: Option<mxcfb_rect>,
    full_refresh_button_hitbox: Option<mxcfb_rect>,
    square_size: u32,
    piece_padding: u32,
    overlay_padding: u32,
    piece_hitboxes: Vec<Vec<mxcfb_rect>>,
    /// The squared that were visually affected and should be redrawn
    redraw_squares: FxHashSet<SQ>,
    /// If the amount of changes squares cannot be easily decided this
    /// is a easy way to update everything. Has a performance hit though.
    redraw_all_squares: bool,
    selected_square: Option<SQ>,
    move_hints: FxHashSet<SQ>,
    /// Remember a press to decide whether to show options or do a move at once
    finger_down_square: Option<SQ>,
    /// Resized to fit selected_square
    img_pieces: FxHashMap</* Piece */ char, image::DynamicImage>,
    img_piece_selected: image::DynamicImage,
    img_piece_movehint: image::DynamicImage,
    pub back_button_pressed: bool,
    /// Do a full screen refresh on next draw
    force_full_refresh: Option<SystemTime>,
    last_checkmate_check: SystemTime,
}

impl GameScene {
    pub fn new(game_mode: GameMode) -> Self {
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
                Self::get_orig_pice_img(piece).resize(
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

        let (bot_job_tx, bot_job_rx) = channel();
        let (bot_move_tx, bot_move_rx) = channel();
        if game_mode != GameMode::PvP {
            Self::spawn_bot_thread(bot_job_rx, bot_move_tx);
        }

        Self {
            board: Default::default(),
            first_draw: true,
            bot_job: bot_job_tx,
            bot_move: bot_move_rx,
            ignore_user_moves: false,
            game_mode,
            piece_hitboxes,
            square_size,
            piece_padding,
            overlay_padding,
            selected_square: None,
            move_hints: Default::default(),
            finger_down_square: None,
            img_pieces,
            img_piece_selected,
            img_piece_movehint,
            redraw_squares: Default::default(),
            redraw_all_squares: false,
            back_button_hitbox: None,
            undo_button_hitbox: None,
            full_refresh_button_hitbox: None,
            back_button_pressed: false,
            force_full_refresh: None,
            last_checkmate_check: SystemTime::now(),
        }
    }

    fn get_orig_pice_img(piece: &Piece) -> &'static image::DynamicImage {
        match *piece {
            Piece::BlackKing => &IMG_KING_BLACK,
            Piece::BlackQueen => &IMG_QUEEN_BLACK,
            Piece::BlackBishop => &IMG_BISHOP_BLACK,
            Piece::BlackRook => &IMG_ROOK_BLACK,
            Piece::BlackKnight => &IMG_KNIGHT_BLACK,
            Piece::BlackPawn => &IMG_PAWN_BLACK,
            Piece::WhiteKing => &IMG_KING_WHITE,
            Piece::WhiteQueen => &IMG_QUEEN_WHITE,
            Piece::WhiteBishop => &IMG_BISHOP_WHITE,
            Piece::WhiteRook => &IMG_ROOK_WHITE,
            Piece::WhiteKnight => &IMG_KNIGHT_WHITE,
            Piece::WhitePawn => &IMG_PAWN_WHITE,
            Piece::None => panic!("Cannot get img for Piece::None"),
        }
    }

    fn draw_board(&mut self, canvas: &mut Canvas) {
        for x in 0..8 {
            for y in 0..8 {
                let square = to_square(x, y); // Flip board so white is at the bottom
                if !self.redraw_all_squares && !self.redraw_squares.contains(&square) {
                    continue;
                }

                let is_bright_bg = x % 2 == y % 2;
                let bounds = &self.piece_hitboxes[x][y];
                canvas.fill_rect(
                    Point2 {
                        x: Some(bounds.left as i32),
                        y: Some(bounds.top as i32),
                    },
                    self.piece_hitboxes[x][y].size().cast().unwrap(),
                    if is_bright_bg {
                        color::GRAY(50)
                    } else {
                        color::GRAY(100)
                    },
                );
                let piece = self.board.piece_at_sq(square);
                if piece != Piece::None {
                    // Actual piece here
                    let piece_img = self
                        .img_pieces
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

                    // Overlay image if square is selected
                    if self.selected_square.is_some() && self.selected_square.unwrap() == square {
                        canvas.draw_image(
                            Point2 {
                                x: (bounds.left + self.overlay_padding) as i32,
                                y: (bounds.top + self.overlay_padding) as i32,
                            },
                            &self.img_piece_selected,
                            true,
                        );
                    }
                }

                // Overlay image if square is selected
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
            }
        }

        self.redraw_squares.clear();
        self.redraw_all_squares = false;
    }

    fn spawn_bot_thread(
        job: Receiver<Option<(Board, u16)>>,
        job_result: Sender<BitMove>,
    ) -> thread::JoinHandle<()> {
        thread::Builder::new()
            .name("ChessBot".to_owned())
            .spawn(move || loop {
                let job_data = job.recv().unwrap();
                if job_data.is_none() {
                    // Abort requested
                    println!("Bot thread is terminating");
                    break;
                }
                let (board, depth) = job_data.unwrap();
                let started = SystemTime::now();
                let bot_move = Self::do_bot_move(board, depth);
                let elapsed = started.elapsed().unwrap_or(Duration::new(0, 0));
                let reaction_delay = Duration::from_millis(CLI_OPTS.bot_reaction_delay.into());

                if elapsed < reaction_delay {
                    thread::sleep(reaction_delay - elapsed);
                }
                //let elapsed =
                job_result.send(bot_move).ok();
            })
            .unwrap()
    }

    fn do_bot_move(board: Board, depth: u16) -> BitMove {
        println!("Bot is working...");
        //let depth = board.depth() + 1; // Should probably be this
        let bot_bit_move = JamboreeSearcher::best_move(board, depth);
        bot_bit_move
    }

    fn try_move(&mut self, bit_move: BitMove) -> Result<(), String> {
        let mut selected_move: Option<BitMove> = None;
        for legal_move in self.board.generate_moves().iter() {
            if legal_move.get_src_u8() == bit_move.get_src_u8()
                && legal_move.get_dest_u8() == bit_move.get_dest_u8()
            {
                selected_move = Some(legal_move.clone());
            }
        }
        if selected_move.is_none() {
            return Err("Move not found as possibility".to_owned());
        }
        let selected_move = selected_move.unwrap();

        self.board.apply_move(selected_move);
        if let Err(e) = self.board.is_okay() {
            self.board.undo_move();
            return Err(format!("Board got into illegal state after move: {:?}", e));
        }

        if selected_move.is_castle() {
            // More than just src and dest changed
            self.redraw_all_squares = true;
        }

        Ok(())
    }

    fn clear_move_hints(&mut self) {
        for last_move_hint in &self.move_hints {
            self.redraw_squares.insert(last_move_hint.clone());
        }
        self.move_hints.clear();
    }

    fn set_move_hints(&mut self, square: SQ) {
        self.clear_move_hints();

        for legal_move in self.board.generate_moves().iter() {
            if legal_move.get_src() == square {
                self.move_hints.insert(legal_move.get_dest());
                self.redraw_squares.insert(legal_move.get_dest());
            }
        }
    }

    fn on_user_move(&mut self, src: SQ, dest: SQ) {
        self.selected_square = None;
        self.finger_down_square = None;
        self.clear_move_hints();
        let bit_move = BitMove::make(0, src, dest);
        if let Err(e) = self.try_move(bit_move) {
            println!("Invalid move: {}", e);
        } else {
            self.redraw_squares.insert(dest.clone());
            // Task bot to do a move
            if self.game_mode != GameMode::PvP {
                self.bot_job
                    .send(Some((self.board.clone(), self.game_mode.clone() as u16)))
                    .ok();
                self.ignore_user_moves = true;
            }
        }
    }
}

impl Drop for GameScene {
    fn drop(&mut self) {
        // Signal bot thread to terminate
        self.bot_job.send(None).ok();
        println!("Bot thread should terminate");
    }
}

impl Scene for GameScene {
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
                    }
                    multitouch::MultitouchEvent::Release { finger } => {
                        if self.back_button_hitbox.is_some()
                            && Canvas::is_hitting(finger.pos, self.back_button_hitbox.unwrap())
                        {
                            self.back_button_pressed = true;
                        }
                        if self.undo_button_hitbox.is_some()
                            && Canvas::is_hitting(finger.pos, self.undo_button_hitbox.unwrap())
                        {
                            if self.game_mode == GameMode::PvP {
                                if self.board.moves_played() >= 1 {
                                    self.board.undo_move();
                                    self.redraw_all_squares = true;
                                }
                            } else if !self.ignore_user_moves && self.board.moves_played() >= 2 {
                                self.board.undo_move(); // Bots move
                                self.board.undo_move(); // Players move
                                self.redraw_all_squares = true;
                            }
                        }
                        if self.full_refresh_button_hitbox.is_some()
                            && Canvas::is_hitting(
                                finger.pos,
                                self.full_refresh_button_hitbox.unwrap(),
                            )
                        {
                            self.force_full_refresh = Some(SystemTime::now());
                        } else if !self.ignore_user_moves {
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
                                                // Move
                                                self.redraw_squares.insert(new_square.clone());
                                                self.on_user_move(last_selected_square, new_square);
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
                                                self.selected_square = Some(new_square);
                                                self.redraw_squares.insert(new_square.clone());
                                                self.set_move_hints(new_square);
                                            }
                                        };
                                    }
                                }
                            }
                        }
                        self.finger_down_square = None;
                    }
                    _ => {}
                }
            }
            _ => {}
        };
    }

    fn draw(&mut self, canvas: &mut Canvas) {
        if self.first_draw {
            // First frame
            canvas.clear();

            self.back_button_hitbox = Some(canvas.draw_button(
                Point2 {
                    x: Some(50),
                    y: Some(90),
                },
                "Main Menu",
                75.0,
                10,
                20,
            ));
            self.undo_button_hitbox = Some(canvas.draw_button(
                Point2 {
                    x: Some(
                        self.back_button_hitbox.unwrap().left as i32
                            + self.back_button_hitbox.unwrap().width as i32
                            + 50,
                    ),
                    y: Some(90),
                },
                "Undo",
                75.0,
                10,
                20,
            ));
            self.full_refresh_button_hitbox = Some(canvas.draw_button(
                Point2 {
                    x: Some(
                        self.undo_button_hitbox.unwrap().left as i32
                            + self.undo_button_hitbox.unwrap().width as i32
                            + 50,
                    ),
                    y: Some(90),
                },
                "Refresh Screen",
                75.0,
                10,
                20,
            ));
            self.redraw_all_squares = true;
            self.draw_board(canvas);
            canvas.update_full();
            self.first_draw = false;
            // Refresh again after 500ms
            self.force_full_refresh = Some(SystemTime::now() + Duration::from_millis(500));
        }

        // Await bot move
        if let Ok(bot_bit_move) = self.bot_move.try_recv() {
            if let Err(e) = self.try_move(bot_bit_move) {
                panic!("Invalid move by bot: {}", e);
            }
            self.redraw_squares.insert(bot_bit_move.get_src());
            self.redraw_squares.insert(bot_bit_move.get_dest());
            println!("Bot moved");
            self.ignore_user_moves = false;
        }

        if self.redraw_all_squares || self.redraw_squares.len() > 0 {
            self.draw_board(canvas);
            self.redraw_all_squares = false;
            canvas.update_partial(&mxcfb_rect {
                left: 0,
                top: 0,
                width: DISPLAYWIDTH.into(),
                height: DISPLAYHEIGHT.into(),
            });
        }

        if self.force_full_refresh.is_some() && self.force_full_refresh.unwrap() < SystemTime::now()
        {
            canvas.update_full();
            self.force_full_refresh = None;
        }

        // Check periodicially for checkmate.
        // The function pleco::Board::checkmate() is supposed to be compuationally
        // expensive. I measured 2-3us at the beginning on the rM1 but who knows.
        // This more a compromize between development speed and correctness.
        let checkmate_check_elapsed = self.last_checkmate_check.elapsed();
        if checkmate_check_elapsed.is_ok()
            && checkmate_check_elapsed.unwrap() > Duration::from_millis(3000)
            && self.board.checkmate()
        {
            self.last_checkmate_check = SystemTime::now();
            canvas.draw_text(
                Point2 {
                    x: None,
                    y: Some(DISPLAYHEIGHT as i32 - 100),
                },
                "Checkmate!",
                100.0,
            );
            unsafe {
                self.board.apply_null_move(); // Allow the other to go on
            }
        }
    }
}
