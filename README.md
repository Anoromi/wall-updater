## Wall-updater

Bing wallpaper for GNOME.
This is a cli/daemon that allows you to periodically fetch the Bing daily image and set it as the GNOME desktop (and screensaver) wallpaper.

### Requirements
- **GNOME** desktop environment
- Rust toolchain (to build from source)

### Build
```bash
# From repository root
cargo build --release
```

### CLI usage
```bash
# Restart the daemon (kills if running, then starts)
wall-updater restart

# Add GNOME autostart entry for the daemon (~/.config/autostart)
wall-updater autostart
```

### Behavior
- Polls Bing API hourly for a new image.
- Downloads the latest image and writes it to `current_wallpaper.jpg` in the state directory.
- Sets GNOME wallpaper using `gsettings` for:
  - `org.gnome.desktop.background picture-uri`
  - `org.gnome.desktop.background picture-uri-dark`
  - `org.gnome.desktop.screensaver picture-uri`

### Autostart
`wall-updater autostart` creates `~/.config/autostart/wall-updater-daemon.desktop` pointing to the built `wall-updater-daemon` binary. It is scoped to GNOME.

### Troubleshooting
- Ensure `gsettings` is available and GNOME is running.
- Check the state directory for `err.log` if the daemon encountered errors.
- If the daemon appears stuck, remove the stale `daemon.pid` and restart via the CLI.