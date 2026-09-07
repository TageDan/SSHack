mod admin_app;
mod app;
mod cli;
mod conf;
mod database;
mod screen;
mod theme;

use std::error::Error;

use clap::Parser;

use crate::{
    cli::{Args, Commands},
    conf::Conf,
};

// inspired by: https://github.com/Eugeny/russh/blob/main/russh/examples/ratatui_app.rs
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    database::create_missing_db();
    let args = Args::parse();
    match args.command {
        Commands::Run { local, leaderboard } => {
            let conf = Conf::get();
            if !local {
                let args_1 = args.clone();
                let conf_1 = conf.clone();
                let task1 = tokio::task::spawn(async {
                    let mut server = app::server::AppServer::new(conf_1, args_1);
                    server.run().await.expect("Failed running server")
                });
                let task2 = tokio::task::spawn(async {
                    let mut server = admin_app::server::AdminAppServer::new(conf, args);
                    server.run().await.expect("Failed running server")
                });
                task1.await.expect("Failed running app server");
                task2.await.expect("Failed running admin server");
            } else {
                let mut term = ratatui::init();
                let res = app::app(&mut term, conf, leaderboard);
                ratatui::restore();
                return res;
            }
        }
        Commands::Flags { command } => command.run(),
    }

    Ok(())
}
