use super::Scene;
use crate::canvas::*;
use chess::{Color as PieceColor, File, Game, Piece, Rank, Square};
use fxhash::FxHashMap;
use libremarkable::image;
use libremarkable::input::{gpio, multitouch, multitouch::Finger, InputEvent};
use pleco::bot_prelude::*;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;
use std::time::SystemTime;

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
}

fn to_square(x: usize, y: usize) -> Square {
    Square::make_square(Rank::from_index(y), File::from_index(x))
}

fn to_chess_move(bit_move: pleco::BitMove) -> chess::ChessMove {
    let promo = if bit_move.is_promo() {
        Some(match bit_move.promo_piece() {
            pleco::PieceType::K => Piece::King,
            pleco::PieceType::Q => Piece::Queen,
            pleco::PieceType::B => Piece::Bishop,
            pleco::PieceType::R => Piece::Rook,
            pleco::PieceType::N => Piece::Knight,
            pleco::PieceType::P => Piece::Pawn,
            _ => panic!("Invalid promo piece!"),
        })
    } else {
        None
    };
    chess::ChessMove::new(
        to_square(bit_move.src_col() as usize, bit_move.src_row() as usize),
        to_square(bit_move.dest_col() as usize, bit_move.dest_row() as usize),
        promo,
    )
}

#[derive(Clone, Copy)]
pub enum Difficulty {
    Easy = 1,
    Normal = 2,
    Hard = 4,
    // Could go up to about 8-10 (depending on the algo) before getting too slow. But probably fairly unbeatable then.
}

pub struct GameScene {
    game: Game,
    bot_difficulty: Difficulty,
    first_draw: bool,
    ignore_user_moves: bool,
    bot_job: Sender<Option<(chess::Board, u16)>>,
    bot_move: Receiver<chess::ChessMove>,
    back_button_hitbox: Option<mxcfb_rect>,
    square_size: u32,
    piece_hitboxes: Vec<Vec<mxcfb_rect>>,
    redraw_squares: Vec<Square>,
    selected_square: Option<Square>,
    /// Resized to fit selected_square
    img_pieces: FxHashMap<(Piece, PieceColor), image::DynamicImage>,
    img_piece_selected: image::DynamicImage,
    pub back_button_pressed: bool,
}

impl GameScene {
    pub fn new(bot_difficulty: Difficulty) -> Self {
        let square_size = DISPLAYWIDTH as u32 / 8;

        // Calculate hitboxes
        let mut piece_hitboxes = Vec::new();
        for x in 0..8 {
            let mut y_axis = Vec::new();
            for y in 0..8 {
                y_axis.push(mxcfb_rect {
                    left: ((DISPLAYWIDTH as u32 - square_size * 8) / 2) + square_size * x,
                    top: ((DISPLAYHEIGHT as u32 - square_size * 8) / 2) + square_size * y,
                    width: square_size,
                    height: square_size,
                });
            }
            piece_hitboxes.push(y_axis);
        }

        // Create resized images
        let mut img_pieces: FxHashMap<(Piece, PieceColor), image::DynamicImage> =
            Default::default();
        for piece_color in [PieceColor::Black, PieceColor::White].iter() {
            for piece in [
                Piece::King,
                Piece::Queen,
                Piece::Bishop,
                Piece::Rook,
                Piece::Knight,
                Piece::Pawn,
            ]
            .iter()
            {
                img_pieces.insert(
                    (*piece, *piece_color),
                    Self::get_orig_pice_img(piece, piece_color).resize(
                        square_size,
                        square_size,
                        image::FilterType::Lanczos3,
                    ),
                );
            }
        }
        let img_piece_selected =
            IMG_PIECE_SELECTED.resize(square_size, square_size, image::FilterType::Lanczos3);

        let (bot_job_tx, bot_job_rx) = channel();
        let (bot_move_tx, bot_move_rx) = channel();
        Self::spawn_bot_thread(bot_job_rx, bot_move_tx);

        Self {
            game: chess::Game::new(),
            first_draw: true,
            bot_job: bot_job_tx,
            bot_move: bot_move_rx,
            ignore_user_moves: false,
            bot_difficulty,
            piece_hitboxes,
            square_size,
            selected_square: None,
            img_pieces,
            img_piece_selected,
            redraw_squares: Vec::new(),
            back_button_hitbox: None,
            back_button_pressed: false,
        }
    }

    fn get_orig_pice_img(piece: &Piece, color: &PieceColor) -> &'static image::DynamicImage {
        match color {
            PieceColor::Black => match piece {
                Piece::King => &IMG_KING_BLACK,
                Piece::Queen => &IMG_QUEEN_BLACK,
                Piece::Bishop => &IMG_BISHOP_BLACK,
                Piece::Rook => &IMG_ROOK_BLACK,
                Piece::Knight => &IMG_KNIGHT_BLACK,
                Piece::Pawn => &IMG_PAWN_BLACK,
            },
            PieceColor::White => match piece {
                Piece::King => &IMG_KING_WHITE,
                Piece::Queen => &IMG_QUEEN_WHITE,
                Piece::Bishop => &IMG_BISHOP_WHITE,
                Piece::Rook => &IMG_ROOK_WHITE,
                Piece::Knight => &IMG_KNIGHT_WHITE,
                Piece::Pawn => &IMG_PAWN_WHITE,
            },
        }
    }

    fn draw_board(&mut self, canvas: &mut Canvas, draw_all: bool) {
        for x in 0..8 {
            for y in 0..8 {
                let square = to_square(x, 7 - y); // Flip board so white is at the bottom
                if !draw_all && !self.redraw_squares.contains(&square) {
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
                if let Some(piece) = self.game.current_position().piece_on(square) {
                    let piece_color = self.game.current_position().color_on(square).unwrap();

                    let piece_img = self
                        .img_pieces
                        .get(&(piece, piece_color))
                        .expect("Failed to find resized piece img!");
                    canvas.draw_image(bounds.top_left().cast().unwrap(), &piece_img, true);
                }

                // Overlay image if square is selected
                if self.selected_square.is_some() && self.selected_square.unwrap() == square {
                    canvas.draw_image(
                        bounds.top_left().cast().unwrap(),
                        &self.img_piece_selected,
                        true,
                    );
                }
            }
        }

        self.redraw_squares.clear();
    }

    fn spawn_bot_thread(
        job: Receiver<Option<(chess::Board, u16)>>,
        job_result: Sender<chess::ChessMove>,
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
                job_result.send(Self::do_bot_move(board, depth)).unwrap();
            })
            .unwrap()
    }

    fn do_bot_move(board: chess::Board, depth: u16) -> chess::ChessMove {
        println!("Bot is working...");
        let start = SystemTime::now();
        let pleco_board = pleco::Board::from_fen(&board.to_string())
            .expect("Failed to copy default board to pleco");
        let bot_bit_move = JamboreeSearcher::best_move(pleco_board, depth);
        println!("Bot took {}ms", start.elapsed().unwrap().as_millis());
        to_chess_move(bot_bit_move)
    }
}

impl Drop for GameScene {
    fn drop(&mut self) {
        // Signal bot thread to terminate
        self.bot_job.send(None).unwrap();
        println!("Bot thread should terminate");
    }
}

impl Scene for GameScene {
    fn on_input(&mut self, event: InputEvent) {
        match event {
            InputEvent::GPIO { event } => {}
            InputEvent::MultitouchEvent { event } => {
                // Taps and buttons
                match event {
                    multitouch::MultitouchEvent::Press { finger } => {
                        if let Some(back_button_hitbox) = self.back_button_hitbox {
                            if Canvas::is_hitting(finger.pos, back_button_hitbox) {
                                self.back_button_pressed = true;
                            } else if !self.ignore_user_moves {
                                for x in 0..8 {
                                    for y in 0..8 {
                                        if Canvas::is_hitting(finger.pos, self.piece_hitboxes[x][y])
                                        {
                                            let new_square = to_square(x, 7 - y);
                                            if let Some(last_selected_square) = self.selected_square
                                            {
                                                self.redraw_squares
                                                    .push(last_selected_square.clone());

                                                if last_selected_square == new_square {
                                                    // Cancel move
                                                    self.selected_square = None;
                                                } else {
                                                    // Move
                                                    self.selected_square = None;
                                                    let chess_move = chess::ChessMove::new(
                                                        last_selected_square,
                                                        new_square,
                                                        None,
                                                    );
                                                    if self.game.make_move(chess_move) {
                                                        self.redraw_squares
                                                            .push(new_square.clone());
                                                        // Task bot to do a move
                                                        self.bot_job
                                                            .send(Some((
                                                                self.game
                                                                    .current_position()
                                                                    .clone(),
                                                                self.bot_difficulty.clone() as u16,
                                                            )))
                                                            .unwrap();
                                                        self.ignore_user_moves = true;
                                                    } else {
                                                        println!("Invalid move");
                                                    }
                                                }
                                            } else {
                                                self.selected_square = Some(new_square);
                                                self.redraw_squares.push(new_square.clone());
                                            };
                                        }
                                    }
                                }
                            }
                        }
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
                    y: Some(75),
                },
                "Main Menu",
                50.0,
                10,
                20,
            ));
            self.draw_board(canvas, true);
            canvas.update_full();
            self.first_draw = false;
        }

        // Await bot move
        if let Ok(bot_chess_move) = self.bot_move.try_recv() {
            if self.game.make_move(bot_chess_move) {
                self.redraw_squares.push(bot_chess_move.get_source());
                self.redraw_squares.push(bot_chess_move.get_dest());
            } else {
                panic!("The Chess-Bot (pleco lib) made unexpected invalid move according to the \"chess\" lib.");
            }
            println!("Bot decided");
            self.ignore_user_moves = false;
        }

        if self.redraw_squares.len() > 0 {
            self.draw_board(canvas, false);
            canvas.update_partial(&mxcfb_rect {
                left: 0,
                top: 0,
                width: DISPLAYWIDTH.into(),
                height: DISPLAYHEIGHT.into(),
            });
        }
    }
}
