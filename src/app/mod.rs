pub mod screens;
pub mod server;

use ratatui::Frame;

use ratatui::crossterm;
use ratatui::crossterm::event::KeyModifiers;

use ratatui::crossterm::event::KeyCode;

use std::time::Duration;

use ratatui::crossterm::event::poll;

use crate::cli::Args;
use crate::screen::Screen;
use screens::leaderboard::LeaderboardScreen;

use screens::home::HomeScreen;

use std::error::Error;

use crate::conf::Conf;

use ratatui::DefaultTerminal;

pub(crate) fn app(
    terminal: &mut DefaultTerminal,
    conf: Conf,
    leaderboard: bool,
) -> Result<(), Box<dyn Error>> {
    let mut app = App {
        screen: if !leaderboard {
            Box::new(screens::home::HomeScreen::new(
            conf,
            russh::keys::PublicKey::read_openssh_file(
                &std::env::home_dir()
                    .ok_or(
                        "to run locally you still need a public key under ~/.shh/id_ed_25519.pub, this is just so that your account can be identified correctly",
                    )?
                    .join(".ssh/id_ed25519.pub"),
            )?,
        ))
        } else {
            Box::new(LeaderboardScreen::new(None, conf))
        },
    };
    loop {
        terminal.draw(|f| app.render(f))?;
        if poll(Duration::from_millis(50)).is_ok_and(|x| x) {
            if let Some(k) = crossterm::event::read()?.as_key_press_event() {
                match (k.code, k.modifiers) {
                    (KeyCode::Char('q'), KeyModifiers::CONTROL) => return Ok(()),
                    k => {
                        if let Some(t) = app.screen.handle_input(Some(k)) {
                            app.screen = t;
                        }
                    }
                }
            }
        }
        if let Some(t) = app.screen.handle_input(None) {
            app.screen = t;
        }
    }
}

pub struct App {
    pub screen: Box<dyn Screen + Send>,
}

impl App {
    pub fn render(&mut self, f: &mut Frame) {
        self.screen.render(f)
    }

    pub(crate) fn new(conf: Conf, key: russh::keys::PublicKey, args: Args) -> Self {
        Self {
            screen: if !args.leaderboard() {
                Box::new(HomeScreen::new(conf, key))
            } else {
                Box::new(LeaderboardScreen::new(None, conf))
            },
        }
    }
}
