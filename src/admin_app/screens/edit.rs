use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Layout},
    style::Style,
    widgets::Block,
};
use ratatui_textarea::TextArea;

use crate::{
    admin_app::screens::flags::AdminScreen,
    conf::Conf,
    database::{self, Flag},
    screen::{Screen, draw_screen_border},
};

#[derive(PartialEq, PartialOrd)]
enum EditState {
    Navigation,
    Focused,
}

pub struct EditScreen<'a> {
    flag: Option<Flag>,
    focus: usize,
    title: TextArea<'a>,
    points: TextArea<'a>,
    description: TextArea<'a>,
    flag_string: TextArea<'a>,
    state: EditState,
    conf: Conf,
    error: Option<String>,
}

impl EditScreen<'_> {
    pub fn new(flag: Option<Flag>, conf: Conf) -> Self {
        Self {
            flag: flag.clone(),
            focus: 0,
            title: TextArea::new(vec![
                flag.as_ref()
                    .map(|x| x.name())
                    .unwrap_or_default()
                    .to_string(),
            ]),
            points: TextArea::new(vec![format!(
                "{}",
                flag.as_ref().map(|x| x.points()).unwrap_or_default()
            )]),
            description: TextArea::new(vec![
                flag.as_ref()
                    .map(|x| x.description())
                    .unwrap_or_default()
                    .to_string(),
            ]),
            flag_string: TextArea::new(vec![
                flag.as_ref()
                    .map(|x| x.flag())
                    .unwrap_or_default()
                    .to_string(),
            ]),
            state: EditState::Navigation,
            conf,
            error: None,
        }
    }

    fn submit(&mut self) -> Option<Box<dyn Screen + Send>> {
        self.state = match self.state {
            EditState::Navigation => EditState::Focused,
            EditState::Focused => {
                self.focus_next();
                EditState::Navigation
            }
        };
        None
    }

    fn escape(&mut self) -> Option<Box<dyn Screen + Send>> {
        match self.state {
            EditState::Navigation => Some(Box::new(AdminScreen::new(self.conf.clone()))),
            EditState::Focused => {
                self.state = EditState::Navigation;
                None
            }
        }
    }

    fn focus_next(&mut self) -> Option<Box<dyn Screen + Send>> {
        self.focus = self.focus.saturating_add(1).min(3);
        None
    }

    fn focus_prev(&mut self) -> Option<Box<dyn Screen + Send>> {
        self.focus = self.focus.saturating_sub(1);
        None
    }

    fn save(&mut self) -> Option<Box<dyn Screen + Send>> {
        let points = self.points.lines().iter().fold(String::new(), |x, a| x + a);
        let points = points
            .parse::<i32>()
            .or_else(|e| {
                self.error = Some(e.to_string());
                Err(e)
            })
            .ok()?;
        if let Some(f) = self.flag.as_ref() {
            if let Err(e) = database::update_flag(
                &f.id(),
                &self.title.lines().iter().fold(String::new(), |x, a| x + a),
                &self
                    .description
                    .lines()
                    .iter()
                    .fold(String::new(), |x, a| x + a),
                points,
                &self
                    .flag_string
                    .lines()
                    .iter()
                    .fold(String::new(), |x, a| x + a),
            ) {
                self.error = Some(e.to_string());
                return None;
            };
        } else {
            if let Err(e) = database::create_flag(
                &self.title.lines().iter().fold(String::new(), |x, a| x + a),
                &self
                    .description
                    .lines()
                    .iter()
                    .fold(String::new(), |x, a| x + a),
                points,
                &self
                    .flag_string
                    .lines()
                    .iter()
                    .fold(String::new(), |x, a| x + a),
            ) {
                self.error = Some(e.to_string());
                return None;
            };
        }
        Some(Box::new(AdminScreen::new(self.conf.clone())))
    }
}

impl Screen for EditScreen<'_> {
    fn handle_input(
        &mut self,
        key: Option<(
            ratatui::crossterm::event::KeyCode,
            ratatui::crossterm::event::KeyModifiers,
        )>,
    ) -> Option<Box<dyn Screen + Send>> {
        let key = key?;
        self.error = None;
        match key {
            (KeyCode::Char('s'), KeyModifiers::CONTROL) if !(self.state == EditState::Focused) => {
                return self.save();
            }
            (KeyCode::Enter, _) if !(self.state == EditState::Focused && self.focus == 2) => {
                return self.submit();
            }
            (KeyCode::Esc, _) => return self.escape(),
            (KeyCode::Down, _) if self.state == EditState::Navigation => self.focus_next(),
            (KeyCode::Up, _) if self.state == EditState::Navigation => self.focus_prev(),
            (k, m) if self.state == EditState::Focused => {
                let event = KeyEvent::new(k, m);
                let _ = match self.focus {
                    0 => self.title.input(event),
                    1 => self.points.input(event),
                    2 => self.description.input(event),
                    3 => self.flag_string.input(event),
                    _ => true,
                };
                None
            }
            _ => None,
        }
    }
    fn render(&mut self, f: &mut ratatui::Frame) {
        let area = draw_screen_border(
            f,
            vec!["FLAGS", "EDIT"],
            1,
            match self.state {
                EditState::Navigation => "^Q[QUIT] Esc[BACK] ⇵[NAV] ↵[FOCUS] ^S[SAVE]",
                EditState::Focused => "^Q[QUIT] Esc[BACK]",
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

        let [title, points, description, flag_string] = if col.height >= 15 {
            Layout::vertical([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(4),
                Constraint::Length(3),
            ])
            .vertical_margin(1)
            .areas(col)
        } else {
            Layout::vertical([
                if self.focus == 0 {
                    Constraint::Length(3)
                } else {
                    Constraint::Length(0)
                },
                if self.focus == 0 || self.focus == 1 {
                    Constraint::Length(3)
                } else {
                    Constraint::Length(0)
                },
                if self.focus == 1 || self.focus == 2 || self.focus == 3 {
                    Constraint::Min(4)
                } else {
                    Constraint::Length(0)
                },
                if self.focus == 2 || self.focus == 3 {
                    Constraint::Length(3)
                } else {
                    Constraint::Length(0)
                },
            ])
            .vertical_margin(1)
            .areas(col)
        };

        let color1 = Style::new()
            .fg(self.conf.theme.base08)
            .bg(self.conf.theme.base00);
        let color2 = Style::new()
            .fg(self.conf.theme.base05)
            .bg(self.conf.theme.base00);

        // TITLE
        let block = Block::bordered()
            .title_top("TITLE")
            .style(if self.focus == 0 { color1 } else { color2 });

        f.render_widget(&block, title);

        f.render_widget(&self.title, block.inner(title));

        // POINTS
        let block = Block::bordered()
            .title_top("POINTS")
            .style(if self.focus == 1 { color1 } else { color2 });

        f.render_widget(&block, points);

        f.render_widget(&self.points, block.inner(points));

        // DESCRIPTION
        let block = Block::bordered()
            .title_top("DESCRIPTION")
            .style(if self.focus == 2 { color1 } else { color2 });

        f.render_widget(&block, description);

        f.render_widget(&self.description, block.inner(description));
        let block = Block::bordered()
            .title_top("FLAG")
            .style(if self.focus == 3 { color1 } else { color2 });

        f.render_widget(&block, flag_string);

        f.render_widget(&self.flag_string, block.inner(flag_string));
    }
}
