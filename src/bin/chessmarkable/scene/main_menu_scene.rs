use super::Scene;
use crate::canvas::*;
use libremarkable::input::{multitouch::MultitouchEvent, InputEvent};

pub struct MainMenuScene {
    drawn: bool,

    play_pvp_button_hitbox: Option<mxcfb_rect>,
    pub play_pvp_button_pressed: bool,
    pvp_toggle_piece_rotation_hitbox: Option<mxcfb_rect>,
    pvp_toggle_piece_rotation_redraw: bool,
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
    pub pvp_piece_rotation_enabled: bool,
}

impl MainMenuScene {
    pub fn new(only_exit_to_xochitl: bool) -> Self {
        Self {
            drawn: false,
            play_pvp_button_hitbox: None,
            play_pvp_button_pressed: false,
            pvp_toggle_piece_rotation_hitbox: None,
            pvp_toggle_piece_rotation_redraw: false,
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
            pvp_piece_rotation_enabled: false,
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

    fn draw_rotation_button(&mut self, canvas: &mut Canvas) {
        if let Some(hitbox) = self.pvp_toggle_piece_rotation_hitbox {
            // Extand hitbox fully horizontal to accomodate enlargement of button
            canvas.fill_rect(
                Point2 {
                    x: Some(0),
                    y: Some((hitbox.top) as i32),
                },
                Vector2 {
                    x: DISPLAYWIDTH as u32,
                    y: hitbox.height,
                },
                color::WHITE,
            );
        }
        self.pvp_toggle_piece_rotation_hitbox = Some(canvas.draw_text(
            Point2 {
                x: None,
                y: Some(775),
            },
            "      Rotate board",
            50.0,
        ));

        canvas.draw_rect(
            Point2 {
                x: Some(self.pvp_toggle_piece_rotation_hitbox.unwrap().left as i32),
                y: Some((self.pvp_toggle_piece_rotation_hitbox.unwrap().top - 10) as i32),
            },
            Vector2 { x: 50, y: 50 },
            2,
        );

        if self.pvp_piece_rotation_enabled {
            canvas.fill_rect(
                Point2 {
                    x: Some(self.pvp_toggle_piece_rotation_hitbox.unwrap().left as i32 + 4),
                    y: Some((self.pvp_toggle_piece_rotation_hitbox.unwrap().top - 10 + 4) as i32),
                },
                Vector2 {
                    x: 50 - 8,
                    y: 50 - 8,
                },
                color::BLACK,
            );
        }

        // Update button and everything to left and right since button may have enlarged
        let mut ext_hitbox = self.pvp_toggle_piece_rotation_hitbox.clone().unwrap();
        ext_hitbox.top -= 10;
        ext_hitbox.height += 20;
        self.pvp_toggle_piece_rotation_hitbox = Some(ext_hitbox);
    }
}

impl Scene for MainMenuScene {
    fn draw(&mut self, canvas: &mut Canvas) {
        if self.indicate_loading {
            self.indicate_loading(canvas);
            self.indicate_loading = false;
            return;
        }

        if self.pvp_toggle_piece_rotation_redraw {
            self.draw_rotation_button(canvas);
            canvas.update_partial(&self.pvp_toggle_piece_rotation_hitbox.unwrap());
            self.pvp_toggle_piece_rotation_redraw = false;
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
            "chessMarkable",
            150.0,
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

        self.draw_rotation_button(canvas);

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
            if let MultitouchEvent::Release { finger, .. } = event {
                let position = finger.pos;
                if self.play_pvp_button_hitbox.is_some()
                    && Canvas::is_hitting(position, self.play_pvp_button_hitbox.unwrap())
                {
                    self.play_pvp_button_pressed = true;
                    self.indicate_loading = true;
                }
                if self.pvp_toggle_piece_rotation_hitbox.is_some()
                    && Canvas::is_hitting(position, self.pvp_toggle_piece_rotation_hitbox.unwrap())
                {
                    self.pvp_piece_rotation_enabled = !self.pvp_piece_rotation_enabled;
                    self.pvp_toggle_piece_rotation_redraw = true;
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
