use std::{fs, path::Path};

pub fn verify(file_path: &str) -> bool {
    Path::new(file_path).is_dir()
}

pub fn get_watch_dirs(config_file_path: &str) -> Vec<String> {
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

