mod cli;
mod config;
mod server;
mod status_types;

use std::fs;

use clap::Parser;

use crate::cli::{Cli, Commands};
use crate::server::Server;

fn main() {
    let cli = Cli::parse();
    let toml_path = dirs::home_dir().unwrap().join("jp_tool_status.toml");
    let config_dir = dirs::home_dir().unwrap();
    let mut config = crate::config::Config::create_or_load(toml_path);

    match cli.commands {
        Commands::Watch {
            file_path,
            repo_name,
        } => {
            config.watch_file(fs::canonicalize(file_path).unwrap(), repo_name);
        }
        Commands::ListAll => {
            config.list_all();
        }
        Commands::Remove { repo_name } => config.remove(repo_name),
        Commands::Start => {
            let mut server = Server::new(config, config_dir);
            if let Err(e) = server.start() {
                eprintln!("Failed to start server: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Stop => {
            let server = Server::new(config, config_dir);
            if let Err(e) = server.stop() {
                eprintln!("Failed to stop server: {}", e);
                std::process::exit(1);
            }
        }
    }
}
