mod systemd;
mod watcher;

use clap::Parser;
use git2::{IndexAddOption, Repository};
use log::LevelFilter;
use notify::Result;
use simplelog::{Config as LogConfig, SimpleLogger, WriteLogger};
use std::fs;
use std::io;
use std::path::Path;

#[derive(Parser, Debug)]
struct Cli {
    config_file: String,
    user_dotfiles: String,
    #[arg(long, help = "Run with systemd integration (sd_notify)")]
    systemd: bool,
}

fn init_logger(systemd: bool) {
    if systemd {
        SimpleLogger::init(LevelFilter::Info, LogConfig::default()).unwrap();
    } else {
        WriteLogger::init(
            LevelFilter::Info,
            LogConfig::default(),
            fs::File::create("~.oxidots.log").unwrap(),
        )
        .unwrap();
    }
}

fn verify(file_path: &str) -> bool {
    Path::new(file_path).is_dir()
}

fn get_watch_dirs(config_file_path: &str) -> Vec<String> {
    let content = match fs::read_to_string(config_file_path) {
        Ok(content) => content,
        Err(e) => {
            log::error!("Error reading config file {:?}", e);
            panic!("Error reading config file, see log file for details");
        }
    };

    let watch_files: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    for file in watch_files.iter() {
        log::info!("Verifying file {:?}", file);
        if !verify(file) {
            log::error!("Error reading file {:?}", file);
        }
    }

    watch_files
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let entry_type = entry.file_type()?;

        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if entry_type.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn initial_sync(watch_files: &Vec<String>, user_dotfiles: &str) {
    for f in watch_files {
        let src_path = Path::new(f);
        let dir_name = src_path.file_name().unwrap();
        let dst_path = Path::new(user_dotfiles).join(dir_name);

        let _ = copy_dir_all(src_path, dst_path);
    }
}

pub fn git_sync(user_dotfiles: &str) {
    let repo = match Repository::open(user_dotfiles) {
        Ok(repo) => repo,
        Err(e) => panic!("Failed to open: {}", e),
    };

    let mut index = repo.index().unwrap();

    let _ = index.add_all(["."].iter(), IndexAddOption::DEFAULT, None);
    let _ = index.write();

    println!("DEBUG: Index has {} entries", index.len());

    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();

    let parent_commit = match repo.head() {
        Ok(head) => {
            let target = head.target().unwrap();
            Some(repo.find_commit(target).unwrap())
        }
        Err(_) => None, // No previous commits (initial commit)
    };

    let message: &str = "Oxidots: update";

    let commit = repo.commit(
        Some("HEAD"),
        &repo.signature().unwrap(),
        &repo.signature().unwrap(),
        message,
        &tree,
        &[&parent_commit.unwrap()], // @TODO: Will panic if no parent commit
    );

    println!("DEBUG COMMIT {:?}", commit);
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    init_logger(cli.systemd);

    let watch_dirs = get_watch_dirs(cli.config_file.as_str());

    initial_sync(&watch_dirs, cli.user_dotfiles.as_str());

    if cli.systemd {
        systemd::maybe_start_watchdog();
    }

    watcher::watch(watch_dirs, cli.user_dotfiles.as_str(), cli.systemd)
}
