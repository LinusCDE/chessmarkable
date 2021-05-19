use super::Scene;
use crate::canvas::*;
use crate::savestates::Savestates;
use libremarkable::input::{multitouch::MultitouchEvent, InputEvent};

pub struct BoardSelectScene {
    drawn: bool,

    pub selected_gamemode: crate::scene::GameMode,
    pub pvp_piece_rotation_enabled: bool,

    select_slot_1_button_hitbox: Option<mxcfb_rect>,
    pub select_slot_1_button_pressed: bool,
    select_slot_2_button_hitbox: Option<mxcfb_rect>,
    pub select_slot_2_button_pressed: bool,
    select_slot_3_button_hitbox: Option<mxcfb_rect>,
    pub select_slot_3_button_pressed: bool,

    reset_slot_1_button_hitbox: Option<mxcfb_rect>,
    pub reset_slot_1_button_pressed: bool,
    reset_slot_2_button_hitbox: Option<mxcfb_rect>,
    pub reset_slot_2_button_pressed: bool,
    reset_slot_3_button_hitbox: Option<mxcfb_rect>,
    pub reset_slot_3_button_pressed: bool,

    back_button_hitbox: Option<mxcfb_rect>,
    pub back_button_pressed: bool,

    indicate_loading: bool,
}

impl BoardSelectScene {
    pub fn new(
        selected_gamemode: crate::scene::GameMode,
        pvp_piece_rotation_enabled: bool,
    ) -> Self {
        Self {
            drawn: false,
            selected_gamemode,
            pvp_piece_rotation_enabled,
            select_slot_1_button_hitbox: None,
            select_slot_1_button_pressed: false,
            select_slot_2_button_hitbox: None,
            select_slot_2_button_pressed: false,
            select_slot_3_button_hitbox: None,
            select_slot_3_button_pressed: false,

            reset_slot_1_button_hitbox: None,
            reset_slot_1_button_pressed: false,
            reset_slot_2_button_hitbox: None,
            reset_slot_2_button_pressed: false,
            reset_slot_3_button_hitbox: None,
            reset_slot_3_button_pressed: false,
            back_button_hitbox: None,
            back_button_pressed: false,
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

impl Scene for BoardSelectScene {
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
            "chessMarkable",
            150.0,
        );

        canvas.draw_text(
            Point2 {
                x: None,
                y: Some(500),
            },
            "Continue on...",
            75.0,
        );

        self.select_slot_1_button_hitbox = Some(canvas.draw_button(
            Point2 {
                x: None,
                y: Some(600),
            },
            "Slot 1",
            75.0,
            25,
            50,
        ));

        self.select_slot_2_button_hitbox = Some(canvas.draw_button(
            Point2 {
                x: None,
                y: Some(
                    100 + self.select_slot_1_button_hitbox.unwrap().top as i32
                        + self.select_slot_1_button_hitbox.unwrap().height as i32,
                ),
            },
            "Slot 2",
            75.0,
            25,
            50,
        ));

        self.select_slot_3_button_hitbox = Some(canvas.draw_button(
            Point2 {
                x: None,
                y: Some(
                    100 + self.select_slot_2_button_hitbox.unwrap().top as i32
                        + self.select_slot_2_button_hitbox.unwrap().height as i32,
                ),
            },
            "Slot 3",
            75.0,
            25,
            50,
        ));

        canvas.draw_text(
            Point2 {
                x: None,
                y: Some(1000),
            },
            "Start over on...",
            75.0,
        );

        self.reset_slot_1_button_hitbox = Some(canvas.draw_button(
            Point2 {
                x: None,
                y: Some(1100),
            },
            "Slot 1",
            75.0,
            25,
            50,
        ));

        self.reset_slot_2_button_hitbox = Some(canvas.draw_button(
            Point2 {
                x: None,
                y: Some(
                    100 + self.reset_slot_1_button_hitbox.unwrap().top as i32
                        + self.reset_slot_1_button_hitbox.unwrap().height as i32,
                ),
            },
            "Slot 2",
            75.0,
            25,
            50,
        ));

        self.reset_slot_3_button_hitbox = Some(canvas.draw_button(
            Point2 {
                x: None,
                y: Some(
                    100 + self.reset_slot_2_button_hitbox.unwrap().top as i32
                        + self.reset_slot_2_button_hitbox.unwrap().height as i32,
                ),
            },
            "Slot 3",
            75.0,
            25,
            50,
        ));

        self.back_button_hitbox = Some(canvas.draw_button(
            Point2 {
                x: None,
                y: Some(1700),
            },
            "Back",
            125.0,
            25,
            50,
        ));

        canvas.update_full();
    }

    fn on_input(&mut self, event: InputEvent) {
        if let InputEvent::MultitouchEvent { event } = event {
            if let MultitouchEvent::Release { finger, .. } = event {
                let position = finger.pos;
                if self.select_slot_1_button_hitbox.is_some()
                    && Canvas::is_hitting(position, self.select_slot_1_button_hitbox.unwrap())
                {
                    self.select_slot_1_button_pressed = true;
                    self.indicate_loading = true;
                } else if self.select_slot_2_button_hitbox.is_some()
                    && Canvas::is_hitting(position, self.select_slot_2_button_hitbox.unwrap())
                {
                    self.select_slot_2_button_pressed = true;
                    self.indicate_loading = true;
                } else if self.select_slot_3_button_hitbox.is_some()
                    && Canvas::is_hitting(position, self.select_slot_3_button_hitbox.unwrap())
                {
                    self.select_slot_3_button_pressed = true;
                    self.indicate_loading = true;
                } else if self.reset_slot_1_button_hitbox.is_some()
                    && Canvas::is_hitting(position, self.reset_slot_1_button_hitbox.unwrap())
                {
                    self.reset_slot_1_button_pressed = true;
                    self.indicate_loading = true;
                } else if self.reset_slot_2_button_hitbox.is_some()
                    && Canvas::is_hitting(position, self.reset_slot_2_button_hitbox.unwrap())
                {
                    self.reset_slot_2_button_pressed = true;
                    self.indicate_loading = true;
                } else if self.reset_slot_3_button_hitbox.is_some()
                    && Canvas::is_hitting(position, self.reset_slot_3_button_hitbox.unwrap())
                {
                    self.reset_slot_3_button_pressed = true;
                    self.indicate_loading = true;
                } else if self.back_button_hitbox.is_some()
                    && Canvas::is_hitting(position, self.back_button_hitbox.unwrap())
                {
                    self.back_button_pressed = true;
                }
            }
        }
    }
}
