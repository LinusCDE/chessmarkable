use super::Scene;
use crate::canvas::*;
use chess::{Color as PieceColor, File, Game, Piece, Rank, Square};
use fxhash::FxHashMap;
use libremarkable::image;
use libremarkable::input::{gpio, multitouch, multitouch::Finger, InputEvent};

lazy_static! {
    // Black set
    static ref IMG_KING_BLACK: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../res/knight-black.png"))
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
        image::load_from_memory(include_bytes!("../../res/knight-white.png"))
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

pub struct GameScene {
    game: Game,
    first_draw: bool,
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
    pub fn new() -> Self {
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

        Self {
            game: chess::Game::new(),
            first_draw: true,
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
                let square = to_square(x, y);
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
                            } else {
                                for x in 0..8 {
                                    for y in 0..8 {
                                        if Canvas::is_hitting(finger.pos, self.piece_hitboxes[x][y])
                                        {
                                            let new_square = to_square(x, y);
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
