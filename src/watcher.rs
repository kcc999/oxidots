use notify::{Event, Result, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc;

use crate::git_sync;
use crate::systemd;

pub fn watch(watch_dirs: Vec<String>, user_dotfiles: &str, systemd_mode: bool) -> Result<()> {
    let (tx, rx) = mpsc::channel::<Result<Event>>();
    let mut watcher = notify::recommended_watcher(move |res| {
        // Ignore send errors if receiver was dropped (shutdown)
        let _ = tx.send(res);
    })?;

    for file in watch_dirs.iter() {
        log::info!("Watching --> {:?}", file.as_str());
        watcher.watch(Path::new(file.as_str()), RecursiveMode::NonRecursive)?;
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
                    log::info!("Modified file: {:?}", event.paths.get(0));
                    git_sync(user_dotfiles);
                }
            }
            Err(e) => log::error!("watch error: {:?}", e),
        }
    }

    Ok(())
}
