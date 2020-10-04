mod game_scene;
mod main_menu_scene;

pub use game_scene::GameScene;
pub use main_menu_scene::MainMenuScene;

use crate::canvas::Canvas;
use downcast_rs::Downcast;
use libremarkable::input::InputEvent;

pub trait Scene: Downcast {
    fn on_input(&mut self, _event: InputEvent) { }
    fn draw(&mut self, canvas: &mut Canvas);
}
impl_downcast!(Scene);
