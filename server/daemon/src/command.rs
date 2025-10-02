use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "daemon",
    about = "Start the daemon",
    version = "1.0.0",
    author = "Fanick_zz"
)]
pub struct Command {
    #[arg(
        short = 'p',
        long = "port",
        default_value_t = 8521,
        help = "Port to listen on"
    )]
    pub port: u16,
    #[arg(
        short = 'l',
        long = "log",
        default_value_t = String::from("info"),
        help = "Log level"
    )]
    pub log_level: String,
    #[arg(short = 'd', long = "debug", help = "Enable debug mode")]
    pub debug: bool,
    #[arg(
        short = 'm',
        long = "model",
        help = "Model to use, daemon, ui",
        default_value_t = String::from("daemon")
    )]
    pub model: String,
}
