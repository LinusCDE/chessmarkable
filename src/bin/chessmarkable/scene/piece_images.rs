use chessmarkable::game::Piece;
use libremarkable::image;

lazy_static! {

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
}

pub fn get_orig_piece_img(piece: &Piece) -> &'static image::DynamicImage {
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

