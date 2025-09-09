use super::Scene;
use crate::canvas::*;
use crate::pgns::*;
use libremarkable::input::{InputEvent, MultitouchEvent};
use anyhow::Error;
use chess_pgn_parser::{Game, read_games};
use std::fs::File;
use std::io::Read;
use regex::Regex;
use crate::REPLAYS_PER_PAGE;

const BOX_HEIGHT: i32 = 180;
const FIRST_BOX_Y_POS: i32 = 350;

const EVENT_TAG: &str = "Event";
const WHITE_TAG: &str = "White";
const BLACK_TAG: &str = "Black";
const ROUND_TAG: &str = "Round";

pub struct PgnSelectScene {
    drawn: bool,
    pub game_vec: Vec<Game>,
    pgn_vec: Vec<Pgn>,
    pub selected_pgn: Option<Pgn>,
    selected_pgn_changed: bool,

    pub current_page_number: u32,
    total_pages: u32,

    button_1_hitbox: Option<mxcfb_rect>,
    pub button_1_pressed: bool,
    button_2_hitbox: Option<mxcfb_rect>,
    pub button_2_pressed: bool,
    button_3_hitbox: Option<mxcfb_rect>,
    pub button_3_pressed: bool,
    button_4_hitbox: Option<mxcfb_rect>,
    pub button_4_pressed: bool,
    button_5_hitbox: Option<mxcfb_rect>,
    pub button_5_pressed: bool,
    button_6_hitbox: Option<mxcfb_rect>,
    pub button_6_pressed: bool,

    next_page_button_hitbox: Option<mxcfb_rect>,
    prev_page_button_hitbox: Option<mxcfb_rect>,
    back_button_hitbox: Option<mxcfb_rect>,
    pub return_to_main_menu: bool,

    indicate_loading: bool,
}

impl PgnSelectScene {
    pub fn new(
        selected_pgn: Option<Pgn>,
    ) -> Self {
        let selected_pgn_changed = if selected_pgn.is_some() {true} else {false};
        Self {
            drawn: false,
            current_page_number: 0,
            total_pages: 1,
            button_1_hitbox: None,
            button_1_pressed: false,
            button_2_hitbox: None,
            button_2_pressed: false,
            button_3_hitbox: None,
            button_3_pressed: false,
            button_4_hitbox: None,
            button_4_pressed: false,
            button_5_hitbox: None,
            button_5_pressed: false,
            button_6_hitbox: None,
            next_page_button_hitbox: None,
            prev_page_button_hitbox: None,
            back_button_hitbox: None,
            return_to_main_menu: false,
            indicate_loading: false,
            selected_pgn_changed,
            selected_pgn,
            pgn_vec: vec![],
            game_vec: vec![],
            button_6_pressed: false
        }
    }

    fn indicate_loading(&self, canvas: &mut Canvas) {
        let rect = canvas.draw_text(
            Point2 {
                x: None,
                y: Some(350),
            },
            "Loading..",
            50.0,
        );
        canvas.update_partial(&rect);
    }
}

impl Scene for PgnSelectScene {
    fn draw(&mut self, canvas: &mut Canvas) {
        if self.indicate_loading {
            self.indicate_loading(canvas);
            return;
        }
        if self.drawn {
            return;
        }
        self.drawn = true;

        canvas.clear();
        let choose_pgn_mode = !self.selected_pgn.is_some();
        if choose_pgn_mode {
            self.total_pages = match crate::pgns::total_number_of_pgn() {
                0 => 1,
                num => (num as f64 / REPLAYS_PER_PAGE as f64).ceil() as u32
            };
            self.pgn_vec = match crate::pgns::read((self.current_page_number * REPLAYS_PER_PAGE) as usize, ((self.current_page_number + 1) * REPLAYS_PER_PAGE - 1) as usize) {
                Ok(vec) => vec,
                Err(_) => Vec::new()
            };
            let mut no_pgn_found_str = "No PGNs found, please add them to: ".to_string();
            no_pgn_found_str.push_str(&crate::CLI_OPTS.pgn_location.to_owned().into_os_string().into_string().unwrap());
            if self.pgn_vec.len() == 0 {
                canvas.draw_multi_line_text(
                    None,
                    700,
                    &no_pgn_found_str,
                    50,
                    2,
                    85.0,
                    0.8
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
            }
            self.button_1_hitbox = draw_button_for_pgn(canvas, self.pgn_vec.get(0), FIRST_BOX_Y_POS, 50.0);
            self.button_2_hitbox = draw_button_for_pgn(canvas, self.pgn_vec.get(1), FIRST_BOX_Y_POS + BOX_HEIGHT, 50.0);
            self.button_3_hitbox = draw_button_for_pgn(canvas, self.pgn_vec.get(2), FIRST_BOX_Y_POS + BOX_HEIGHT * 2, 50.0);
            self.button_4_hitbox = draw_button_for_pgn(canvas, self.pgn_vec.get(3), FIRST_BOX_Y_POS + BOX_HEIGHT * 3, 50.0);
            self.button_5_hitbox = draw_button_for_pgn(canvas, self.pgn_vec.get(4), FIRST_BOX_Y_POS + BOX_HEIGHT * 4, 50.0);
            self.button_6_hitbox = draw_button_for_pgn(canvas, self.pgn_vec.get(5), FIRST_BOX_Y_POS + BOX_HEIGHT * 5, 50.0);
        } else {
            if self.selected_pgn_changed {
                let mut file = File::open(self.selected_pgn.as_ref().unwrap().path.to_str().unwrap()).unwrap();
                let mut png_file_contents = String::new();
                file.read_to_string(&mut png_file_contents).expect("Unable to read file");
                //Library doesn't play nice with comments inside brackets
                //This gets rid of up to two levels of bracket nesting
                let re = Regex::new(r"\((?:[^)(]|\((?:[^)(]|\([^)(]*\))*\))*\)").unwrap();
                let result = re.replace_all(png_file_contents.as_str(), "");
                let re = Regex::new(r"\n").unwrap();
                let result = re.replace_all(&result, " ");
                let re = Regex::new(r"\s\s").unwrap();
                let result = re.replace_all(&result, " ");
                self.game_vec = match read_games(&result) {
                    Ok(games) => games,
                    Err(e) => {
                        println!("{:?}", e);
                        vec![]
                    }
                };
                self.total_pages = (self.game_vec.len() as f64 / REPLAYS_PER_PAGE as f64).ceil() as u32;
                self.selected_pgn_changed = false;
            }
            if self.game_vec.len() == 0 {
                canvas.draw_text(
                    Point2 {
                        x: None,
                        y: Some(700),
                    },
                    "Couldn't parse any games from PGN",
                    75.0,
                );
            } else {
                canvas.draw_text(
                    Point2 {
                        x: None,
                        y: Some(300),
                    },
                    "Choose Game:",
                    75.0,
                );
            }
            let index_of_first_game = (self.current_page_number * REPLAYS_PER_PAGE) as usize;
            self.button_1_hitbox = draw_button_for_game(canvas, self.game_vec.get(index_of_first_game), FIRST_BOX_Y_POS, 50.0);
            self.button_2_hitbox = draw_button_for_game(canvas, self.game_vec.get(index_of_first_game + 1), FIRST_BOX_Y_POS + BOX_HEIGHT, 50.0);
            self.button_3_hitbox = draw_button_for_game(canvas, self.game_vec.get(index_of_first_game + 2), FIRST_BOX_Y_POS + BOX_HEIGHT * 2, 50.0);
            self.button_4_hitbox = draw_button_for_game(canvas, self.game_vec.get(index_of_first_game + 3), FIRST_BOX_Y_POS + BOX_HEIGHT * 3, 50.0);
            self.button_5_hitbox = draw_button_for_game(canvas, self.game_vec.get(index_of_first_game + 4), FIRST_BOX_Y_POS + BOX_HEIGHT * 4, 50.0);
            self.button_6_hitbox = draw_button_for_game(canvas, self.game_vec.get(index_of_first_game + 5), FIRST_BOX_Y_POS + BOX_HEIGHT * 5, 50.0);
        }
        canvas.draw_text(
            Point2 {
                x: None,
                y: Some(150),
            },
            "chessMarkable",
            150.0,
        );
        let back_button_text = match choose_pgn_mode {
            true => "Main Menu",
            false => "PGNs"
        };
        self.back_button_hitbox = Some(canvas.draw_button(
            Point2 {
                x: None,
                y: Some(1700),
            },
            back_button_text,
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

        canvas.update_full();
    }

    fn on_input(&mut self, event: InputEvent) {
        if let InputEvent::MultitouchEvent { event } = event {
            if let MultitouchEvent::Release { finger, .. } = event {
                let position = finger.pos;
                if self.selected_pgn.is_some() {
                    if self.back_button_hitbox.is_some()
                        && Canvas::is_hitting(position, self.back_button_hitbox.unwrap())
                    {
                        self.unload_pgn();
                    } else if self.button_1_hitbox.is_some()
                        && Canvas::is_hitting(position, self.button_1_hitbox.unwrap())
                    {
                        self.indicate_loading = true;
                        self.button_1_pressed = true;
                    } else if self.button_2_hitbox.is_some()
                        && Canvas::is_hitting(position, self.button_2_hitbox.unwrap())
                    {
                        self.indicate_loading = true;
                        self.button_2_pressed = true;
                    } else if self.button_3_hitbox.is_some()
                        && Canvas::is_hitting(position, self.button_3_hitbox.unwrap())
                    {
                        self.indicate_loading = true;
                        self.button_3_pressed = true;
                    } else if self.button_4_hitbox.is_some()
                        && Canvas::is_hitting(position, self.button_4_hitbox.unwrap())
                    {
                        self.indicate_loading = true;
                        self.button_4_pressed = true;
                    } else if self.button_5_hitbox.is_some()
                        && Canvas::is_hitting(position, self.button_5_hitbox.unwrap())
                    {
                        self.indicate_loading = true;
                        self.button_5_pressed = true;
                    } else if self.button_6_hitbox.is_some()
                        && Canvas::is_hitting(position, self.button_6_hitbox.unwrap())
                    {
                        self.indicate_loading = true;
                        self.button_6_pressed = true;
                    }
                } else {
                    if self.back_button_hitbox.is_some()
                        && Canvas::is_hitting(position, self.back_button_hitbox.unwrap())
                    {
                        self.return_to_main_menu = true;
                    } else if self.button_1_hitbox.is_some()
                        && Canvas::is_hitting(position, self.button_1_hitbox.unwrap())
                    {
                        self.load_pgn(self.pgn_vec[0].clone())
                    } else if self.button_2_hitbox.is_some()
                        && Canvas::is_hitting(position, self.button_2_hitbox.unwrap())
                    {
                        self.load_pgn(self.pgn_vec[1].clone())
                    } else if self.button_3_hitbox.is_some()
                        && Canvas::is_hitting(position, self.button_3_hitbox.unwrap())
                    {
                        self.load_pgn(self.pgn_vec[2].clone())
                    } else if self.button_4_hitbox.is_some()
                        && Canvas::is_hitting(position, self.button_4_hitbox.unwrap())
                    {
                        self.load_pgn(self.pgn_vec[3].clone())
                    } else if self.button_5_hitbox.is_some()
                        && Canvas::is_hitting(position, self.button_5_hitbox.unwrap())
                    {
                        self.load_pgn(self.pgn_vec[4].clone())
                    } else if self.button_6_hitbox.is_some()
                        && Canvas::is_hitting(position, self.button_6_hitbox.unwrap())
                    {
                        self.load_pgn(self.pgn_vec[5].clone())
                    }
                }
                if self.next_page_button_hitbox.is_some()
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
            self.drawn = false;
            self.current_page_number = self.current_page_number + 1;
        }
    }

    fn go_to_prev_page(&mut self) {
        if self.current_page_number != 0 {
            self.drawn = false;
            self.current_page_number = self.current_page_number - 1;
        }
    }

    fn load_pgn(&mut self, pgn: Pgn) {
        self.selected_pgn = Some(pgn);
        self.selected_pgn_changed = true;
        self.drawn = false;
        self.current_page_number = 0;
    }

    fn unload_pgn(&mut self) {
        self.selected_pgn = None;
        self.drawn = false;
        self.current_page_number = 0;
    }
}

fn construct_text_for_replay(game: &Game) -> String {
    let default_tuple: &(String, String) = &("N/A".parse().unwrap(), "N/A".parse().unwrap());
    let white_tag: &(String, String) = game.tags.iter().find(|tag| tag.to_owned().0 == WHITE_TAG).unwrap_or(default_tuple);
    let black_tag: &(String, String) = game.tags.iter().find(|tag| tag.to_owned().0 == BLACK_TAG).unwrap_or(default_tuple);
    let event_tag: &(String, String) = game.tags.iter().find(|tag| tag.to_owned().0 == EVENT_TAG).unwrap_or(default_tuple);
    let round_tag: &(String, String) = game.tags.iter().find(|tag| tag.to_owned().0 == ROUND_TAG).unwrap_or(default_tuple);
    let mut replay_text = white_tag.to_owned().1;
    replay_text.push_str(" vs ");
    replay_text.push_str(&black_tag.1);
    replay_text.push_str(" at ");
    replay_text.push_str(&event_tag.1);
    replay_text.push_str(" ");
    replay_text.push_str(&round_tag.1);
    replay_text
}

fn draw_button_for_pgn(canvas: &mut Canvas, maybe_pgn_ref: Option<&Pgn>, y_pos: i32, font_size: f32) -> Option<mxcfb_rect> {
    match maybe_pgn_ref {
        Some(pgn_ref) => Some(canvas.draw_box_button(y_pos, BOX_HEIGHT as u32, &pgn_ref.path.file_name().unwrap().to_owned().into_string().unwrap_or("Can't read file name".to_string()), font_size)),
        None => None
    }
}

fn draw_button_for_game(canvas: &mut Canvas, maybe_game_ref: Option<&Game>, y_pos: i32, font_size: f32) -> Option<mxcfb_rect> {
    match maybe_game_ref {
        Some(game_ref) => Some(canvas.draw_box_button(y_pos, BOX_HEIGHT as u32, &construct_text_for_replay(game_ref), font_size)),
        None => None
    }
}
