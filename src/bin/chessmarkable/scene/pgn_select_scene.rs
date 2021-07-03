use super::Scene;
use crate::canvas::*;
use crate::savestates::Savestates;
use crate::pgns::*;
use libremarkable::input::{multitouch::MultitouchEvent, InputEvent};
use anyhow::Error;

const REPLAYS_PER_PAGE: u32 = 5;

pub struct PgnSelectScene {
    drawn: bool,
    pgn_vec: Vec<Pgn>,
    pub pvp_piece_rotation_enabled: bool,

    pub current_page_number: u32,
    pub total_pages: u32,

    pgn_1_button_hitbox: Option<mxcfb_rect>,
    pub pgn_1_button_pressed: bool,
    pgn_2_button_hitbox: Option<mxcfb_rect>,
    pub pgn_2_button_pressed: bool,
    pgn_3_button_hitbox: Option<mxcfb_rect>,
    pub pgn_3_button_pressed: bool,
    pgn_4_button_hitbox: Option<mxcfb_rect>,
    pub pgn_4_button_pressed: bool,
    pgn_5_button_hitbox: Option<mxcfb_rect>,
    pub pgn_5_button_pressed: bool,
    pgn_6_button_hitbox: Option<mxcfb_rect>,
    pub pgn_6_button_pressed: bool,

    next_page_button_hitbox: Option<mxcfb_rect>,
    pub next_page_button_pressed: bool,
    prev_page_button_hitbox: Option<mxcfb_rect>,
    pub prev_page_button_pressed: bool,
    back_button_hitbox: Option<mxcfb_rect>,
    pub back_button_pressed: bool,

    indicate_loading: bool,
}

impl PgnSelectScene {
    pub fn new(
        pvp_piece_rotation_enabled: bool
    ) -> Self {
        Self {
            pgn_vec: Vec::new(),
            drawn: false,
            pvp_piece_rotation_enabled,
            current_page_number: 0,
            total_pages: match crate::pgns::total_number_of_pgn() {
                0 => 1,
                num => (num as f64 / REPLAYS_PER_PAGE as f64).ceil() as u32
            },
            pgn_1_button_hitbox: None,
            pgn_1_button_pressed: false,
            pgn_2_button_hitbox: None,
            pgn_2_button_pressed: false,
            pgn_3_button_hitbox: None,
            pgn_3_button_pressed: false,
            pgn_4_button_hitbox: None,
            pgn_4_button_pressed: false,
            pgn_5_button_hitbox: None,
            pgn_5_button_pressed: false,
            pgn_6_button_hitbox: None,
            pgn_6_button_pressed: false,
            next_page_button_hitbox: None,
            prev_page_button_hitbox: None,
            back_button_hitbox: None,
            back_button_pressed: false,
            indicate_loading: true,
        }
    }

    fn indicate_loading(&self, canvas: &mut Canvas) {
        let rect = canvas.draw_text(
            Point2 {
                x: None,
                y: Some(350),
            },
            "Loading pgns..",
            50.0,
        );
        canvas.update_partial(&rect);
    }
}

impl Scene for PgnSelectScene {
    fn draw(&mut self, canvas: &mut Canvas) {
        if self.indicate_loading {
            self.indicate_loading(canvas);
            self.pgn_vec = match crate::pgns::read(((self.current_page_number * REPLAYS_PER_PAGE) as usize), ((self.current_page_number + 1) * REPLAYS_PER_PAGE - 1) as usize) {
                Ok(vec) => vec,
                Err(_) => Vec::new()
            };
            self.indicate_loading = false;
            return;
        }

        if self.drawn {
            return;
        }
        self.drawn = true;


        canvas.clear();
        canvas.draw_text(
            Point2 {
                x: None,
                y: Some(150),
            },
            "chessMarkable",
            150.0,
        );
        if self.pgn_vec.len() == 0 {
            canvas.draw_text(
                Point2 {
                    x: None,
                    y: Some(700),
                },
                "No PGNs found in PGN directory",
                75.0,
            );
        } else {
            canvas.draw_text(
                Point2 {
                    x: None,
                    y: Some(300),
                },
                "Choose PGN:",
                75.0,
            );
            self.pgn_1_button_hitbox = draw_replay_for_pgn(canvas, self.pgn_vec.get(0), 500, 37.5, 75);
            self.pgn_2_button_hitbox = draw_replay_for_pgn(canvas, self.pgn_vec.get(1), 675, 37.5, 75);
            self.pgn_3_button_hitbox = draw_replay_for_pgn(canvas, self.pgn_vec.get(2), 850, 37.5, 75);
            self.pgn_4_button_hitbox = draw_replay_for_pgn(canvas, self.pgn_vec.get(3), 1025, 37.5, 75);
            self.pgn_5_button_hitbox = draw_replay_for_pgn(canvas, self.pgn_vec.get(4), 1200, 37.5, 75);
            self.pgn_6_button_hitbox = draw_replay_for_pgn(canvas, self.pgn_vec.get(5), 1375, 37.5, 75);

            self.back_button_hitbox = Some(canvas.draw_button(
                Point2 {
                    x: None,
                    y: Some(1700),
                },
                "Main Menu",
                75.0,
                25,
                50,
            ));

            self.next_page_button_hitbox = if self.current_page_number + 1 < self.total_pages {
                Some(canvas.draw_button(
                    Point2 {
                        x: Some((self.back_button_hitbox.unwrap().left + self.back_button_hitbox.unwrap().width + 150) as i32),
                        y: Some(1700),
                    },
                    ">",
                    125.0,
                    50,
                    50,
                ))
            } else { None };
            self.prev_page_button_hitbox = if self.current_page_number != 0 {
                Some(canvas.draw_button(
                    Point2 {
                        x: Some((self.back_button_hitbox.unwrap().left - 200) as i32),
                        y: Some(1700),
                    },
                    "<",
                    125.0,
                    50,
                    50,
                ))
            } else { None };
        };

        canvas.update_full();
    }

    fn on_input(&mut self, event: InputEvent) {
        if let InputEvent::MultitouchEvent { event } = event {
            if let MultitouchEvent::Release { finger, .. } = event {
                let position = finger.pos;
                if self.back_button_hitbox.is_some()
                    && Canvas::is_hitting(position, self.back_button_hitbox.unwrap())
                {
                    self.back_button_pressed = true;
                } else if self.next_page_button_hitbox.is_some()
                    && Canvas::is_hitting(position, self.next_page_button_hitbox.unwrap())
                {
                    self.go_to_next_page()
                } else if self.prev_page_button_hitbox.is_some()
                    && Canvas::is_hitting(position, self.prev_page_button_hitbox.unwrap())
                {
                    self.go_to_prev_page()
                }
            }
        }
    }
}

impl PgnSelectScene {
    fn go_to_next_page(&mut self) {
        if self.current_page_number + 1 < self.total_pages {
            self.indicate_loading = true;
            self.drawn = false;
            self.current_page_number = self.current_page_number + 1;
        }
    }

    fn go_to_prev_page(&mut self) {
        if self.current_page_number != 0 {
            self.indicate_loading = true;
            self.drawn = false;
            self.current_page_number = self.current_page_number - 1;
        }
    }
}

fn construct_text_for_replay(pgn_ref: &Pgn) -> String {
    let mut replay_text = pgn_ref.white_player_name.to_owned().unwrap_or("White".parse().unwrap());
    replay_text.push_str(" vs ");
    replay_text.push_str(pgn_ref.black_player_name.as_ref().unwrap_or(&"Black".to_string()));
    replay_text.push_str(" at ");
    replay_text.push_str(pgn_ref.event.as_ref().unwrap_or(&"Event".to_string()));
    replay_text.push_str(" ");
    replay_text.push_str(pgn_ref.round.as_ref().unwrap_or(&"".to_string()));
    replay_text
}

fn draw_replay_for_pgn(canvas: &mut Canvas, maybe_pgn_ref: Option<&Pgn>, y_pos: i32, font_size: f32, vgap: u32) -> Option<mxcfb_rect> {
    match maybe_pgn_ref {
        Some(pgn_ref) => Some(canvas.draw_box_button(Some(y_pos), &*construct_text_for_replay(pgn_ref), font_size, vgap)),
        None => None
    }
}
