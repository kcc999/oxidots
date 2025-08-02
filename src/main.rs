use std::{fs::{self}, path::Path, sync::mpsc};

use clap::Parser;
use notify::{Event, Result, RecursiveMode, Watcher};

#[derive(Parser)]
struct Cli {
    config_file: String,
    user_dotfiles: String
}

pub fn verify(file_path: &str) -> bool {
    Path::new(file_path).is_dir()
}

pub fn initial_sync(watch_files: Vec<String>, user_dotfiles: &str) {
    for f in watch_files {
        let directory_name = f.split("/").last().unwrap();
        let new_path = user_dotfiles.to_owned() + directory_name;
        println!("Copying {:?} to {:?}", f.as_str(), new_path.as_str());
        let _ = fs::copy(f.as_str(), new_path.as_str());
    }
}

pub fn get_watch_files(config_file_path: &str) -> Vec<String> {
    let content = match fs::read_to_string(config_file_path) {
        Ok(content) => content,
        Err(_) => {
            panic!("Error reading config file");
        }
    };

    let watch_files: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    for file in watch_files.iter() {
        if !verify(file) {
            panic!("Error reading file");
        }
    }
    
    return watch_files;
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let watch_files: Vec<String> = get_watch_files(cli.config_file.as_str());

    let (tx, rx) = mpsc::channel::<Result<Event>>();

    let mut watcher = notify::recommended_watcher(tx)?;
    
    for file in watch_files.iter() {
        println!("Watching --> {:?}", file.as_str());
        watcher.watch(Path::new(file.as_str()), RecursiveMode::NonRecursive)?;
    }

    for res in rx {
        match res {
            Ok(event) => {
                if event.kind == notify::EventKind::Modify(notify::event::ModifyKind::Data(notify::event::DataChange::Any)) {
                    println!("Modified file: {:?}", event.paths.get(0));
                } 
            }
            Err(e) => println!("watch error: {:?}", e),
        }
    }

    Ok(())
}
