use std::{fs, io, path::Path};

pub fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
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
        let src_path = Path::new(f);
        let dir_name = src_path.file_name().unwrap();
        let dst_path = Path::new(user_dotfiles).join(dir_name);

        let _ = copy_dir_all(src_path, dst_path);
    }
}

