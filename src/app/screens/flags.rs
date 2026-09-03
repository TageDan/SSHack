use std::{error::Error, option::Option};

use ratatui::{
    Frame,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Layout, Margin, Rect},
    style::{Style, Stylize},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState},
};
use ratatui_textarea::{Input, Key, TextArea};

use crate::{
    app::screens::leaderboard::LeaderboardScreen,
    conf::Conf,
    database::{Flag, User},
    screen::{Screen, draw_screen_border},
};

#[derive(PartialEq)]
enum BrowseScreenState {
    Browse,
    Submit,
}

pub struct BrowseScreen<'a> {
    state: BrowseScreenState,
    user: User,
    flags: Vec<Flag>,
    error: Option<String>,
    table_state: TableState,
    scroll: u16,
    submission: TextArea<'a>,
    filter: SearchFilter,
    conf: Conf,
}

impl Screen for BrowseScreen<'_> {
    fn handle_input(
        &mut self,
        key: Option<(KeyCode, KeyModifiers)>,
    ) -> Option<Box<dyn Screen + Send>> {
        // if no key is pressed, return early for now
        let key = key?;

        self.error = None;
        if self.filter.ui_active {
            if self.filter.handle_input(key) {
                self.reload();
            };
            return None;
        }
        match key {
            (KeyCode::Enter, _) => return self.submit(),
            (KeyCode::Esc, _) => return self.escape(),
            (KeyCode::Down, _) => self.focus_next(),
            (KeyCode::Up, _) => self.focus_prev(),
            (KeyCode::Char('r'), KeyModifiers::CONTROL) => return self.reload(),
            (KeyCode::Char('f'), KeyModifiers::CONTROL)
                if self.state == BrowseScreenState::Browse =>
            {
                self.filter.ui_active = true
            }
            (KeyCode::Right, KeyModifiers::CONTROL) if self.state == BrowseScreenState::Browse => {
                return Some(Box::new(LeaderboardScreen::new(
                    Some(self.user.clone()),
                    self.conf.clone(),
                )));
            }
            (k, m) if self.state == BrowseScreenState::Submit => {
                let event = KeyEvent::new(k, m);
                self.submission.input(event);
            }
            _ => (),
        };
        None
    }
    fn render(&mut self, f: &mut Frame) {
        let commands = match self.state {
            BrowseScreenState::Browse => {
                if self.filter.ui_active {
                    "^Q[QUIT] Esc[BACK] ⇥[KIND] ⇄[VALUE] ↵[APPLY]"
                } else {
                    "^Q[QUIT] ⇵[NAV] ↵[SELECT] ^R[RELOAD] ^⇄[TABS] ^F[FILTER]"
                }
            }
            BrowseScreenState::Submit => "^Q[QUIT] Esc[BACK] ⇵[SCROLL] ↵[SUBMIT] ^R[RELOAD]",
        };
        let area = draw_screen_border(
            f,
            if self.conf.about.is_some() {
                vec!["FLAGS", "LEADERBOARD", "ABOUT"]
            } else {
                vec!["FLAGS", "LEADERBOARD"]
            },
            0,
            commands,
            self.error.as_deref(),
            Some(&self.user),
            &self.conf,
        );
        let [col1, col2] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).areas(area);

        self.draw_table(f, col1);
        if let Err(e) = self.draw_preview(f, col2) {
            self.error = Some(e.to_string());
        };
        if self.filter.ui_active {
            self.filter.render(f, area, &self.conf);
        }
    }
}

impl BrowseScreen<'_> {
    fn submit(&mut self) -> Option<Box<dyn Screen + Send>> {
        match self.state {
            BrowseScreenState::Browse => {
                self.state = BrowseScreenState::Submit;
            }
            BrowseScreenState::Submit => {
                let Some(index) = self.table_state.selected() else {
                    self.error = Some("no flag selected to submit".to_string());
                    return None;
                };
                if let Some(flag) = self.flags.get(index) {
                    if flag.flag() != self.submission.lines()[0] {
                        self.error = Some("incorrect flag submitted".to_string());
                        return None;
                    }
                    if let Err(e) = flag.mark_solved_for_user(self.user.id()) {
                        self.error = Some(e.to_string());
                        return None;
                    };
                    self.submission = TextArea::default();
                    self.state = BrowseScreenState::Browse;
                    self.reload();
                }
            }
        }
        None
    }

    pub fn new(user: User, conf: Conf) -> Self {
        let (flags, error) = match Flag::get_all_with_user(&user) {
            Ok(flags) => (flags, None),
            Err(e) => (vec![], Some(e.to_string())),
        };

        Self {
            state: BrowseScreenState::Browse,
            table_state: TableState::default().with_selected(0),
            user,
            flags,
            error,
            scroll: 0,
            filter: SearchFilter::new(),
            submission: TextArea::default(),
            conf,
        }
    }

    fn focus_next(&mut self) {
        match self.state {
            BrowseScreenState::Browse => self.table_state.select_next(),
            BrowseScreenState::Submit => self.scroll = self.scroll.saturating_add(1),
        }
    }

    fn focus_prev(&mut self) {
        match self.state {
            BrowseScreenState::Browse => self.table_state.select_previous(),
            BrowseScreenState::Submit => self.scroll = self.scroll.saturating_sub(1),
        }
    }

    fn escape(&mut self) -> Option<Box<dyn Screen + Send>> {
        match self.state {
            BrowseScreenState::Submit => {
                self.scroll = 0;
                self.submission = TextArea::default();
                self.state = BrowseScreenState::Browse;
                return None;
            }
            _ => None,
        }
    }

    fn draw_table(&mut self, f: &mut Frame, a: Rect) {
        let header = ["Name", "Description", "Points", "Solved"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .fg(self.conf.theme.base08)
            .bg(self.conf.theme.base00)
            .italic()
            .bold()
            .height(1);

        let rows = self.flags.iter().enumerate().map(|(i, f)| {
            f.row_parts()
                .into_iter()
                .map(Cell::from)
                .collect::<Row>()
                .fg(self.conf.theme.base05)
                .bg(if i % 2 == 1 {
                    self.conf.theme.base00
                } else {
                    self.conf.theme.base01
                })
                .height(1)
        });

        let table = Table::new(
            rows,
            [
                Constraint::Fill(1),
                Constraint::Fill(2),
                Constraint::Fill(1),
                // seven for 'Solved' +1
                Constraint::Length(7),
            ],
        )
        .header(header)
        .row_highlight_style(
            Style::new()
                .fg(self.conf.theme.base05)
                .bg(self.conf.theme.base03),
        )
        .highlight_symbol(" >")
        .block(
            Block::new().borders(Borders::RIGHT).border_style(
                Style::new()
                    .fg(self.conf.theme.base01)
                    .bg(self.conf.theme.base00),
            ),
        );

        f.render_stateful_widget(table, a, &mut self.table_state);
    }

    fn draw_preview(&self, f: &mut Frame<'_>, a: Rect) -> Result<(), Box<dyn Error>> {
        let flag = self
            .flags
            .get(
                self.table_state
                    .selected()
                    .ok_or("failed to get selected flag")?,
            )
            .ok_or("failed to get selected flag")?;
        let [header, description, submission] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(3),
        ])
        .areas(a);

        let style1 = match self.state {
            BrowseScreenState::Browse => Style::new()
                .fg(self.conf.theme.base04)
                .bg(self.conf.theme.base00),
            BrowseScreenState::Submit => Style::new()
                .fg(self.conf.theme.base05)
                .bg(self.conf.theme.base00),
        };

        let style2 = match self.state {
            BrowseScreenState::Browse => Style::new()
                .fg(self.conf.theme.base04)
                .bg(self.conf.theme.base00),
            BrowseScreenState::Submit => Style::new()
                .fg(self.conf.theme.base08)
                .bg(self.conf.theme.base00),
        };

        let title = Paragraph::new(format!(
            "{}\nPoints - {}{}",
            flag.name(),
            flag.points(),
            if flag.solved() { " - SOLVED" } else { "" }
        ))
        .style(style2)
        .bold()
        .italic()
        .centered()
        .block(Block::new().borders(Borders::BOTTOM).border_style(style1));
        f.render_widget(title, header);

        let description_text = Paragraph::new(flag.description())
            .wrap(ratatui::widgets::Wrap { trim: false })
            .scroll((self.scroll, 0))
            .style(style1);

        f.render_widget(description_text, description);

        let block = Block::bordered().title_top("Submit Flag").style(style2);
        f.render_widget(&block, submission);
        f.render_widget(&self.submission, block.inner(submission));

        Ok(())
    }

    fn reload(&mut self) -> Option<Box<dyn Screen + Send>> {
        let _ = self.user.reload().is_err_and(|x| {
            self.error = Some(x.to_string());
            true
        });
        match Flag::get_all_with_user(&self.user) {
            Ok(flags) => match self.filter.search_kind {
                SearchFilterKind::None => self.flags = flags,
                SearchFilterKind::Solved(b) => {
                    self.flags = flags.into_iter().filter(|x| x.solved() == b).collect()
                }
                SearchFilterKind::Search(ref s) => {
                    self.flags = flags
                        .into_iter()
                        .filter(|x| x.name().contains(s) || x.description().contains(s))
                        .collect()
                }
            },
            Err(e) => self.error = Some(e.to_string()),
        };
        None
    }
}

struct SearchFilter {
    search_kind: SearchFilterKind,
    ui_active: bool,
}

impl SearchFilter {
    fn new() -> Self {
        Self {
            search_kind: SearchFilterKind::None,
            ui_active: false,
        }
    }

    fn handle_input(&mut self, key: (KeyCode, KeyModifiers)) -> bool {
        match key {
            (KeyCode::Enter, _) => {
                self.ui_active = false;
                return true;
            }
            (KeyCode::Esc, _) => {
                self.ui_active = false;
            }
            (KeyCode::BackTab, _) => match self.search_kind {
                SearchFilterKind::None => self.search_kind = SearchFilterKind::Solved(false),
                SearchFilterKind::Solved(_) => {
                    self.search_kind = SearchFilterKind::Search(String::new())
                }
                SearchFilterKind::Search(_) => self.search_kind = SearchFilterKind::None,
            },
            (KeyCode::Tab, _) => match self.search_kind {
                SearchFilterKind::None => {
                    self.search_kind = SearchFilterKind::Search(String::new())
                }
                SearchFilterKind::Search(_) => self.search_kind = SearchFilterKind::Solved(false),

                SearchFilterKind::Solved(_) => self.search_kind = SearchFilterKind::None,
            },
            (KeyCode::Char(c), _) => match self.search_kind {
                SearchFilterKind::Search(ref mut string) => string.push(c),
                _ => (),
            },
            (KeyCode::Backspace, _) => match self.search_kind {
                SearchFilterKind::Search(ref mut string) => {
                    string.pop();
                }
                _ => (),
            },
            (KeyCode::Left | KeyCode::Right, _) => match self.search_kind {
                SearchFilterKind::Solved(ref mut toggle) => {
                    *toggle = !*toggle;
                }
                _ => (),
            },
            _ => (),
        }
        false
    }

    fn render(&self, f: &mut Frame, area: Rect, conf: &Conf) {
        let [_, col, _] = Layout::horizontal([Constraint::Fill(1); 3]).areas(area);
        let [_, row, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(8),
            Constraint::Fill(1),
        ])
        .areas(col);
        f.render_widget(Clear, row);
        f.render_widget(
            Block::bordered()
                .title("Filter")
                .fg(conf.theme.base05)
                .bg(conf.theme.base01),
            row,
        );
        let area = row.inner(Margin::new(1, 1));
        let [_, kind, value, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Fill(1),
        ])
        .areas(area);
        match self.search_kind {
            SearchFilterKind::None => {
                f.render_widget(
                    Paragraph::new("None").block(Block::bordered().title("Kind")),
                    kind,
                );
            }
            SearchFilterKind::Solved(b) => {
                f.render_widget(
                    Paragraph::new("Solved").block(Block::bordered().title("Kind")),
                    kind,
                );
                f.render_widget(
                    Paragraph::new(format!("{}", b).as_str())
                        .block(Block::bordered().title("value"))
                        .fg(conf.theme.base08),
                    value,
                );
            }
            SearchFilterKind::Search(ref s) => {
                f.render_widget(
                    Paragraph::new("Search").block(Block::bordered().title("Kind")),
                    kind,
                );
                f.render_widget(
                    Paragraph::new(format!("{}", s).as_str())
                        .block(Block::bordered().title("value"))
                        .fg(conf.theme.base08),
                    value,
                );
            }
        }
    }
}

enum SearchFilterKind {
    None,
    Solved(bool),
    Search(String),
}
