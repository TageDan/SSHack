use std::collections::HashMap;
use std::error::Error;
use std::io::{Read, Write};
use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::cursor::{Hide, Show};
use ratatui::crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::layout::Rect;
use ratatui::{Terminal, TerminalOptions, Viewport};
use russh::keys::ssh_key::rand_core::OsRng;
use russh::keys::ssh_key::{self, PublicKey};
use russh::server::*;
use russh::{Channel, ChannelId, Pty};
use tokio::sync::Mutex;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

use crate::admin_app::AdminApp;
use crate::cli::Args;
use crate::conf::Conf;

type SshTerminal = Terminal<CrosstermBackend<TerminalHandle>>;

struct TerminalHandle {
    sender: UnboundedSender<Vec<u8>>,
    // The sink collects the data which is finally sent to sender.
    sink: Vec<u8>,
}

impl TerminalHandle {
    async fn start(handle: Handle, channel_id: ChannelId) -> Self {
        let (sender, mut receiver) = unbounded_channel::<Vec<u8>>();
        tokio::spawn(async move {
            while let Some(data) = receiver.recv().await {
                let result = handle.data(channel_id, data.into()).await;
                if result.is_err() {
                    eprintln!("Failed to send data: {result:?}");
                }
            }
        });
        Self {
            sender,
            sink: Vec::new(),
        }
    }
}

// The crossterm backend writes to the terminal handle.
impl std::io::Write for TerminalHandle {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.sink.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let result = self.sender.send(self.sink.clone());
        if result.is_err() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                result.unwrap_err(),
            ));
        }

        self.sink.clear();
        Ok(())
    }
}

#[derive(Clone)]
pub struct AdminAppServer {
    clients: Arc<Mutex<HashMap<usize, (SshTerminal, AdminApp)>>>,
    new_channels: Arc<Mutex<HashMap<ChannelId, Channel<Msg>>>>,
    id: usize,
    conf: Conf,
    key: Option<russh::keys::PublicKey>,
    args: Args,
}

impl AdminAppServer {
    pub fn new(conf: Conf, args: Args) -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
            id: 0,
            key: None,
            new_channels: Arc::new(Mutex::new(HashMap::new())),
            conf,
            args,
        }
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let clients = self.clients.clone();
        // By introducing a interval we put a cap on the rendering rate whichhelps us reduce the bandwidth used over ssh, increasing performance.
        let mut interval = tokio::time::interval(Duration::from_millis(1000 / 60));
        tokio::spawn(async move {
            loop {
                interval.tick().await;
                for (_, (terminal, app)) in clients.lock().await.iter_mut() {
                    _ = terminal.draw(|f| {
                        app.render(f);
                    });

                    // Ugly fix, needs something better
                    if let Some(s) = app.screen.handle_input(None) {
                        app.screen = s;
                    }
                }
            }
        });

        let key = if let Some(k) = get_key() {
            k
        } else {
            let key =
                russh::keys::PrivateKey::random(&mut OsRng, ssh_key::Algorithm::Ed25519).unwrap();
            set_key(&key).unwrap();
            key
        };

        let config = Config {
            inactivity_timeout: Some(std::time::Duration::from_secs(3600)),
            auth_rejection_time: std::time::Duration::from_secs(3),
            auth_rejection_time_initial: Some(std::time::Duration::from_secs(0)),
            keys: vec![key],
            nodelay: true,
            ..Default::default()
        };

        if let Some(port) = self.conf.admin_port {
            println!("Running admin app on port {}!", port);
            self.run_on_address(Arc::new(config), ("0.0.0.0", port))
                .await?;
        }
        Ok(())
    }
}

fn get_key() -> Option<ssh_key::PrivateKey> {
    let mut path = std::env::home_dir()?;
    path.push(".sshack");
    path.push("priv_key");
    let mut file = std::fs::File::open(path).ok()?;
    let mut content = String::new();
    file.read_to_string(&mut content).ok()?;
    ssh_key::PrivateKey::from_openssh(content).ok()
}

fn set_key(key: &ssh_key::PrivateKey) -> Result<(), Box<dyn Error>> {
    let mut path = std::env::home_dir().ok_or("could not save file")?;
    path.push(".sshack");
    path.push("priv_key");
    let mut file = std::fs::File::create(path)?;
    let key = key.to_openssh(ssh_key::LineEnding::LF)?;
    file.write_all(key.as_bytes())?;
    Ok(())
}

impl Server for AdminAppServer {
    type Handler = Self;
    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Self {
        let s = self.clone();
        self.id += 1;
        s
    }
}

impl Handler for AdminAppServer {
    type Error = anyhow::Error;

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        self.new_channels.lock().await.insert(channel.id(), channel);

        Ok(true)
    }

    async fn auth_publickey(&mut self, _: &str, key: &PublicKey) -> Result<Auth, Self::Error> {
        self.key = Some(key.clone());
        Ok(Auth::Accept)
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let client = self.clients.lock().await.contains_key(&self.id);
        match client {
            true => {
                let mut current_event = Vec::new();
                let mut iter = data.iter().peekable();
                while let Some(b) = iter.next() {
                    current_event.push(*b);

                    if let Ok(Some(ke)) = terminput::Event::parse_from(&current_event) {
                        if ke.as_key().is_some_and(|x| {
                            x.code == terminput::KeyCode::Esc && iter.peek().is_some()
                        }) {
                            continue;
                        }
                        current_event.clear();
                        match terminput_crossterm::to_crossterm(ke)? {
                            Event::Key(k) => match (k.code, k.modifiers) {
                                (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                                    let mut clients = self.clients.lock().await;
                                    clients.remove(&self.id);

                                    // Restore terminal
                                    let mut data_out = Vec::new();
                                    execute!(data_out, LeaveAlternateScreen, Show)?;
                                    let res = session.handle().data(channel, data_out.into()).await;
                                    if res.is_err() {
                                        eprintln!("error restoring terminal")
                                    }

                                    let _ = session.handle().close(channel).await;
                                }
                                k => {
                                    let mut clients = self.clients.lock().await;
                                    let (_, app) = clients.get_mut(&self.id).unwrap();
                                    if let Some(s) = app.screen.handle_input(Some(k)) {
                                        app.screen = s;
                                    }
                                }
                            },
                            _ => (),
                        };
                    }
                }
            }

            false => (),
        }

        Ok(())
    }

    /// The client's window size has changed.
    async fn window_change_request(
        &mut self,
        _: ChannelId,
        col_width: u32,
        row_height: u32,
        _: u32,
        _: u32,
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        let rect = Rect {
            x: 0,
            y: 0,
            width: col_width as u16,
            height: row_height as u16,
        };

        let mut clients = self.clients.lock().await;
        let (terminal, _) = clients.get_mut(&self.id).unwrap();
        terminal.resize(rect)?;

        Ok(())
    }

    /// The client requests a pseudo-terminal with the given
    /// specifications.
    ///
    /// **Note:** Success or failure should be communicated to the client by calling
    /// `session.channel_success(channel)` or `session.channel_failure(channel)` respectively.
    async fn pty_request(
        &mut self,
        channel: ChannelId,
        _: &str,
        col_width: u32,
        row_height: u32,
        _: u32,
        _: u32,
        _: &[(Pty, u32)],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let rect = Rect {
            x: 0,
            y: 0,
            width: col_width as u16,
            height: row_height as u16,
        };
        let channel = self
            .new_channels
            .lock()
            .await
            .remove(&channel)
            .ok_or(anyhow!(
                "Could not get the channel for which a pty was requested"
            ))?;

        let terminal_handle = TerminalHandle::start(session.handle(), channel.id()).await;

        let backend = CrosstermBackend::new(terminal_handle);

        // the correct viewport area will be set when the client request a pty
        let options = TerminalOptions {
            viewport: Viewport::Fixed(Rect::default()),
        };

        let mut terminal = Terminal::with_options(backend, options)?;
        let key = self.key.clone();
        let app = AdminApp::new(
            self.conf.clone(),
            key.clone().ok_or(anyhow!("no public key"))?,
            self.args.clone(),
        );

        execute!(terminal.backend_mut(), EnterAlternateScreen, Hide)?;
        terminal.resize(rect)?;
        session.channel_success(channel.id())?;

        let mut clients = self.clients.lock().await;
        clients.insert(self.id, (terminal, app));

        Ok(())
    }
}

impl Drop for AdminAppServer {
    fn drop(&mut self) {
        let id = self.id;
        let clients = self.clients.clone();
        tokio::spawn(async move {
            let mut clients = clients.lock().await;
            clients.remove(&id);
        });
    }
}
