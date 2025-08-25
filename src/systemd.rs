use sd_notify::{notify, NotifyState};
use std::env;
use std::thread;
use std::time::{Duration, Instant};

pub fn notify_ready() -> Result<(), sd_notify::Error> {
    // Do not unset environment; let systemd reuse socket for future pings
    notify(false, &[NotifyState::Ready])
}

pub fn notify_status(status: &str) -> Result<(), sd_notify::Error> {
    notify(false, &[NotifyState::Status(status.to_string())])
}

pub fn notify_stopping() -> Result<(), sd_notify::Error> {
    notify(false, &[NotifyState::Stopping])
}

pub fn maybe_start_watchdog() {
    let watchdog_usec = match env::var("WATCHDOG_USEC") {
        Ok(v) => v,
        Err(_) => return,
    };
    let usec: u64 = match watchdog_usec.parse() {
        Ok(v) => v,
        Err(_) => return,
    };

    // Ping interval: half of watchdog period
    let interval = Duration::from_micros(usec / 2);
    thread::spawn(move || loop {
        let start = Instant::now();
        let _ = notify(false, &[NotifyState::Watchdog]);
        let elapsed = start.elapsed();
        if interval > elapsed {
            thread::sleep(interval - elapsed);
        }
    });
}
