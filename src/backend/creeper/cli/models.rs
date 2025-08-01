use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    #[arg(short, long)]
    game_dir: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    Launch {
        #[arg(short, long)]
        version: Option<String>,
        #[arg(long)]
        offline: bool,
    },
    List,
    Update,
    Interactive,
    Delete,
}
