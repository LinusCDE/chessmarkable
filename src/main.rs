#[macro_use]
extern crate downcast_rs;
#[macro_use]
extern crate lazy_static;

mod canvas;
mod scene;

use crate::canvas::Canvas;
use crate::scene::*;
use clap::{crate_authors, crate_version, Clap};
use lazy_static::lazy_static;
use libremarkable::input::{ev::EvDevContext, InputDevice, InputEvent};
use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, SystemTime};

#[derive(Clap)]
#[clap(version = crate_version!(), author = crate_authors!())]
pub struct Opts {
    #[clap(
        long,
        short = 'X',
        about = "Stop xochitl service when a xochitl process is found. Useful when running without any launcher."
    )]
    kill_xochitl: bool,

    #[clap(
        long,
        short = 'd',
        default_value = "1500",
        about = "Minimum amount of time, the bots wait before it makes its move in milliseconds"
    )]
    bot_reaction_delay: u16,

    #[clap(
        long,
        short = 'M',
        about = "Disable merging individual field updates into one big partial draw"
    )]
    no_merge: bool,
}

lazy_static! {
    pub static ref CLI_OPTS: Opts = Opts::parse();
}

fn main() {
    let only_exit_to_xochitl = if !CLI_OPTS.kill_xochitl {
        false
    } else if let Ok(status) = Command::new("pidof").arg("xochitl").status() {
        if status.code().unwrap() == 0 {
            Command::new("systemctl")
                .arg("stop")
                .arg("xochitl")
                .status()
                .ok();
            println!("Xochitl was found and killed. You may only exit by starting Xochitl again.");
            true
        } else {
            false
        }
    } else {
        false
    };

    let mut canvas = Canvas::new();

    let (input_tx, input_rx) = std::sync::mpsc::channel::<InputEvent>();
    EvDevContext::new(InputDevice::GPIO, input_tx.clone()).start();
    EvDevContext::new(InputDevice::Multitouch, input_tx).start();
    //EvDevContext::new(InputDevice::Wacom, input_tx.clone()).start();
    const FPS: u16 = 30;
    const FRAME_DURATION: Duration = Duration::from_millis(1000 / FPS as u64);

    let mut current_scene: Box<dyn Scene> = Box::new(MainMenuScene::new(only_exit_to_xochitl));

    loop {
        let before_input = SystemTime::now();
        for event in input_rx.try_iter() {
            current_scene.on_input(event);
        }

        current_scene.draw(&mut canvas);
        current_scene = update(current_scene, &mut canvas, only_exit_to_xochitl);

        // Wait remaining frame time
        let elapsed = before_input.elapsed().unwrap();
        if elapsed < FRAME_DURATION {
            sleep(FRAME_DURATION - elapsed);
        }
    }
}

fn update(
    scene: Box<dyn Scene>,
    canvas: &mut Canvas,
    only_exit_to_xochitl: bool,
) -> Box<dyn Scene> {
    if let Some(game_scene) = scene.downcast_ref::<GameScene>() {
        if game_scene.back_button_pressed {
            return Box::new(MainMenuScene::new(only_exit_to_xochitl));
        }
    } else if let Some(main_menu_scene) = scene.downcast_ref::<MainMenuScene>() {
        if main_menu_scene.play_pvp_button_pressed {
            return Box::new(GameScene::new(GameMode::PvP));
        } else if main_menu_scene.play_easy_button_pressed {
            return Box::new(GameScene::new(GameMode::EasyBot));
        } else if main_menu_scene.play_normal_button_pressed {
            return Box::new(GameScene::new(GameMode::NormalBot));
        } else if main_menu_scene.play_hard_button_pressed {
            return Box::new(GameScene::new(GameMode::HardBot));
        } else if main_menu_scene.exit_xochitl_button_pressed {
            canvas.clear();
            canvas.update_full();
            Command::new("systemctl")
                .arg("start")
                .arg("xochitl")
                .status()
                .ok();
            std::process::exit(0);
        } else if main_menu_scene.exit_button_pressed {
            canvas.clear();
            canvas.update_full();
            std::process::exit(0);
        }
    }
    scene
}
