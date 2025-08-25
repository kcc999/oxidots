# Oxidots

Oxidots watches one or more directories listed in a config file, copies their contents to your dotfiles repository, and commits changes automatically when files change.

## Systemd Integration

When started with `--systemd`, Oxidots integrates with systemd using sd_notify semantics:

- READY: Signals `READY=1` after file watchers are registered so `Type=notify` units transition to running.
- STATUS: Sets a human-readable status displayed by `systemctl status`.
- Watchdog: If `WatchdogSec` is configured for the unit, systemd provides `WATCHDOG_USEC` and Oxidots pings `WATCHDOG=1` at half the configured interval.
- Logging: With `--systemd`, logs go to stdout/stderr and are captured by journald. Without it, logs go to a local file (`~.oxidots.log`).

### Example unit

Create `/etc/systemd/system/oxidots.service` (system service) or `~/.config/systemd/user/oxidots.service` (user service):

```
[Unit]
Description=Oxidots: sync and commit dotfiles
After=network.target

[Service]
Type=notify
# Adjust the path to your compiled binary and arguments
ExecStart=/usr/local/bin/oxidots /path/to/config.txt /path/to/dotfiles --systemd
# Optional: enable watchdog; Oxidots will auto-detect and ping
WatchdogSec=30s
Restart=on-failure
# For user services with paths under $HOME, you may want:
# WorkingDirectory=%h

[Install]
WantedBy=multi-user.target
```

Enable and start:

```
sudo systemctl daemon-reload
sudo systemctl enable --now oxidots.service
# or for user service:
# systemctl --user daemon-reload
# systemctl --user enable --now oxidots.service
```

Verify readiness and watchdog:

```
systemctl status oxidots.service
# Look for "Ready" state and no watchdog timeouts
journalctl -u oxidots.service -f
```

## Usage

```
oxidots <config_file> <dotfiles_repo> [--systemd]
```

- `config_file`: A text file with one path per line, each path is a directory to watch. Example:
  - `/home/you/.config/nvim`
  - `/home/you/.config/alacritty`
- `dotfiles_repo`: Path to a local directory where Oxidots mirrors directories and commits on change. If it is not a Git repo, Oxidots will initialize one automatically.
- `--systemd`: Enables sd_notify readiness and optional watchdog pings; logs are sent to journald.

Notes:
- Initial snapshot: Oxidots performs an initial directory copy, and commits changes on subsequent file modifications. To force an immediate first commit, make a trivial change in one of the watched files (e.g., `touch <file>` and save) after startup.

## Build

```
cargo build --release
```

The resulting binary will be at `target/release/oxidots`.

## Tests

Run unit tests:

```
cargo test
```

If you hit linker errors for `git2`/`libgit2`, install your system's development libraries (e.g., Debian/Ubuntu: `sudo apt install libgit2-dev pkg-config`) or ask to switch to a vendored `libgit2` build.

## How It Works (brief)

- Reads watch directories from the config file.
- Performs an initial copy of each listed directory into `<dotfiles_repo>/<dirname>`.
- Starts non-recursive file watchers on each listed directory.
- On content modifications, stages all changes in `dotfiles_repo` and creates a commit.

## Troubleshooting

- No logs in systemd mode: Ensure the service runs with `--systemd`; check `journalctl -u oxidots`.
- Watchdog timeouts: Increase `WatchdogSec` or check system load; Oxidots pings at half the watchdog interval.
- No initial commit: The repo is initialized automatically if missing. The first commit happens on the first detected change; make a trivial edit to trigger it.
