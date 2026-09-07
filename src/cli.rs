use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use toml::{from_str, to_string};

use crate::database::{self, Flag};

#[derive(Parser, Clone)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Clone)]
pub enum Commands {
    /// Run the server
    Run {
        /// run locally
        #[arg(short, long)]
        local: bool,
        /// open leaderboard page without logging in
        #[arg(long, default_value_t = false)]
        leaderboard: bool,
    },
    /// Add/Delete/List flags
    ///  (should be used before ctf starts as it will change flag id's and hence remove solves)
    // TODO: fix this
    Flags {
        #[command(subcommand)]
        command: FlagCommands,
    },
}

#[derive(Subcommand, Clone)]
pub enum FlagCommands {
    /// list flags in a tidy list
    List,
    /// delete a flag
    Delete {
        #[arg(short)]
        id: i32,
    },
    /// load json file with flags into database
    Load {
        /// Path to read from (default stdin)
        #[arg(short, long)]
        path: Option<PathBuf>,
    },
    /// print flags as json to file
    Write {
        /// Path to write to (default stdout)
        #[arg(short, long)]
        path: Option<PathBuf>,
    },
    /// add a new flag
    Add {
        /// name of the flag
        #[arg(short, long)]
        name: String,
        /// description of the flag (including links to files / services)
        #[arg(short, long)]
        description: String,
        /// points for solving the flag
        #[arg(short, long)]
        points: i32,
        /// solution string
        #[arg(short, long)]
        flag: String,
    },
}

#[derive(Serialize, Deserialize)]
struct Flags {
    flag: Vec<Flag>,
}

impl FlagCommands {
    pub fn run(&self) {
        match &self {
            FlagCommands::Delete { id } => database::delete_flag(*id).unwrap(),
            FlagCommands::List => {
                for flag in database::Flag::get_all().unwrap() {
                    println!(
                        "{} - {} - {} - {} - {}",
                        flag.id(),
                        flag.name(),
                        &flag.description()[..10.min(flag.description().len())],
                        flag.points(),
                        flag.flag()
                    )
                }
            }
            FlagCommands::Add {
                name,
                description,
                points,
                flag,
            } => database::create_flag(&name, &description, *points, &flag).unwrap(),
            FlagCommands::Write { path } => {
                let mut file: Box<dyn Write> = match path {
                    None => Box::new(std::io::stdout()),
                    Some(path) => {
                        let file = File::create(path).unwrap();
                        Box::new(file)
                    }
                };
                let flags = Flags {
                    flag: Flag::get_all().unwrap(),
                };
                let flags_string = to_string(&flags).unwrap();
                file.write_all(flags_string.as_bytes()).unwrap();
            }
            FlagCommands::Load { path } => {
                let mut file: Box<dyn Read> = match path {
                    None => Box::new(std::io::stdin()),
                    Some(path) => {
                        let file = File::open(path).unwrap();
                        Box::new(file)
                    }
                };
                let mut content = String::new();
                file.read_to_string(&mut content).unwrap();
                let flags: Flags = from_str(&content).unwrap();
                database::clear_flags();
                for flag in flags.flag {
                    database::create_flag(
                        flag.name(),
                        flag.description(),
                        flag.points(),
                        flag.flag(),
                    );
                }
            }
        }
    }
}

impl Args {
    pub fn leaderboard(&self) -> bool {
        match self.command {
            Commands::Run {
                local: _,
                leaderboard: true,
            } => true,
            _ => false,
        }
    }
}
