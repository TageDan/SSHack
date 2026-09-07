use ratatui::Frame;
use ratatui::crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Stylize;
use ratatui::widgets::{Block, Paragraph};

use crate::conf::Conf;
use crate::database::User;

pub fn draw_screen_border(
    f: &mut Frame,
    titles: Vec<&'static str>,
    current_title: usize,
    commands: &'static str,
    error: Option<&str>,
    user: Option<&User>,
    conf: &Conf,
) -> Rect {
    let area = f.area();
    let [top_bar, body, command_bar, error_bar] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(area);
    if let Some(e) = error {
        f.render_widget(
            Paragraph::new(e)
                .fg(ratatui::style::Color::Red)
                .bold()
                .bg(conf.theme.base00),
            error_bar,
        );
    } else {
        f.render_widget(Paragraph::new("").bg(conf.theme.base00), error_bar);
    }

    let version = " SSHack v".to_owned() + env!("CARGO_PKG_VERSION") + " ";
    let [commands_box, version_box] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(version.len() as u16),
    ])
    .areas(command_bar);

    f.render_widget(
        Paragraph::new(commands)
            .fg(conf.theme.base04)
            .bg(conf.theme.base01),
        commands_box,
    );

    f.render_widget(
        Paragraph::new(version)
            .fg(conf.theme.base04)
            .bg(conf.theme.base01),
        version_box,
    );

    // fill background
    f.render_widget(Block::new().bg(conf.theme.base00), body);
    f.render_widget(Block::new().bg(conf.theme.base01), top_bar);

    let [title_rect, rest] = Layout::horizontal([
        Constraint::Length(titles.iter().map(|x| x.len() + 2).sum::<usize>() as u16), // plus 2 for padding
        Constraint::Fill(1),
    ])
    .areas(top_bar);

    let constraints = titles
        .iter()
        .map(|x| Constraint::Length(x.len() as u16 + 2));

    let parts = Layout::horizontal(constraints).split(title_rect);

    for (i, title) in titles.into_iter().enumerate() {
        f.render_widget(
            Paragraph::new(format!(" {} ", title))
                .fg(if current_title == i {
                    conf.theme.base05
                } else {
                    conf.theme.base04
                })
                .bg(if current_title == i {
                    conf.theme.base02
                } else {
                    conf.theme.base01
                }),
            parts[i],
        );
    }

    if let Some(u) = user {
        let points = format!(" {} points ", u.points().to_string());
        let username = format!(" {} ", u.name());

        let [_, user_rect, point_rect] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(username.len() as u16),
            Constraint::Length(points.len() as u16),
        ])
        .areas(rest);

        f.render_widget(
            Paragraph::new(username)
                .fg(conf.theme.base05)
                .bg(conf.theme.base02),
            user_rect,
        );
        f.render_widget(
            Paragraph::new(points)
                .fg(conf.theme.base05)
                .bg(conf.theme.base01),
            point_rect,
        );
    }
    body
}

pub trait Screen {
    fn handle_input(
        &mut self,
        key: Option<(KeyCode, KeyModifiers)>,
    ) -> Option<Box<dyn Screen + Send>>;
    fn render(&mut self, f: &mut Frame);
}
