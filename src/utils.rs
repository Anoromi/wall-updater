use std::{env, io, path::PathBuf};

use anyhow::Result;

pub fn create_application_default_path() -> Result<PathBuf> {
    let path = {
        #[cfg(target_os = "linux")]
        {
            use anyhow::anyhow;

            let mut path = env::var("XDG_STATE_HOME")
                .map(PathBuf::from)
                .or_else(|_| {
                    env::var("HOME").map(|home| {
                        let mut path = PathBuf::from(home);
                        path.push(".local/state");
                        path
                    })
                })
                .map_err(|_| anyhow!("Couldn't find neither XDG_STATE_HOME nor HOME"))?;
            path.push(env!("CARGO_PKG_NAME"));
            path
        }
    };

    match std::fs::create_dir_all(&path) {
        Ok(_) => Ok(path),
        Err(v) if v.kind() == io::ErrorKind::AlreadyExists => Ok(path),
        Err(v) => Err(v.into()),
    }
}