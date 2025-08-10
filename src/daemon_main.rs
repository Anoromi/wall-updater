// This runs daemon on windows without creating a console. Disable during development to see
// stdout.
#![windows_subsystem = "windows"]

use std::{env::args, fs, path::PathBuf};

use anyhow::Result;
use clap::Parser;
use wall_updater::{daemon::start_daemon, utils::create_application_default_path};

#[derive(Parser, Debug, Clone)]
#[command(name = "wall-updater-daemon")]
struct DaemonArgs {
    /// Force running in the current process (do not detach)
    #[arg(long, default_value_t = false)]
    force: bool,

    /// Optional app directory to store state (pid, images)
    #[arg(long)]
    dir: Option<PathBuf>,
}

fn main() {
    run_service(args().collect::<Vec<_>>()).unwrap();
}

fn run_service(command_args: Vec<String>) -> Result<()> {
    let args = DaemonArgs::parse_from(&command_args);

    if !args.force {
        #[cfg(target_os = "windows")]
        {
            let mut command_args = command_args;
            println!("Starting detached process");
            use std::os::windows::process::CommandExt;
            use windows::Win32::System::Threading::DETACHED_PROCESS;

            let mut command_args_to_child = command_args.clone();
            command_args_to_child.push("--force".into());
            let process_name = std::env::current_exe()?;
            println!("Process {:?}", process_name);
            let mut command = std::process::Command::new(process_name);
            command.args(command_args_to_child.into_iter().skip(1));
            command.creation_flags(DETACHED_PROCESS.0);
            command.stdin(std::process::Stdio::null());
            command.stdout(std::process::Stdio::null());
            command.stderr(std::process::Stdio::null());
            #[allow(clippy::zombie_processes)]
            command.spawn()?;
            println!("Created daemon");
            return Ok(());
        }
        #[cfg(unix)]
        {
            use daemonize::Daemonize;

            let daemonize = Daemonize::new()
                .stdout(daemonize::Stdio::devnull())
                .stderr(daemonize::Stdio::devnull())
                .execute();
            match daemonize {
                daemonize::Outcome::Parent(parent) => {
                    parent.inspect_err(|e| {
                        eprintln!("Failed to create daemon on parent side {e:?}")
                    })?;
                    println!("Created daemon");
                    return Ok(());
                }
                daemonize::Outcome::Child(_) => (),
            }
        }
    }

    run(args)
}

fn run(args: DaemonArgs) -> Result<()> {
    let app_dir = args.dir.map_or_else(create_application_default_path, Ok)?;
    let result = start_daemon(app_dir.clone());
    if let Err(e) = result {
        let mut error_file = app_dir;
        error_file.push("err.log");
        fs::write(error_file, format!("{:?}", e))?;
    }
    Ok(())
}
