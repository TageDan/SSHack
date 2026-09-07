use std::time::Instant;

use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    layout::{Constraint, Layout},
    style::Stylize,
    text::Text,
    widgets::Paragraph,
};
use tachyonfx::{
    CellFilter, Effect, EffectTimer,
    fx::{self},
};

use crate::{
    app::screens::flags::BrowseScreen,
    conf::Conf,
    screen::{Screen, draw_screen_border},
};
use crate::{app::screens::register::RegisterScreen, database::User};

pub struct HomeScreen {
    conf: Conf,
    key: russh::keys::PublicKey,
    last_time: Instant,
    effect_text: Effect,
}

impl Screen for HomeScreen {
    fn handle_input(
        &mut self,
        _key: Option<(KeyCode, KeyModifiers)>,
    ) -> Option<Box<dyn Screen + Send>> {
        if self.effect_text.done() {
            match User::login(self.key.clone()) {
                Some(u) => Some(Box::new(BrowseScreen::new(u, self.conf.clone()))),
                None => Some(Box::new(RegisterScreen::new(
                    self.conf.clone(),
                    self.key.clone(),
                ))),
            }
        } else {
            None
        }
    }
    fn render(&mut self, f: &mut ratatui::Frame) {
        let area = draw_screen_border(
            f,
            vec!["HOME"],
            0,
            "WELCOME TO SSHACK",
            None,
            None,
            &self.conf,
        );

        let [_, banner_part, _] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Max(self.conf.banner.lines().count() as u16 + 1),
            Constraint::Fill(1),
        ])
        .areas(area);

        let [_, banner, _] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(
                self.conf
                    .banner
                    .lines()
                    .map(|x| x.chars().count())
                    .max()
                    .unwrap_or(0) as u16,
            ),
            Constraint::Fill(1),
        ])
        .areas(banner_part);
        f.render_widget(
            Paragraph::new(Text::raw(&self.conf.banner))
                .fg(self.conf.theme.base08)
                .bg(self.conf.theme.base00),
            banner,
        );

        let delta_time = self.last_time.elapsed();
        self.last_time = Instant::now();

        // In your render loop
        self.effect_text
            .process(delta_time.into(), f.buffer_mut(), banner);
    }
}

impl HomeScreen {
    pub fn new(conf: Conf, key: russh::keys::PublicKey) -> Self {
        let text = if conf.animation {
            let text1 = fx::slide_in(
                tachyonfx::Motion::LeftToRight,
                10,
                0,
                conf.theme.base00,
                EffectTimer::from_ms(500, tachyonfx::Interpolation::QuadInOut),
            )
            .with_filter(CellFilter::NonEmpty);

            let text2 = fx::slide_out(
                tachyonfx::Motion::LeftToRight,
                10,
                0,
                conf.theme.base00,
                EffectTimer::from_ms(500, tachyonfx::Interpolation::QuadInOut),
            )
            .with_filter(CellFilter::NonEmpty);

            fx::sequence(&[text1, fx::sleep(1000), text2])
        } else {
            fx::sleep(1000)
        };

        Self {
            conf,
            key,
            last_time: Instant::now(),
            effect_text: text,
        }
    }
}
