use std::io::Read;

use serde::Deserialize;
use toml::from_str;

use crate::theme::{STANDARD_THEME, Theme};

#[derive(Clone)]
pub struct Conf {
    pub theme: Theme,
    pub banner: String,
    pub port: u16,
    pub animation: bool,
    pub password: Option<String>,
    pub about: Option<String>,
    pub admin_password: Option<String>,
    pub admin_port: Option<u16>,
}

#[derive(Deserialize)]
struct ConfToml {
    theme: String,
    banner: String,
    port: u16,
    animation: Option<bool>,
    password: Option<String>,
    admin_password: Option<String>,
    admin_port: Option<u16>,
    about: Option<String>,
}
impl ConfToml {
    fn conf(self) -> Option<Conf> {
        Some(Conf {
            theme: Theme::new(&self.theme)?,
            banner: self.banner,
            port: self.port,
            animation: self.animation.unwrap_or(true),
            password: self.password,
            admin_password: self.admin_password,
            about: self.about,
            admin_port: self.admin_port,
        })
    }
}

impl Conf {
    pub fn get() -> Self {
        Self::get_from_file().unwrap_or(Self {
            theme: Theme::new("colors").unwrap_or(STANDARD_THEME),
            banner: r#"
   __________ __         __  
  / __/ __/ // /__ _____/ /__
 _\ \_\ \/ _  / _ `/ __/  '_/
/___/___/_//_/\_,_/\__/_/\_\
"#
            .to_string(),
            port: 1337,
            about: Some(
                "
This CTF was built using the SSHack ctf framework: https://github.com/d-z0n/SSHack 
                "
                .to_string(),
            ),
            animation: true,
            password: None,
            admin_password: Some("unPWNable!".to_string()),
            admin_port: Some(13373),
        })
    }

    fn get_from_file() -> Option<Self> {
        let mut file = std::env::home_dir()?;
        file.push(".config");
        file.push("sshack");
        file.push("config.toml");
        let mut f = std::fs::File::open(file).ok()?;
        let mut content = String::new();
        f.read_to_string(&mut content).ok()?;
        let conf_str: ConfToml = from_str(&content).ok()?;
        Some(conf_str.conf()?)
    }
}
