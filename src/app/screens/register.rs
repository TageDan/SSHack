use crate::{
    app::screens::{flags::BrowseScreen, home::HomeScreen},
    conf::Conf,
    database,
    screen::{Screen, draw_screen_border},
};
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Layout},
    style::Style,
    widgets::{Block, Paragraph},
};
use ratatui_textarea::TextArea;

pub struct RegisterScreen<'a> {
    username: TextArea<'a>,
    password: TextArea<'a>,
    selected: u8,
    key: russh::keys::PublicKey,
    error: Option<String>,
    conf: Conf,
}

impl Screen for RegisterScreen<'_> {
    fn handle_input(
        &mut self,
        key: Option<(KeyCode, KeyModifiers)>,
    ) -> Option<Box<dyn Screen + Send>> {
        // if no key is pressed, return early for now
        let key = key?;
        // Remove error on input
        self.error = None;
        match key {
            (KeyCode::Enter, _) => return self.submit(),
            (KeyCode::Up, _) | (KeyCode::BackTab, KeyModifiers::SHIFT) => self.prev(),
            (KeyCode::Down, _) | (KeyCode::Tab, _) => self.next(),
            (KeyCode::Esc, _) => {
                return Some(Box::new(HomeScreen::new(
                    self.conf.clone(),
                    self.key.clone(),
                )));
            }
            (k, m) => {
                let event = KeyEvent::new(k, m);
                self.handle_input(event);
            }
            _ => (),
        };
        None
    }
    fn render(&mut self, f: &mut ratatui::Frame) {
        let area = draw_screen_border(
            f,
            vec!["REGISTER"],
            0,
            if self.conf.password.is_some() {
                "^Q[QUIT] ↵[SUBMIT]"
            } else {
                "^Q[QUIT] ↵[SUBMIT] ⇵[NAV]"
            },
            self.error.as_deref(),
            None,
            &self.conf,
        );

        let [_, col, _] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Fill(2),
            Constraint::Fill(1),
        ])
        .areas(area);

        if self.conf.password.is_none() {
            let [_, user, _] = Layout::vertical([
                Constraint::Fill(1),
                Constraint::Length(3),
                Constraint::Fill(1),
            ])
            .areas(col);

            let color = Style::new()
                .fg(self.conf.theme.base08)
                .bg(self.conf.theme.base00);

            let block = Block::bordered().title_top("USERNAME").style(color);

            f.render_widget(&block, user);

            f.render_widget(&self.username, block.inner(user));
        } else {
            let [_, user, pass, _] = Layout::vertical([
                Constraint::Fill(1),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Fill(1),
            ])
            .areas(col);

            let color1 = Style::new()
                .fg(self.conf.theme.base08)
                .bg(self.conf.theme.base00);
            let color2 = Style::new()
                .fg(self.conf.theme.base05)
                .bg(self.conf.theme.base00);

            let block = Block::bordered()
                .title_top("USERNAME")
                .style(if self.selected == 0 { color1 } else { color2 });

            f.render_widget(&block, user);

            f.render_widget(&self.username, block.inner(user));

            let block = Block::bordered()
                .title_top("PASSWORD")
                .style(if self.selected == 1 { color1 } else { color2 });

            f.render_widget(&block, pass);

            f.render_widget(&self.password, block.inner(pass));
        }
    }
}

impl RegisterScreen<'_> {
    pub fn new(conf: Conf, key: russh::keys::PublicKey) -> Self {
        let mut password_text_area = TextArea::default();
        password_text_area.set_mask_char('\u{2022}');

        Self {
            username: TextArea::default(),
            password: password_text_area,
            selected: 0,
            key,
            error: None,
            conf,
        }
    }

    fn submit(&mut self) -> Option<Box<dyn Screen + Send>> {
        if self.conf.password.is_some() && self.selected == 0 {
            self.selected = 1;
            return None;
        }
        if self.username.is_empty() {
            self.error = Some("username can not be empty".to_string());
            return None;
        }
        if self
            .conf
            .password
            .as_ref()
            .is_some_and(|x| x != &self.password.lines()[0])
        {
            self.error = Some("Wrong password".to_string());
            return None;
        }
        match database::User::register_user(&self.username.lines()[0], self.key.clone()) {
            Err(e) => {
                self.error = Some(e.to_string());
                return None;
            }
            Ok(u) => {
                return Some(Box::new(BrowseScreen::new(u, self.conf.clone())));
            }
        }
    }

    fn handle_input(&mut self, e: KeyEvent) {
        match self.selected {
            0 => {
                self.username.input(e);
            }
            1 => {
                self.password.input(e);
            }
            _ => (),
        };
    }

    fn next(&mut self) {
        if self.conf.password.is_some() {
            self.selected = 1.min(self.selected + 1);
        }
    }

    fn prev(&mut self) {
        if self.conf.password.is_some() {
            self.selected = self.selected.saturating_sub(1);
        }
    }
}
