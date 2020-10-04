use super::Scene;
use crate::canvas::*;
use libremarkable::input::{multitouch::MultitouchEvent, InputEvent};

pub struct MainMenuScene {
    drawn: bool,

    play_pvp_button_hitbox: Option<mxcfb_rect>,
    pub play_pvp_button_pressed: bool,
    play_easy_button_hitbox: Option<mxcfb_rect>,
    pub play_easy_button_pressed: bool,
    play_normal_button_hitbox: Option<mxcfb_rect>,
    pub play_normal_button_pressed: bool,
    play_hard_button_hitbox: Option<mxcfb_rect>,
    pub play_hard_button_pressed: bool,

    exit_button_hitbox: Option<mxcfb_rect>,
    pub exit_button_pressed: bool,

    exit_xochitl_button_hitbox: Option<mxcfb_rect>,
    pub exit_xochitl_button_pressed: bool,

    only_exit_to_xochitl: bool,
    indicate_loading: bool,
}

impl MainMenuScene {
    pub fn new(only_exit_to_xochitl: bool) -> Self {
        Self {
            drawn: false,
            play_pvp_button_hitbox: None,
            play_pvp_button_pressed: false,
            play_easy_button_hitbox: None,
            play_easy_button_pressed: false,
            play_normal_button_hitbox: None,
            play_normal_button_pressed: false,
            play_hard_button_hitbox: None,
            play_hard_button_pressed: false,
            exit_button_hitbox: None,
            exit_button_pressed: false,
            exit_xochitl_button_hitbox: None,
            exit_xochitl_button_pressed: false,
            only_exit_to_xochitl,
            indicate_loading: false,
        }
    }

    fn indicate_loading(&self, canvas: &mut Canvas) {
        let rect = canvas.draw_text(
            Point2 {
                x: None,
                y: Some(350),
            },
            "Loading game... (preparing assets)",
            50.0,
        );
        canvas.update_partial(&rect);
    }
}

impl Scene for MainMenuScene {
    fn draw(&mut self, canvas: &mut Canvas) {
        if self.indicate_loading {
            self.indicate_loading(canvas);
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
                y: Some(300),
            },
            "Chess",
            400.0,
        );

        canvas.draw_text(
            Point2 {
                x: None,
                y: Some(500),
            },
            "Player vs Player",
            75.0,
        );

        self.play_pvp_button_hitbox = Some(canvas.draw_button(
            Point2 {
                x: None,
                y: Some(650),
            },
            "Play!",
            125.0,
            25,
            50,
        ));

        canvas.draw_text(
            Point2 {
                x: None,
                y: Some(900),
            },
            "Player vs Bot",
            75.0,
        );

        self.play_easy_button_hitbox = Some(canvas.draw_button(
            Point2 {
                x: None,
                y: Some(1050),
            },
            "Easy",
            125.0,
            25,
            50,
        ));
        self.play_normal_button_hitbox = Some(canvas.draw_button(
            Point2 {
                x: None,
                y: Some(
                    150 + self.play_easy_button_hitbox.unwrap().top as i32
                        + self.play_easy_button_hitbox.unwrap().height as i32,
                ),
            },
            "Normal",
            125.0,
            25,
            50,
        ));
        self.play_hard_button_hitbox = Some(canvas.draw_button(
            Point2 {
                x: None,
                y: Some(
                    150 + self.play_normal_button_hitbox.unwrap().top as i32
                        + self.play_normal_button_hitbox.unwrap().height as i32,
                ),
            },
            "Hard",
            125.0,
            25,
            50,
        ));

        if self.only_exit_to_xochitl {
            self.exit_xochitl_button_hitbox = Some(canvas.draw_button(
                Point2 {
                    x: None,
                    y: Some(1650),
                },
                "Exit to Xochitl",
                125.0,
                25,
                50,
            ));
        } else {
            self.exit_button_hitbox = Some(canvas.draw_button(
                Point2 {
                    x: None,
                    y: Some(1700),
                },
                "Exit",
                125.0,
                25,
                50,
            ));
        }

        canvas.update_full();
    }

    fn on_input(&mut self, event: InputEvent) {
        if let InputEvent::MultitouchEvent { event } = event {
            if let MultitouchEvent::Press { finger, .. } = event {
                let position = finger.pos;
                if self.play_pvp_button_hitbox.is_some()
                    && Canvas::is_hitting(position, self.play_pvp_button_hitbox.unwrap())
                {
                    self.play_pvp_button_pressed = true;
                    self.indicate_loading = true;
                } else if self.play_easy_button_hitbox.is_some()
                    && Canvas::is_hitting(position, self.play_easy_button_hitbox.unwrap())
                {
                    self.play_easy_button_pressed = true;
                    self.indicate_loading = true;
                } else if self.play_normal_button_hitbox.is_some()
                    && Canvas::is_hitting(position, self.play_normal_button_hitbox.unwrap())
                {
                    self.play_normal_button_pressed = true;
                    self.indicate_loading = true;
                } else if self.play_hard_button_hitbox.is_some()
                    && Canvas::is_hitting(position, self.play_hard_button_hitbox.unwrap())
                {
                    self.play_hard_button_pressed = true;
                    self.indicate_loading = true;
                } else if self.exit_button_hitbox.is_some()
                    && Canvas::is_hitting(position, self.exit_button_hitbox.unwrap())
                {
                    self.exit_button_pressed = true;
                } else if self.exit_xochitl_button_hitbox.is_some()
                    && Canvas::is_hitting(position, self.exit_xochitl_button_hitbox.unwrap())
                {
                    self.exit_xochitl_button_pressed = true;
                }
            }
        }
    }
}
