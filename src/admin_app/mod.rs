pub mod screens;
pub mod server;

use ratatui::Frame;

use crate::cli::Args;

use screens::home::HomeScreen;

use crate::conf::Conf;
use crate::screen;

pub struct AdminApp {
    pub screen: Box<dyn screen::Screen + Send>,
}

impl AdminApp {
    pub fn render(&mut self, f: &mut Frame) {
        self.screen.render(f)
    }

    pub(crate) fn new(conf: Conf, key: russh::keys::PublicKey, _args: Args) -> Self {
        Self {
            screen: Box::new(HomeScreen::new(conf, key)),
        }
    }
}
