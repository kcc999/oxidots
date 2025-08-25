use notify::{Event, Result, RecursiveMode, Watcher};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use crate::git_sync;
use crate::systemd;

fn mirror_modified_file(
    src_file: &Path,
    watch_dirs: &[String],
    user_dotfiles: &str,
) -> std::io::Result<Option<PathBuf>> {
    // Find which watch_dir this file belongs to
    for d in watch_dirs {
        let d_path = Path::new(d);
        if src_file.starts_with(d_path) {
            // destination base is <user_dotfiles>/<basename(d)>
            let base_name = d_path
                .file_name()
                .or_else(|| {
                    use std::path::Component;
                    d_path
                        .components()
                        .rev()
                        .find_map(|c| match c {
                            Component::Normal(os) => Some(os),
                            _ => None,
                        })
                })
                .unwrap_or_default();

            let rel = src_file.strip_prefix(d_path).unwrap_or(src_file);
            let dst = Path::new(user_dotfiles).join(base_name).join(rel);
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(src_file, &dst)?;
            return Ok(Some(dst));
        }
    }
    Ok(None)
}

pub fn watch(watch_dirs: Vec<String>, user_dotfiles: &str, systemd_mode: bool) -> Result<()> {
    let (tx, rx) = mpsc::channel::<Result<Event>>();
    let mut watcher = notify::recommended_watcher(move |res| {
        // Ignore send errors if receiver was dropped (shutdown)
        let _ = tx.send(res);
    })?;

    for file in watch_dirs.iter() {
        log::info!("Watching --> {:?}", file.as_str());
        // Watch recursively so nested file changes are detected
        watcher.watch(Path::new(file.as_str()), RecursiveMode::Recursive)?;
    }

    if systemd_mode {
        // Signal systemd that we are ready after watchers are registered
        if let Err(e) = systemd::notify_ready() {
            log::warn!("systemd notify READY failed: {:?}", e);
        }
        // Optional: set a human-readable status
        let _ = systemd::notify_status("oxidots: monitoring dotfiles");
    }

    for res in rx {
        match res {
            Ok(event) => {
                if event.kind
                    == notify::EventKind::Modify(notify::event::ModifyKind::Data(
                        notify::event::DataChange::Content,
                    ))
                {
                    // Mirror each modified file into the user repo, then commit
                    for p in &event.paths {
                        if let Some(src) = p.as_path().to_str() {
                            log::debug!("Event path: {}", src);
                        }
                        match mirror_modified_file(p.as_path(), &watch_dirs, user_dotfiles) {
                            Ok(Some(dst)) => log::info!(
                                "Mirrored modified file to repo: {:?}",
                                dst.to_string_lossy()
                            ),
                            Ok(None) => log::warn!(
                                "Modified file not under any watched dir: {:?}",
                                p
                            ),
                            Err(e) => log::error!(
                                "Failed to mirror modified file {:?}: {:?}",
                                p, e
                            ),
                        }
                    }
                    git_sync(user_dotfiles);
                }
            }
            Err(e) => log::error!("watch error: {:?}", e),
        }
    }

    Ok(())
}
