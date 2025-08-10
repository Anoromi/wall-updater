use std::{env, fs, path::PathBuf, process::Command};

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use wall_updater::utils::create_application_default_path;

#[derive(Parser, Debug)]
#[command(name = "wall-updater")]
#[command(about = "Bing wallpaper updater for GNOME", long_about = None)]
struct Cli {
    /// Optional application state directory
    #[arg(long)]
    dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start the daemon (fails if already running)
    Start,
    /// Restart the daemon if running, else start it
    Restart,
    /// Add application to gnome autostart
    Autostart
}

fn pid_path(dir: &PathBuf) -> PathBuf {
    dir.join("daemon.pid")
}

#[cfg(unix)]
fn is_process_running(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM) }
}

fn is_daemon_running(dir: &PathBuf) -> Result<bool> {
    let pid_file = pid_path(dir);
    if !pid_file.exists() {
        return Ok(false);
    }
    let pid_str = fs::read_to_string(&pid_file).context("read pid file")?;
    let pid: i32 = pid_str.trim().parse().context("parse pid")?;
    #[cfg(unix)]
    {
        Ok(is_process_running(pid))
    }
    #[cfg(not(unix))]
    {
        Ok(true)
    }
}

fn try_kill_existing(dir: &PathBuf) -> Result<bool> {
    let pid_file = pid_path(dir);
    if !pid_file.exists() {
        return Ok(false);
    }
    let pid_str = fs::read_to_string(&pid_file).context("read pid file")?;
    let pid: i32 = pid_str.trim().parse().context("parse pid")?;
    #[cfg(unix)]
    unsafe {
        let _ = libc::kill(pid, libc::SIGTERM);
    }
    std::thread::sleep(std::time::Duration::from_millis(300));
    #[cfg(unix)]
    unsafe {
        if libc::kill(pid, 0) != 0 {
            let _ = fs::remove_file(&pid_file);
        }
    }
    Ok(true)
}

fn spawn_daemon(dir: &PathBuf) -> Result<()> {
    let exe = std::env::current_exe().context("current exe")?;
    let daemon_exe = exe
        .parent()
        .ok_or_else(|| anyhow!("no exe parent"))?
        .join("wall-updater-daemon");

    let mut cmd = Command::new(daemon_exe);
    cmd.arg("--dir").arg(dir);
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());
    let _child = cmd.spawn().context("spawn daemon")?;
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let app_dir = cli.dir.map_or_else(create_application_default_path, Ok)?;
    fs::create_dir_all(&app_dir).ok();

    match cli.command {
        Commands::Start => {
            if is_daemon_running(&app_dir)? {
                return Err(anyhow!("daemon already running"));
            }
            spawn_daemon(&app_dir)?;
        }
        Commands::Restart => {
            try_kill_existing(&app_dir)?;
            spawn_daemon(&app_dir)?;
        },
        Commands::Autostart => {
            autostart()?;
        }
    }

    Ok(())
}


fn autostart() -> Result<()> {
    let mut exe_path = std::env::current_exe()?;
    let current_extension = exe_path.extension().map(|v| v.to_owned());
    exe_path.set_file_name(format!("{}-daemon", env!("CARGO_PKG_NAME")).as_str());
    if let Some(extension) = current_extension {
        exe_path.set_extension(extension);
    }

    let mut autostart_path = env::var("HOME").expect("HOME should be present on Linux");
    autostart_path.push_str("/.config/autostart");
    autostart_path.push_str(format!("/{}-daemon.desktop", env!("CARGO_PKG_NAME")).as_str());
    let autostart_config = format!(
        r#"
[Desktop Entry]
Type=Application
Name={}
Exec={}
OnlyShowIn=GNOME;
X-GNOME-Autostart-enabled=true
StartupNotify=false"#,
        env!("CARGO_PKG_NAME"),
        exe_path.to_str().unwrap()
    );
    std::fs::write(autostart_path, autostart_config).unwrap();
    Ok(())
}