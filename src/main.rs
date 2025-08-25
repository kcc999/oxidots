mod systemd;
mod watcher;

use clap::Parser;
use git2::{IndexAddOption, Repository, Signature, StatusOptions};
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
        // Handle paths that may end with a trailing separator where file_name() is None
        let dir_name = src_path
            .file_name()
            .or_else(|| {
                use std::path::Component;
                src_path
                    .components()
                    .rev()
                    .find_map(|c| match c {
                        Component::Normal(os) => Some(os),
                        _ => None,
                    })
            })
            .unwrap_or_default();
        let dst_path = Path::new(user_dotfiles).join(dir_name);

        let _ = copy_dir_all(src_path, dst_path);
    }
}

fn ensure_repo(path: &str) -> Repository {
    if !Path::new(path).exists() {
        let _ = fs::create_dir_all(path);
    }
    match Repository::open(path) {
        Ok(repo) => repo,
        Err(_) => Repository::init(path).expect("Failed to init git repo"),
    }
}

pub fn git_sync(user_dotfiles: &str) {
    let repo = ensure_repo(user_dotfiles);

    // Check for working directory changes before staging
    let mut status_opts = StatusOptions::new();
    status_opts
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false);

    let statuses = repo.statuses(Some(&mut status_opts)).unwrap();
    log::debug!(
        "git status entries: {} (wd: {:?})",
        statuses.len(),
        repo.workdir()
    );
    if statuses.len() == 0 {
        log::info!("No changes detected in working directory; skipping commit");
        return;
    }

    let mut index = repo.index().unwrap();
    let _ = index.add_all(["."].iter(), IndexAddOption::DEFAULT | IndexAddOption::FORCE, None);
    let _ = index.write();

    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();

    let mut parents: Vec<git2::Commit> = Vec::new();
    if let Ok(head) = repo.head() {
        if let Some(target) = head.target() {
            if let Ok(commit) = repo.find_commit(target) {
                parents.push(commit);
            }
        }
    }

    let message: &str = "Oxidots: update";

    let sig = match repo.signature() {
        Ok(s) => s,
        Err(_) => match Signature::now("Oxidots", "oxidots@localhost") {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to create signature: {:?}", e);
                return;
            }
        },
    };

    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
    let commit = repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        message,
        &tree,
        &parent_refs,
    );

    println!("DEBUG COMMIT {:?}", commit);
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    init_logger(cli.systemd);

    let watch_dirs = get_watch_dirs(cli.config_file.as_str());

    initial_sync(&watch_dirs, cli.user_dotfiles.as_str());
    // Create an initial snapshot commit so subsequent updates are diffs
    git_sync(cli.user_dotfiles.as_str());

    if cli.systemd {
        systemd::maybe_start_watchdog();
    }

    watcher::watch(watch_dirs, cli.user_dotfiles.as_str(), cli.systemd)
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Repository, Signature};
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    fn write_file(path: &Path, content: &str) {
        let mut f = File::create(path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    fn init_repo_with_initial_commit(path: &Path) -> Repository {
        let repo = Repository::init(path).unwrap();
        {
            let mut cfg = repo.config().unwrap();
            cfg.set_str("user.name", "Oxidots Test").unwrap();
            cfg.set_str("user.email", "test@example.com").unwrap();
        }

        // Create an initial empty commit
        {
            let mut index = repo.index().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            let sig = repo.signature().unwrap_or_else(|_| {
                Signature::now("Oxidots Test", "test@example.com").unwrap()
            });
            let _ = repo
                .commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
                .unwrap();
            // tree dropped at end of this block before moving repo
        }
        repo
    }

    #[test]
    fn verify_checks_directory() {
        let t = TempDir::new().unwrap();
        assert!(verify(t.path().to_str().unwrap()));
        let missing = t.path().join("missing");
        assert!(!verify(missing.to_str().unwrap()));
    }

    #[test]
    fn copy_and_initial_sync_copies_contents() {
        let src_root = TempDir::new().unwrap();
        let nested = src_root.path().join("nvim").join("lua");
        fs::create_dir_all(&nested).unwrap();
        let file_a = nested.join("init.lua");
        write_file(&file_a, "print('hello')\n");

        let dst_repo_dir = TempDir::new().unwrap();
        let watch_files = vec![src_root.path().to_str().unwrap().to_string()];

        initial_sync(&watch_files, dst_repo_dir.path().to_str().unwrap());

        let mirrored = dst_repo_dir
            .path()
            .join(src_root.path().file_name().unwrap())
            .join("nvim")
            .join("lua")
            .join("init.lua");
        let content = fs::read_to_string(mirrored).unwrap();
        assert!(content.contains("hello"));
    }

    #[test]
    fn git_sync_creates_new_commit() {
        let repo_dir = TempDir::new().unwrap();
        let repo = init_repo_with_initial_commit(repo_dir.path());

        // Write a file to the workdir
        let wd = repo.workdir().unwrap();
        let f = wd.join("README.md");
        write_file(&f, "test\n");

        let before = repo.head().unwrap().target().unwrap();
        git_sync(repo_dir.path().to_str().unwrap());
        let after = repo.head().unwrap().target().unwrap();
        assert_ne!(before, after, "HEAD should advance after git_sync");
    }

    #[test]
    fn git_sync_initializes_missing_repo() {
        let repo_dir = TempDir::new().unwrap();
        // No repo initialized here on purpose
        // Create a file so there is something to commit after init
        let f = repo_dir.path().join("test.txt");
        write_file(&f, "hello\n");

        // Should create repo and commit
        git_sync(repo_dir.path().to_str().unwrap());

        let repo = Repository::open(repo_dir.path()).unwrap();
        assert!(repo.head().is_ok(), "HEAD should exist after initial sync/commit");
    }
}
