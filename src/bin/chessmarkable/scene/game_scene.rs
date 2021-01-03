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
use tokio::runtime;
use tokio::sync::mpsc::{channel, Receiver, Sender};

lazy_static! {
    // Underlays / Background layers
    static ref IMG_PIECE_MOVED_FROM: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../../../res/piece-moved-from.png"))
            .expect("Failed to load resource as image!");
    static ref IMG_PIECE_MOVED_TO: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../../../res/piece-moved-to.png"))
            .expect("Failed to load resource as image!");

    // Black set
    static ref IMG_KING_BLACK: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../../../res/king-black.png"))
            .expect("Failed to load resource as image!");
    static ref IMG_QUEEN_BLACK: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../../../res/queen-black.png"))
            .expect("Failed to load resource as image!");
    static ref IMG_BISHOP_BLACK: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../../../res/bishop-black.png"))
            .expect("Failed to load resource as image!");
    static ref IMG_ROOK_BLACK: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../../../res/rook-black.png"))
            .expect("Failed to load resource as image!");
    static ref IMG_KNIGHT_BLACK: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../../../res/knight-black.png"))
            .expect("Failed to load resource as image!");
    static ref IMG_PAWN_BLACK: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../../../res/pawn-black.png"))
            .expect("Failed to load resource as image!");

    // White set
    static ref IMG_KING_WHITE: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../../../res/king-white.png"))
            .expect("Failed to load resource as image!");
    static ref IMG_QUEEN_WHITE: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../../../res/queen-white.png"))
            .expect("Failed to load resource as image!");
    static ref IMG_BISHOP_WHITE: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../../../res/bishop-white.png"))
            .expect("Failed to load resource as image!");
    static ref IMG_ROOK_WHITE: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../../../res/rook-white.png"))
            .expect("Failed to load resource as image!");
    static ref IMG_KNIGHT_WHITE: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../../../res/knight-white.png"))
            .expect("Failed to load resource as image!");
    static ref IMG_PAWN_WHITE: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../../../res/pawn-white.png"))
            .expect("Failed to load resource as image!");

    // Overlays
    static ref IMG_PIECE_SELECTED: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../../../res/piece-selected.png"))
            .expect("Failed to load resource as image!");
    static ref IMG_PIECE_MOVEHINT: image::DynamicImage =
        image::load_from_memory(include_bytes!("../../../../res/piece-move-hint.png"))
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

#[inline]
fn to_square(x: usize, y: usize) -> Square {
    Square::new(x, y).expect("to_square() failed")
}

enum GameBottomInfo {
    GameEnded(String),
    Info(String),
    Error(String),
}

#[derive(Clone, Copy, PartialEq)]
pub enum GameMode {
    PvP = 0,
    EasyBot = 2,
    NormalBot = 4,
    HardBot = 6,
    // Could go up to about 8-10 (depending on the algo) before getting too slow. But probably fairly unbeatable then.
}

pub struct GameScene {
    board: Board,
    /// May be above zero when a fen was imported. Used to prevent panic on undo.
    game_mode: GameMode,
    first_draw: bool,
    back_button_hitbox: Option<mxcfb_rect>,
    undo_button_hitbox: Option<mxcfb_rect>,
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
    /// Remember a press to decide whether to show options or do a move at once
    finger_down_square: Option<Square>,
    pub back_button_pressed: bool,
    /// Do a full screen refresh on next draw
    force_full_refresh: Option<SystemTime>,
    draw_game_bottom_info: Option<GameBottomInfo>,
    draw_game_bottom_info_delay_until: Option<SystemTime>,
    draw_game_bottom_info_last_rect: Option<mxcfb_rect>,
    draw_game_bottom_info_clear_at: Option<SystemTime>,
    is_game_over: bool,
    white_request_sender: Option<Sender<ChessRequest>>,
    black_request_sender: Option<Sender<ChessRequest>>,
    white_update_receiver: Option<Receiver<ChessUpdate>>,
    black_update_receiver: Option<Receiver<ChessUpdate>>,
    possible_moves: Vec<(Square, Square)>,
    runtime: runtime::Runtime,
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
                Self::get_orig_piece_img(piece).resize(
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

        // Create game (will run on as many theads as the cpu has cores)
        let mut runtime = runtime::Builder::new()
            .thread_name("tokio_game_scene")
            .threaded_scheduler()
            //.max_threads(2)
            .build()
            .expect("Failed to create tokio runtime");

        let mut white_request_sender: Option<Sender<ChessRequest>> = None;
        let mut black_request_sender: Option<Sender<ChessRequest>> = None;
        let mut white_update_receiver: Option<Receiver<ChessUpdate>> = None;
        let mut black_update_receiver: Option<Receiver<ChessUpdate>> = None;

        if game_mode == GameMode::PvP {
            let (white_update_tx, white_update_rx) = channel::<ChessUpdate>(256);
            let (white_request_tx, white_request_rx) = channel::<ChessRequest>(256);

            let (black_update_tx, black_update_rx) = channel::<ChessUpdate>(256);
            let (black_request_tx, black_request_rx) = channel::<ChessRequest>(256);

            runtime.spawn(create_game(
                (white_update_tx, white_request_rx),
                (black_update_tx, black_request_rx),
                stubbed_spectator(),
                ChessConfig {
                    starting_fen: CLI_OPTS.intial_fen.clone(),
                    can_black_undo: true,
                    can_white_undo: true,
                    allow_undo_after_loose: true,
                },
            ));

            white_request_sender = Some(white_request_tx);
            black_request_sender = Some(black_request_tx);
            white_update_receiver = Some(white_update_rx);
            black_update_receiver = Some(black_update_rx);
        //Self::spawn_bot_thread(bot_job_rx, bot_move_tx); // TODO
        } else {
            let (white_update_tx, white_update_rx) = channel::<ChessUpdate>(256);
            let (white_request_tx, white_request_rx) = channel::<ChessRequest>(256);

            // Use multithreaded algo when not rM 1
            let bot = if libremarkable::device::CURRENT_DEVICE.model
                == libremarkable::device::Model::Gen1
            {
                debug!("The Bot will use the AlphaBeta algorithm (singlethreaded)");
                runtime
                .block_on(create_bot::<AlphaBetaSearcher>(
                    Player::Black,
                    game_mode as u16,
                    Duration::from_millis(CLI_OPTS.bot_reaction_delay.into()),
                ))
                .expect("Failed to initialize bot task")
            } else {
                debug!("The Bot will use the Jamboree algorithm (multithreaded)");
                runtime
                .block_on(create_bot::<JamboreeSearcher>(
                    Player::Black,
                    game_mode as u16,
                    Duration::from_millis(CLI_OPTS.bot_reaction_delay.into()),
                ))
                .expect("Failed to initialize bot task")
            };

            runtime.spawn(create_game(
                (white_update_tx, white_request_rx),
                bot,
                stubbed_spectator(),
                ChessConfig {
                    starting_fen: CLI_OPTS.intial_fen.clone(),
                    can_black_undo: false,
                    can_white_undo: true,
                    allow_undo_after_loose: true,
                },
            ));

            white_request_sender = Some(white_request_tx);
            white_update_receiver = Some(white_update_rx);
        }

        Self {
            board: Board::default(), // Temporary default (usually stays that but will change when having a custom fen)
            first_draw: true,
            game_mode,
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
            full_refresh_button_hitbox: None,
            back_button_pressed: false,
            force_full_refresh: None,
            draw_game_bottom_info_delay_until: Some(SystemTime::now() + Duration::from_secs(2)),
            draw_game_bottom_info: Some(GameBottomInfo::Info("White starts".to_owned())),
            draw_game_bottom_info_last_rect: None,
            draw_game_bottom_info_clear_at: None,
            is_game_over: false,
            runtime,
            black_request_sender,
            black_update_receiver,
            white_request_sender,
            white_update_receiver,
            possible_moves: vec![],
        }
    }

    fn get_orig_piece_img(piece: &Piece) -> &'static image::DynamicImage {
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

    fn handle_outcome(&mut self, outcome: Option<ChessOutcome>) {
        debug!("Outcome: {:?}", outcome);

        if let Some(outcome) = outcome {
            if let ChessOutcome::Checkmate { winner } = outcome {
                if self.is_game_over {
                    return; // This is not new
                }

                let looser = match winner {
                    Player::Black => "White",
                    Player::White => "Black",
                };
                self.show_bottom_game_info(
                    GameBottomInfo::GameEnded(format!("{} is checkmated!", looser)),
                    None,
                    None,
                );
                self.is_game_over = true;
            } else if let ChessOutcome::Stalemate = outcome {
                if self.is_game_over {
                    return; // This is not new
                }

                self.show_bottom_game_info(
                    GameBottomInfo::GameEnded("Stalemate!".to_owned()),
                    None,
                    None,
                );
                self.is_game_over = true;
            }
        } else if self.is_game_over {
            // Probably undone a move. Is not gameover anymore
            self.is_game_over = false;
        }
    }

    fn clear_bottom_game_info(&mut self) {
        if self.draw_game_bottom_info_last_rect.is_some() {
            self.draw_game_bottom_info_clear_at = Some(SystemTime::now());
        }
    }

    /// Depending on the durations of show_after and clear_after,
    /// previous text can be removed with a delay before displaying
    /// a new one or the new text can be removed after some time.
    fn show_bottom_game_info(
        &mut self,
        info: GameBottomInfo,
        show_after: Option<Duration>,
        clear_after: Option<Duration>,
    ) {
        self.draw_game_bottom_info_delay_until = Some(
            show_after
                .and_then(|delay| Some(SystemTime::now() + delay))
                .unwrap_or(SystemTime::now()),
        );
        self.draw_game_bottom_info = Some(info);
        self.draw_game_bottom_info_clear_at =
            clear_after.and_then(|delay| Some(SystemTime::now() + delay));
    }

    fn draw_board(&mut self, canvas: &mut Canvas) -> Vec<mxcfb_rect> {
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
        self.selected_square = None;
        self.finger_down_square = None;
        self.clear_move_hints();
        self.clear_last_moved_hints();

        let sender = match self.board.turn().into() {
            Player::Black => self.black_request_sender.clone(),
            Player::White => self.white_request_sender.clone(),
        };
        let other_player = self.board.turn().other_player();

        if sender.is_none() {
            self.show_bottom_game_info(
                GameBottomInfo::Error(format!("You can't move {}", self.board.turn())),
                None,
                Some(Duration::from_secs(10)),
            );
            return;
        }
        let mut sender = sender.unwrap();
        self.runtime.spawn(async move {
            sender
                .send(ChessRequest::MovePiece {
                    source: src,
                    destination: dest,
                })
                .await
                .ok();
        });

        if !self.is_local_user(other_player.into()) {
            self.show_bottom_game_info(
                GameBottomInfo::Info("Waiting on your opponent...".to_owned()),
                Some(Duration::from_millis(
                    (CLI_OPTS.bot_reaction_delay + 100) as u64,
                )),
                Some(Duration::from_millis(100)),
            );
        }
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

    /// A local user can tap on the tablet. Neither a bot nor a remotly
    /// connected player are that.
    fn is_local_user(&self, player: Player) -> bool {
        match player {
            Player::Black => self.black_request_sender.is_some(),
            Player::White => self.white_request_sender.is_some(),
        }
    }

    fn handle_updates(&mut self, player: Player, update_receiver: &mut Receiver<ChessUpdate>) {
        for update in update_receiver.try_recv() {
            //debug!("Got update for {}: {:#?}", player, update);
            match update {
                ChessUpdate::Board { ref fen } => self.update_board(fen),
                ChessUpdate::GenericErrorResponse { message } => {
                    warn!(
                        "Received a GenericErrorResponse for {}: {}",
                        player, message
                    );
                    self.show_bottom_game_info(
                        GameBottomInfo::Error(format!("[Error] {}", message)),
                        None,
                        None,
                    );
                }
                ChessUpdate::PossibleMoves { possible_moves } => {
                    self.possible_moves = possible_moves;

                    // In case the user already selected a figure but didn't
                    // receive the possible moves yet, they will get displayed now.
                    if let Some(selected_square) = self.selected_square {
                        self.set_move_hints(selected_square);
                    }
                }
                ChessUpdate::Outcome { outcome } => self.handle_outcome(outcome),
                ChessUpdate::MovePieceFailedResponse { fen, message } => {
                    self.update_board(&fen);
                    self.show_bottom_game_info(
                        GameBottomInfo::Error(format!("{}", message)),
                        None,
                        Some(Duration::from_secs(10)),
                    )
                }
                ChessUpdate::PlayerMovedAPiece {
                    player,
                    moved_piece_source,
                    moved_piece_destination,
                } => {
                    let is_local_user = self.is_local_user(player);
                    if !is_local_user {
                        // This player is not controlled by this frontend.
                        // Either a bot or an remote opponent whoses move
                        // should get marked.
                        self.last_move_from = Some(moved_piece_source);
                        self.last_move_to = Some(moved_piece_destination);
                        self.redraw_squares.insert(moved_piece_source);
                        self.redraw_squares.insert(moved_piece_destination);
                    }
                    info!("{} (is_local_user: {}) made a move", player, is_local_user);
                }
                ChessUpdate::PlayerSwitch { player, ref fen } => {
                    self.update_board(fen);
                    // TODO: Better message depending on game mode
                    if !self.is_game_over {
                        let message = if !self.is_local_user(player) {
                            None
                        } else {
                            if !self.is_local_user(player.other_player()) {
                                Some("It's your turn.".to_owned())
                            } else {
                                Some(format!("It's {}'s turn.", player))
                            }
                        };

                        if let Some(message) = message {
                            self.show_bottom_game_info(GameBottomInfo::Info(message), None, None);
                        }
                    }
                }
                ChessUpdate::MovesUndone { who, moves } => {
                    self.show_bottom_game_info(
                        GameBottomInfo::Info(format!("{} undid {} move(s).", who, moves)),
                        None,
                        Some(Duration::from_secs(3)),
                    );
                    self.clear_last_moved_hints();
                }
                ChessUpdate::UndoMovesFailedResponse { message } => self.show_bottom_game_info(
                    GameBottomInfo::Error(format!("Undo failed: {}", message)),
                    None,
                    Some(Duration::from_secs(10)),
                ),
                ChessUpdate::CurrentTotalMovesReponse { .. } => {}
            }
        }
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
                            let undo_count: u16 = if self.game_mode == GameMode::PvP {
                                1
                            } else {
                                if let Player::Black = self.board.turn().into() {
                                    1
                                } else {
                                    2
                                }
                            };
                            let sender = if self.is_game_over {
                                // Find any player to send the event on
                                if let Some(ref sender) = self.black_request_sender {
                                    Some(sender.clone())
                                } else {
                                    if let Some(ref sender) = self.white_request_sender {
                                        Some(sender.clone())
                                    } else {
                                        None
                                    }
                                }
                            } else {
                                // Only undo when own turn
                                match self.board.turn().into() {
                                    Player::Black => self.black_request_sender.clone(),
                                    Player::White => self.white_request_sender.clone(),
                                }
                            };
                            if sender.is_none() {
                                error!("Undo failed because it cant be sent (not any local players turn).");
                                self.show_bottom_game_info(
                                    GameBottomInfo::Info("You can't undo right now.".to_owned()),
                                    None,
                                    Some(Duration::from_secs(3)),
                                );
                            } else {
                                let mut sender = sender.unwrap();
                                self.runtime.spawn(async move {
                                    sender
                                        .send(ChessRequest::UndoMoves { moves: undo_count })
                                        .await
                                        .ok();
                                });
                            }
                        }
                        if self.full_refresh_button_hitbox.is_some()
                            && Canvas::is_hitting(
                                finger.pos,
                                self.full_refresh_button_hitbox.unwrap(),
                            )
                        {
                            self.force_full_refresh = Some(SystemTime::now());
                        } else if !self.is_game_over {
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
            self.force_full_refresh = Some(SystemTime::now() + Duration::from_millis(250));
        }

        // Handle received `ChessUpdate`s
        if self.white_update_receiver.is_some() {
            let mut update_receiver = self.white_update_receiver.take().unwrap();
            self.handle_updates(Player::White, &mut update_receiver);
            self.white_update_receiver = Some(update_receiver);
        }
        if self.black_update_receiver.is_some() {
            let mut update_receiver = self.black_update_receiver.take().unwrap();
            self.handle_updates(Player::Black, &mut update_receiver);
            self.black_update_receiver = Some(update_receiver);
        }

        // Apply bot move
        /*
        if let Ok(when, bot_bit_move) = self.bot_move.try_recv() {
            self.clear_last_moved_hints();
            // Wait till board got refresh with all changes until now
            // This is useful if the bot is set to and can react
            // almost instantaniously.
            self.draw_board(canvas)
                .iter()
                .map(|rect| canvas.update_partial(rect))
                .collect::<Vec<u32>>() // Prevent two closures using the canvas by buffering here
                .iter()
                .for_each(|marker| canvas.wait_for_update(*marker));

            // Add bot move to board
            if !bot_bit_move.is_null() {
                if let Err(e) = self.try_move(bot_bit_move, true) {
                    panic!("Invalid move by bot: {}", e);
                }

                // Add new moved hints
                self.last_move_from = Some(bot_bit_move.get_src());
                self.redraw_squares.insert(bot_bit_move.get_src());
                self.last_move_to = Some(bot_bit_move.get_dest());
                self.redraw_squares.insert(bot_bit_move.get_dest());
                debug!("Bot moved");
            } else {
                debug!("Bot didn't want to move")
                // A bit below will be checked for proper ending in this case
            }
            self.ignore_user_moves = false;
        }*/

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

        // I don't grasp these conditions anymore as well. I probably
        // coded too long because I basicially brute forced many of these).
        // TODO: Write it as a single enum to make better sense of all this.
        let has_new_bottom_info = self.draw_game_bottom_info.is_some()
            && self.draw_game_bottom_info_delay_until.is_some()
            && self.draw_game_bottom_info_delay_until.unwrap() <= SystemTime::now();
        // Clear previous text when changed or expired
        if has_new_bottom_info
            || (self.draw_game_bottom_info_last_rect.is_some()
                && self.draw_game_bottom_info_clear_at.is_some()
                && self.draw_game_bottom_info_clear_at.unwrap() <= SystemTime::now())
        {
            // Clear any previous text
            if let Some(ref last_rect) = self.draw_game_bottom_info_last_rect {
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
                self.draw_game_bottom_info_last_rect = None;
            }
        }

        // Draw a requested text once
        if has_new_bottom_info {
            if let Some(ref game_bottom_info) = self.draw_game_bottom_info {
                // Old text was cleared above already

                let rect = match game_bottom_info {
                    GameBottomInfo::GameEnded(ref short_message) => canvas.draw_text(
                        Point2 {
                            x: None,
                            y: Some(DISPLAYHEIGHT as i32 - 100),
                        },
                        short_message,
                        100.0,
                    ),
                    GameBottomInfo::Info(ref message) => canvas.draw_text(
                        Point2 {
                            x: None,
                            y: Some(DISPLAYHEIGHT as i32 - 20),
                        },
                        message,
                        50.0,
                    ),
                    GameBottomInfo::Error(ref message) => canvas.draw_text(
                        Point2 {
                            x: Some(5),
                            y: Some(DISPLAYHEIGHT as i32 - 10),
                        },
                        message,
                        35.0,
                    ),
                };
                canvas.update_partial(&rect);
                self.draw_game_bottom_info_last_rect = Some(rect);
                self.draw_game_bottom_info = None;
                if self.draw_game_bottom_info_clear_at.is_some()
                    && self.draw_game_bottom_info_clear_at.unwrap() <= SystemTime::now()
                {
                    self.draw_game_bottom_info_clear_at = None;
                }
            }
        }
    }
}
