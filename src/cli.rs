use clap::{Parser, Subcommand};

/// CLI Tool to send juspay tool data to dashboard
#[derive(Parser, Debug)]
pub struct Cli {
    #[command(subcommand)]
    pub commands: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Add a repo/ status file to watch list
    Watch {
        file_path: String,

        #[arg(short, long)]
        repo_name: Option<String>,
    },

    /// List all repos being watched
    ListAll,

    /// Remove a repo from being watched
    Remove { repo_name: String },

    /// Starts a server that hits the dashboard every 10 seconds by default
    Start,

    /// Stops the running server
    Stop,
}
