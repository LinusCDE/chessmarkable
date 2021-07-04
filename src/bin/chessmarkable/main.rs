#[macro_use]
extern crate anyhow;
#[macro_use]
extern crate downcast_rs;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;

mod canvas;
mod savestates;
mod scene;
mod pgns;

use crate::canvas::Canvas;
use crate::scene::*;
use anyhow::Context;
use clap::{crate_authors, crate_version, Clap};
use lazy_static::lazy_static;
use libremarkable::device::{Model, CURRENT_DEVICE};
use libremarkable::input::{ev::EvDevContext, InputDevice, InputEvent};
use savestates::Savestates;
use std::env;
use std::process::Command;
use std::thread::sleep;
use std::time::{Duration, SystemTime};

#[derive(Clap)]
#[clap(version = crate_version ! (), author = crate_authors ! ())]
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
    short = 'f',
    about = "Path to the file containing the savestates",
    default_value = "/home/root/.config/chessmarkable/savestates.yml"
    )]
    savestates_file: std::path::PathBuf,

    #[clap(
    long,
    short = 'p',
    about = "Path to the file containing the PGNs for PGN viewer",
    default_value = "/home/root/.config/chessmarkable/pgn"
    )]
    pgn_location: std::path::PathBuf,
}

lazy_static! {
    pub static ref CLI_OPTS: Opts = Opts::parse();
    pub static ref SAVESTATES: std::sync::Mutex<Savestates> =
        std::sync::Mutex::new(Default::default());
        // Underlays / Background layers
}

pub const REPLAYS_PER_PAGE: u32 = 6;

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

    *SAVESTATES.lock().unwrap() = match savestates::read() {
        Ok(savestates) => savestates,
        Err(err) => {
            error!(
                "Failed to read savestates file at {:?}: {:?}",
                &CLI_OPTS.savestates_file, err
            );
            std::process::exit(1);
        }
    };

    let mut canvas = Canvas::new();

    let (input_tx, input_rx) = std::sync::mpsc::channel::<InputEvent>();
    EvDevContext::new(InputDevice::GPIO, input_tx.clone()).start();
    EvDevContext::new(InputDevice::Multitouch, input_tx).start();
    //EvDevContext::new(InputDevice::Wacom, input_tx.clone()).start();
    const FPS: u16 = 30;
    const FRAME_DURATION: Duration = Duration::from_millis(1000 / FPS as u64);

    let mut current_scene: Box<dyn Scene> =
        Box::new(MainMenuScene::new(only_exit_to_xochitl, false));

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
            return Box::new(MainMenuScene::new(only_exit_to_xochitl, false));
        }
    } else if let Some(main_menu_scene) = scene.downcast_ref::<MainMenuScene>() {
        let pvp_rot_en = main_menu_scene.pvp_piece_rotation_enabled;
        if main_menu_scene.play_pvp_button_pressed {
            return Box::new(BoardSelectScene::new(GameMode::PvP, pvp_rot_en));
        } else if main_menu_scene.play_easy_button_pressed {
            return Box::new(BoardSelectScene::new(GameMode::EasyBot, pvp_rot_en));
        } else if main_menu_scene.play_normal_button_pressed {
            return Box::new(BoardSelectScene::new(GameMode::NormalBot, pvp_rot_en));
        } else if main_menu_scene.play_hard_button_pressed {
            return Box::new(BoardSelectScene::new(GameMode::HardBot, pvp_rot_en));
        } else if main_menu_scene.viewer_button_pressed {
            return Box::new(PgnSelectScene::new(None));
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
    } else if let Some(board_select_scene) = scene.downcast_ref::<BoardSelectScene>() {
        if board_select_scene.select_slot_1_button_pressed {
            return Box::new(GameScene::new(
                board_select_scene.selected_gamemode,
                SavestateSlot::First,
                board_select_scene.pvp_piece_rotation_enabled,
            ));
        } else if board_select_scene.select_slot_2_button_pressed {
            return Box::new(GameScene::new(
                board_select_scene.selected_gamemode,
                SavestateSlot::Second,
                board_select_scene.pvp_piece_rotation_enabled,
            ));
        } else if board_select_scene.select_slot_3_button_pressed {
            return Box::new(GameScene::new(
                board_select_scene.selected_gamemode,
                SavestateSlot::Third,
                board_select_scene.pvp_piece_rotation_enabled,
            ));
        } else if board_select_scene.reset_slot_1_button_pressed {
            SAVESTATES.lock().unwrap().slot_1 = None;
            return Box::new(GameScene::new(
                board_select_scene.selected_gamemode,
                SavestateSlot::First,
                board_select_scene.pvp_piece_rotation_enabled,
            ));
        } else if board_select_scene.reset_slot_2_button_pressed {
            SAVESTATES.lock().unwrap().slot_2 = None;
            return Box::new(GameScene::new(
                board_select_scene.selected_gamemode,
                SavestateSlot::Second,
                board_select_scene.pvp_piece_rotation_enabled,
            ));
        } else if board_select_scene.reset_slot_3_button_pressed {
            SAVESTATES.lock().unwrap().slot_3 = None;
            return Box::new(GameScene::new(
                board_select_scene.selected_gamemode,
                SavestateSlot::Third,
                board_select_scene.pvp_piece_rotation_enabled,
            ));
        } else if board_select_scene.back_button_pressed {
            return Box::new(MainMenuScene::new(
                only_exit_to_xochitl,
                board_select_scene.pvp_piece_rotation_enabled,
            ));
        }
    } else if let Some(board_select_scene) = scene.downcast_ref::<PgnSelectScene>() {
        let index_of_first_game = (board_select_scene.current_page_number * REPLAYS_PER_PAGE) as usize;
        if board_select_scene.return_to_main_menu {
            return Box::new(MainMenuScene::new(
                only_exit_to_xochitl,
                false,
            ));
        } else if board_select_scene.button_1_pressed {
            return Box::new(ReplayScene::new(
                Some(board_select_scene.game_vec.get(index_of_first_game).unwrap().clone()),
                board_select_scene.selected_pgn.clone()
            ));
        } else if board_select_scene.button_2_pressed {
            return Box::new(ReplayScene::new(
                Some(board_select_scene.game_vec.get(index_of_first_game + 1).unwrap().clone()),
                board_select_scene.selected_pgn.clone()
            ));
        } else if board_select_scene.button_3_pressed {
            return Box::new(ReplayScene::new(
                Some(board_select_scene.game_vec.get(index_of_first_game + 2).unwrap().clone()),
                board_select_scene.selected_pgn.clone()
            ));
        } else if board_select_scene.button_4_pressed {
            return Box::new(ReplayScene::new(
                Some(board_select_scene.game_vec.get(index_of_first_game + 3).unwrap().clone()),
                board_select_scene.selected_pgn.clone()
            ));
        } else if board_select_scene.button_5_pressed {
            return Box::new(ReplayScene::new(
                Some(board_select_scene.game_vec.get(index_of_first_game + 4).unwrap().clone()),
                board_select_scene.selected_pgn.clone()
            ));
        } else if board_select_scene.button_6_pressed {
            return Box::new(ReplayScene::new(
                Some(board_select_scene.game_vec.get(index_of_first_game + 5).unwrap().clone()),
                board_select_scene.selected_pgn.clone()
            ));
        };
    } else if let Some(board_select_scene) = scene.downcast_ref::<ReplayScene>() {
        if board_select_scene.return_to_main_menu {
            return Box::new(PgnSelectScene::new(
                board_select_scene.selected_pgn.clone()
            ));
        }
    }
    scene
}
