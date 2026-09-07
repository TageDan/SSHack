use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyModifiers},
    layout::{Constraint, Layout, Rect},
    widgets::Paragraph,
};

use crate::{
    app::screens::leaderboard::LeaderboardScreen,
    conf::Conf,
    database::User,
    screen::{Screen, draw_screen_border},
};

pub struct AboutScreen {
    user: Option<User>,
    error: Option<String>,
    conf: Conf,
    scroll: u16,
}

impl Screen for AboutScreen {
    fn handle_input(
        &mut self,
        key: Option<(KeyCode, KeyModifiers)>,
    ) -> Option<Box<dyn Screen + Send>> {
        // if no key is pressed, return early for now
        let key = key?;
        self.error = None;
        match key {
            (KeyCode::Tab, _) | (KeyCode::Down, _) => self.focus_next(),
            (KeyCode::BackTab, KeyModifiers::SHIFT) | (KeyCode::Up, _) => self.focus_prev(),
            (KeyCode::Left, KeyModifiers::CONTROL) => {
                return Some(Box::new(LeaderboardScreen::new(
                    self.user.clone(),
                    self.conf.clone(),
                )));
            }
            _ => (),
        };
        None
    }
    fn render(&mut self, f: &mut Frame) {
        let commands = "^Q[QUIT] ⇵[SCROLL] ^⇄[TAB]";
        let area = draw_screen_border(
            f,
            vec!["FLAGS", "LEADERBOARD", "ABOUT"],
            2,
            commands,
            self.error.as_deref(),
            self.user.as_ref(),
            &self.conf,
        );

        self.draw_about(f, area);
    }
}

impl AboutScreen {
    pub fn new(user: Option<User>, conf: Conf) -> Self {
        Self {
            user,
            conf,
            error: None,
            scroll: 0,
        }
    }

    fn focus_next(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }

    fn focus_prev(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    fn draw_about(&mut self, f: &mut Frame, a: Rect) {
        let [_, area, _] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Fill(4),
            Constraint::Fill(1),
        ])
        .areas(a);
        f.render_widget(
            Paragraph::new(self.conf.about.as_ref().unwrap_or(&"".to_string()).as_str())
                .wrap(ratatui::widgets::Wrap { trim: false })
                .scroll((self.scroll, 0))
                .style(
                    ratatui::style::Style::new()
                        .bg(self.conf.theme.base00)
                        .fg(self.conf.theme.base05),
                ),
            area,
        );
    }
}
