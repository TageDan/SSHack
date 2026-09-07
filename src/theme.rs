use std::io::Read;

use ratatui::style::Color;
use serde::Deserialize;

#[derive(Clone)]
#[allow(dead_code)]
pub struct Theme {
    pub base00: Color,
    pub base01: Color,
    pub base02: Color,
    pub base03: Color,
    pub base04: Color,
    pub base05: Color,
    pub base06: Color,
    pub base07: Color,
    pub base08: Color,
    pub base09: Color,
    pub base0a: Color,
    pub base0b: Color,
    pub base0c: Color,
    pub base0d: Color,
    pub base0e: Color,
    pub base0f: Color,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct ThemeStringRepresentation {
    pub base00: String,
    pub base01: String,
    pub base02: String,
    pub base03: String,
    pub base04: String,
    pub base05: String,
    pub base06: String,
    pub base07: String,
    pub base08: String,
    pub base09: String,
    pub base0A: String,
    pub base0B: String,
    pub base0C: String,
    pub base0D: String,
    pub base0E: String,
    pub base0F: String,
}
impl ThemeStringRepresentation {
    fn theme(&self) -> Theme {
        Theme {
            base00: Color::from_u32(u32::from_str_radix(&self.base00, 16).unwrap()),
            base01: Color::from_u32(u32::from_str_radix(&self.base01, 16).unwrap()),
            base02: Color::from_u32(u32::from_str_radix(&self.base02, 16).unwrap()),
            base03: Color::from_u32(u32::from_str_radix(&self.base03, 16).unwrap()),
            base04: Color::from_u32(u32::from_str_radix(&self.base04, 16).unwrap()),
            base05: Color::from_u32(u32::from_str_radix(&self.base05, 16).unwrap()),
            base06: Color::from_u32(u32::from_str_radix(&self.base06, 16).unwrap()),
            base07: Color::from_u32(u32::from_str_radix(&self.base07, 16).unwrap()),
            base08: Color::from_u32(u32::from_str_radix(&self.base08, 16).unwrap()),
            base09: Color::from_u32(u32::from_str_radix(&self.base09, 16).unwrap()),
            base0a: Color::from_u32(u32::from_str_radix(&self.base0A, 16).unwrap()),
            base0b: Color::from_u32(u32::from_str_radix(&self.base0B, 16).unwrap()),
            base0c: Color::from_u32(u32::from_str_radix(&self.base0C, 16).unwrap()),
            base0d: Color::from_u32(u32::from_str_radix(&self.base0D, 16).unwrap()),
            base0e: Color::from_u32(u32::from_str_radix(&self.base0E, 16).unwrap()),
            base0f: Color::from_u32(u32::from_str_radix(&self.base0F, 16).unwrap()),
        }
    }
}

impl Theme {
    pub fn new(theme: &str) -> Option<Self> {
        let mut path = std::env::home_dir()?;
        path.push(".config");
        path.push("sshack");
        path.push("themes");
        path.push(format!("{}.yaml", theme));
        let mut f = std::fs::File::open(path).ok()?;
        let mut content = String::new();
        f.read_to_string(&mut content).ok()?;
        let theme_string_repr: ThemeStringRepresentation =
            serde_yaml_ng::from_str(&content).ok()?;
        Some(theme_string_repr.theme())
    }
}

// Standard Theme (colors.yaml)
pub const STANDARD_THEME: Theme = Theme {
    base00: Color::from_u32(0x111111),
    base01: Color::from_u32(0x333333),
    base02: Color::from_u32(0x555555),
    base03: Color::from_u32(0x777777),
    base04: Color::from_u32(0x999999),
    base05: Color::from_u32(0xbbbbbb),
    base06: Color::from_u32(0xdddddd),
    base07: Color::from_u32(0xffffff),
    base08: Color::from_u32(0xff4136),
    base09: Color::from_u32(0xff851b),
    base0a: Color::from_u32(0xffdc00),
    base0b: Color::from_u32(0x2ecc40),
    base0c: Color::from_u32(0x7fdbff),
    base0d: Color::from_u32(0x0074d9),
    base0e: Color::from_u32(0xb10dc9),
    base0f: Color::from_u32(0x85144b),
};
