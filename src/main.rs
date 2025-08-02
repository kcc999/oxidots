use std::{fs::{self}, io, path::Path, sync::mpsc};

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

pub fn initial_sync(watch_files: &Vec<String>, user_dotfiles: &str) {
    for f in watch_files {
        let directory_name = f.split("/").last().unwrap();
        let new_path = user_dotfiles.to_owned() + directory_name;

        println!("Copying {:?} to {:?}", f.as_str(), new_path.as_str());
        let _ = copy_dir_all(Path::new(f.as_str()), Path::new(new_path.as_str()));
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
        println!("Verifying file {:?}", file);
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
    initial_sync(&watch_files, cli.user_dotfiles.as_str());
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
