use std::collections::HashMap;
use std::error::Error;
use std::io::{Read, Write};
use std::path::PathBuf;
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
use russh_sftp::protocol::{File, FileAttributes, Name, Status, StatusCode, Version};
use tokio::sync::Mutex;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

use crate::cli::Args;
use crate::conf::Conf;
use crate::{app::App, database};

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
pub struct AppServer {
    ctf_clients: Arc<Mutex<HashMap<usize, (SshTerminal, App)>>>,
    new_channels: Arc<Mutex<HashMap<ChannelId, Channel<Msg>>>>,
    id: usize,
    conf: Conf,
    key: Option<russh::keys::PublicKey>,
    args: Args,
}

impl AppServer {
    pub fn new(conf: Conf, args: Args) -> Self {
        Self {
            ctf_clients: Arc::new(Mutex::new(HashMap::new())),
            id: 0,
            key: None,
            new_channels: Arc::new(Mutex::new(HashMap::new())),
            conf,
            args,
        }
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        let clients = self.ctf_clients.clone();
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

        println!("Running on port {}!", self.conf.port);
        self.run_on_address(Arc::new(config), ("0.0.0.0", self.conf.port))
            .await?;
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

impl Server for AppServer {
    type Handler = Self;
    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Self {
        let s = self.clone();
        self.id += 1;
        s
    }
}

impl Handler for AppServer {
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

    async fn subsystem_request(
        &mut self,
        channel_id: ChannelId,
        name: &str,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        if name == "sftp" {
            if let Some(channel) = self.new_channels.lock().await.remove(&channel_id) {
                // Only allw users to get files (importnat for password protected ctfs)
                if self.key.is_none() || database::User::login(self.key.clone().unwrap()).is_none()
                {
                    session.channel_failure(channel_id)?;
                    return Ok(());
                }
                let sftp = SftpSession::default();
                session.channel_success(channel_id)?;
                russh_sftp::server::run(channel.into_stream(), sftp).await;
            }
        } else {
            session.channel_failure(channel_id)?
        }

        Ok(())
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let ctf_client = self.ctf_clients.lock().await.contains_key(&self.id);

        if ctf_client {
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
                    if let Event::Key(k) = terminput_crossterm::to_crossterm(ke)? { match (k.code, k.modifiers) {
                        (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
                            let mut clients = self.ctf_clients.lock().await;
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
                            let mut clients = self.ctf_clients.lock().await;
                            let (_, app) = clients.get_mut(&self.id).unwrap();
                            if let Some(s) = app.screen.handle_input(Some(k)) {
                                app.screen = s;
                            }
                        }
                    } };
                }
            }
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

        let mut clients = self.ctf_clients.lock().await;
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
        let app = App::new(
            self.conf.clone(),
            key.clone().ok_or(anyhow!("no public key"))?,
            self.args.clone(),
        );

        execute!(terminal.backend_mut(), EnterAlternateScreen, Hide)?;
        terminal.resize(rect)?;
        session.channel_success(channel.id())?;

        let mut clients = self.ctf_clients.lock().await;
        clients.insert(self.id, (terminal, app));

        Ok(())
    }
}

impl Drop for AppServer {
    fn drop(&mut self) {
        let id = self.id;
        let clients = self.ctf_clients.clone();
        tokio::spawn(async move {
            let mut clients = clients.lock().await;
            clients.remove(&id);
        });
    }
}

#[derive(Default)]
struct SftpSession {
    version: Option<u32>,
    root_dir_read_done: bool,
    file_read_done: bool,
}

impl russh_sftp::server::Handler for SftpSession {
    type Error = StatusCode;

    fn unimplemented(&self) -> Self::Error {
        StatusCode::OpUnsupported
    }

    async fn stat(
        &mut self,
        id: u32,
        path: String,
    ) -> Result<russh_sftp::protocol::Attrs, Self::Error> {
        if let Some(path) = get_file_path(&path)
            && path.is_file() {
                let mut attrs = FileAttributes::default();
                attrs.set_dir(false);
                attrs.set_symlink(false);
                attrs.set_regular(true);
                return Ok(russh_sftp::protocol::Attrs { id, attrs });
            }
        Err(StatusCode::NoSuchFile)
    }

    async fn fstat(
        &mut self,
        id: u32,
        handle: String,
    ) -> Result<russh_sftp::protocol::Attrs, Self::Error> {
        if let Some(path) = get_file_path(&handle)
            && path.is_file() {
                return Ok(russh_sftp::protocol::Attrs {
                    id,
                    attrs: FileAttributes::default(),
                });
            }
        Err(StatusCode::NoSuchFile)
    }

    async fn lstat(
        &mut self,
        id: u32,
        path: String,
    ) -> Result<russh_sftp::protocol::Attrs, Self::Error> {
        if let Some(path) = get_file_path(&path)
            && path.is_file() {
                let mut attrs = FileAttributes::default();
                attrs.set_dir(false);
                attrs.set_symlink(false);
                attrs.set_regular(true);
                return Ok(russh_sftp::protocol::Attrs { id, attrs });
            }
        Err(StatusCode::NoSuchFile)
    }

    async fn init(
        &mut self,
        version: u32,
        _extensions: HashMap<String, String>,
    ) -> Result<russh_sftp::protocol::Version, Self::Error> {
        if self.version.is_some() {
            return Err(StatusCode::ConnectionLost);
        }
        self.version = Some(version);
        Ok(Version::new())
    }

    async fn close(
        &mut self,
        id: u32,
        _handle: String,
    ) -> Result<russh_sftp::protocol::Status, Self::Error> {
        Ok(Status {
            id,
            status_code: StatusCode::Ok,
            error_message: "Ok".to_string(),
            language_tag: "en-US".to_string(),
        })
    }

    async fn open(
        &mut self,
        id: u32,
        filename: String,
        _pflags: russh_sftp::protocol::OpenFlags,
        _attrs: russh_sftp::protocol::FileAttributes,
    ) -> Result<russh_sftp::protocol::Handle, Self::Error> {
        if let Some(path) = get_file_path(&filename)
            && path.is_file() {
                self.file_read_done = false;

                return Ok(russh_sftp::protocol::Handle {
                    id,
                    handle: filename,
                });
            }
        Err(StatusCode::NoSuchFile)
    }

    async fn read(
        &mut self,
        id: u32,
        handle: String,
        _offset: u64,
        _len: u32,
    ) -> Result<russh_sftp::protocol::Data, Self::Error> {
        if !self.file_read_done {
            self.file_read_done = true;
            if let Some(p) = get_file_path(&handle) {
                let mut file = std::fs::File::open(p).map_err(|_| StatusCode::Failure)?;
                let mut data = vec![];
                file.read_to_end(&mut data)
                    .map_err(|_| StatusCode::Failure)?;
                return Ok(russh_sftp::protocol::Data { id, data });
            }
            return Err(StatusCode::NoSuchFile);
        }
        Err(StatusCode::Eof)
    }

    async fn realpath(&mut self, id: u32, _path: String) -> Result<Name, Self::Error> {
        Ok(Name {
            id,
            files: vec![File::dummy("/")],
        })
    }

    async fn opendir(
        &mut self,
        id: u32,
        path: String,
    ) -> Result<russh_sftp::protocol::Handle, Self::Error> {
        self.root_dir_read_done = false;
        Ok(russh_sftp::protocol::Handle { id, handle: path })
    }

    async fn readdir(&mut self, id: u32, handle: String) -> Result<Name, Self::Error> {
        if handle == "/" && !self.root_dir_read_done {
            self.root_dir_read_done = true;
            let files = get_all_files();
            return Ok(Name {
                id,
                files: files
                    .iter()
                    .map(|x| {
                        File::new(
                            x.file_name().unwrap().to_str().unwrap(),
                            FileAttributes::default(),
                        )
                    })
                    .collect(),
            });
        }
        // If all files have been sent to the client, respond with an EOF
        Err(StatusCode::Eof)
    }
}

fn get_file_path(path: &str) -> Option<PathBuf> {
    let mut new_path = std::env::home_dir()?;
    new_path.push(".config");
    new_path.push("sshack");
    new_path.push("files");
    let check = new_path.canonicalize().ok()?;
    new_path.push(path.trim_start_matches("/"));
    // path traversal
    if !new_path.canonicalize().ok()?.starts_with(check) {
        return None;
    };
    println!("{:?}", new_path);
    Some(new_path)
}

fn get_all_files() -> Vec<PathBuf> {
    let Some(mut new_path) = std::env::home_dir() else {
        return vec![];
    };
    new_path.push(".config");
    new_path.push("sshack");
    new_path.push("files");
    if let Ok(read) = new_path.read_dir() {
        read.filter(|x| x.as_ref().is_ok_and(|f| f.path().is_file()))
            .map(|x| x.unwrap().path())
            .collect()
    } else {
        vec![]
    }
}
