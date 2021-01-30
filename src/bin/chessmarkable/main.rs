#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate downcast_rs;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

mod canvas;
mod scene;

use crate::canvas::Canvas;
use crate::scene::*;
use clap::{crate_authors, crate_version, Clap};
use lazy_static::lazy_static;
use libremarkable::input::{ev::EvDevContext, InputDevice, InputEvent};
use libremarkable::device::{CURRENT_DEVICE, Model};
use std::env;
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

    #[clap(
        long,
        short,
        about = "FEN used for the initial board instead of the default postions. You can get the fen of a game by setting env RUST_LOG=debug"
    )]
    intial_fen: Option<String>,
}

lazy_static! {
    pub static ref CLI_OPTS: Opts = Opts::parse();
}

fn main() {
    let show_log_info = if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "DEBUG");
        true
    } else {
        false
    };
    env_logger::init();
    if show_log_info {
        debug!(concat!(
            "Debug Mode is enabled by default.\n",
            "To change this, set the env \"RUST_LOG\" something else ",
            "(e.g. info, warn, error or comma separated list of \"[module=]<level>\")."
        ));
    }

    if CURRENT_DEVICE.model == Model::Gen2 && std::env::var_os("LD_PRELOAD").is_none() {
        warn!(concat!(
            "\n",
            "You executed retris on a reMarkable 2 without having LD_PRELOAD set.\n",
            "This suggests that you didn't use/enable rm2fb. Without rm2fb you\n",
            "won't see anything on the display!\n",
            "\n",
            "See https://github.com/ddvk/remarkable2-framebuffer/ on how to solve\n",
            "this. Launchers (installed through toltec) should automatically do this."
        ));
    }

    let only_exit_to_xochitl = if !CLI_OPTS.kill_xochitl {
        false
    } else if let Ok(status) = Command::new("pidof").arg("xochitl").status() {
        if status.code().unwrap() == 0 {
            Command::new("systemctl")
                .arg("stop")
                .arg("xochitl")
                .status()
                .ok();
            info!("Xochitl was found and killed. You may only exit by starting Xochitl again.");
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
            return Box::new(GameScene::new(
                GameMode::PvP,
                main_menu_scene.pvp_piece_rotation_enabled,
            ));
        } else if main_menu_scene.play_easy_button_pressed {
            return Box::new(GameScene::new(GameMode::EasyBot, false));
        } else if main_menu_scene.play_normal_button_pressed {
            return Box::new(GameScene::new(GameMode::NormalBot, false));
        } else if main_menu_scene.play_hard_button_pressed {
            return Box::new(GameScene::new(GameMode::HardBot, false));
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
