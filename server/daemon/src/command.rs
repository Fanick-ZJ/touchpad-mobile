use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "daemon",
    about = "Start the daemon",
    version = "1.0.0",
    author = "Fanick_zz"
)]
pub struct Command {
    #[arg(short = 'c', long = "config", help = "Path to config file")]
    pub config_file: PathBuf,
}
