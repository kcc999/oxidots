mod cli;
mod config;
mod fs_ops;
mod git_ops;
mod logging;
mod systemd;
mod watcher;

use clap::Parser;
use notify::Result;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    logging::init_logger(cli.systemd);

    let watch_dirs = config::get_watch_dirs(cli.config_file.as_str());

    fs_ops::initial_sync(&watch_dirs, cli.user_dotfiles.as_str());

    if cli.systemd {
        // If the watchdog is configured (WATCHDOG_USEC present), start ping thread
        systemd::maybe_start_watchdog();
    }

    watcher::watch(watch_dirs, cli.user_dotfiles.as_str(), cli.systemd)
}
