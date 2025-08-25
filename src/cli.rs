use clap::Parser;

#[derive(Parser, Debug)]
pub struct Cli {
    pub config_file: String,
    pub user_dotfiles: String,
    #[arg(long, help = "Run with systemd integration (sd_notify)")]
    pub systemd: bool,
}
