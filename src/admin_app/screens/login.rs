use std::option::Option;

use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyModifiers},
    layout::{Constraint, Layout},
    style::Style,
    widgets::{Block, Paragraph},
};

use crate::{
    admin_app::screens::flags::AdminScreen,
    conf::Conf,
    screen::{Screen, draw_screen_border},
};

pub struct LoginScreen {
    password_input: String,
    conf: Conf,
    error: Option<String>,
}

impl Screen for LoginScreen {
    fn handle_input(
        &mut self,
        key: Option<(KeyCode, KeyModifiers)>,
    ) -> Option<Box<dyn Screen + Send>> {
        // if no key is pressed, return early for now
        let key = key?;

        self.error = None;
        match key {
            (KeyCode::Enter, _) => return self.submit(),
            (KeyCode::Backspace, _) => self.erase(),
            (KeyCode::Char(c), _) => self.write_char(c),
            _ => (),
        };
        None
    }
    fn render(&mut self, f: &mut Frame) {
        let area = draw_screen_border(
            f,
            vec!["ADMIN LOGIN"],
            0,
            "^Q[QUIT] ↵[SUBMIT]",
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
        let [_, user, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Fill(1),
        ])
        .areas(col);

        let color = Style::new()
            .fg(self.conf.theme.base08)
            .bg(self.conf.theme.base00);

        f.render_widget(
            Paragraph::new(self.password_input.as_str())
                .block(Block::bordered().title_top("ADMIN PASSWORD"))
                .style(color),
            user,
        );
    }
}

impl LoginScreen {
    fn submit(&mut self) -> Option<Box<dyn Screen + Send>> {
        if self
            .conf
            .admin_password
            .as_ref()
            .is_none_or(|x| *x == self.password_input)
        {
            return Some(Box::new(AdminScreen::new(self.conf.clone())));
        }
        self.error = Some("wrong password".to_string());
        None
    }

    pub fn new(conf: Conf) -> Self {
        Self {
            password_input: String::new(),
            error: None,
            conf,
        }
    }

    fn erase(&mut self) {
        self.password_input.pop();
    }

    fn write_char(&mut self, c: char) {
        self.password_input.push(c);
    }
}
