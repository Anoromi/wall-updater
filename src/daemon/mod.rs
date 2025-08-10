use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    thread,
    time::Duration,
};

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use url::Url;

const BING_API: &str = "https://www.bing.com/HPImageArchive.aspx?format=js&idx=0&n=1&mkt=en-US";
const CHECK_INTERVAL: Duration = Duration::from_secs(60 * 60); // hourly

#[derive(Debug, Deserialize)]
struct BingApiResponse {
    images: Vec<BingImage>,
}

#[derive(Debug, Deserialize)]
struct BingImage {
    url: String,
    #[allow(dead_code)]
    urlbase: Option<String>,
    #[allow(dead_code)]
    copyright: Option<String>,
    #[allow(dead_code)]
    title: Option<String>,
}

fn read_pid_file(pid_path: &Path) -> Result<Option<i32>> {
    if !pid_path.exists() {
        return Ok(None);
    }
    let mut f = File::open(pid_path).with_context(|| format!("open pid file {:?}", pid_path))?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    let pid: i32 = s.trim().parse().context("parse pid")?;
    Ok(Some(pid))
}

fn write_pid_file(pid_path: &Path, pid: i32) -> Result<()> {
    let mut f = File::create(pid_path).with_context(|| format!("create pid file {:?}", pid_path))?;
    f.write_all(pid.to_string().as_bytes())?;
    Ok(())
}

fn process_exists_unix(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM) }
}

fn set_gnome_wallpaper(image_path: &Path) -> Result<()> {
    let uri = format!("file://{}", image_path.to_string_lossy());
    let status = std::process::Command::new("gsettings")
        .args(["set", "org.gnome.desktop.background", "picture-uri", &uri])
        .status()
        .context("run gsettings for background")?;
    if !status.success() {
        return Err(anyhow!("gsettings failed to set background"));
    }
    let _ = std::process::Command::new("gsettings")
        .args(["set", "org.gnome.desktop.background", "picture-uri-dark", &uri])
        .status();
    let _ = std::process::Command::new("gsettings")
        .args(["set", "org.gnome.desktop.screensaver", "picture-uri", &uri])
        .status();
    Ok(())
}

fn fetch_bing_image_json() -> Result<BingApiResponse> {
    let resp = reqwest::blocking::get(BING_API).context("fetch bing api")?;
    if !resp.status().is_success() {
        return Err(anyhow!("bing api status {}", resp.status()));
    }
    let json = resp.json::<BingApiResponse>().context("parse bing api json")?;
    Ok(json)
}

fn download_image(image_url: &Url, dest_path: &Path) -> Result<()> {
    let mut resp = reqwest::blocking::get(image_url.as_str()).context("download image")?;
    if !resp.status().is_success() {
        return Err(anyhow!("image download status {}", resp.status()));
    }
    let mut file = File::create(dest_path).context("create image file")?;
    let mut buf = Vec::new();
    resp.copy_to(&mut buf).context("read image bytes")?;
    file.write_all(&buf).context("write image file")?;
    Ok(())
}

fn ensure_single_instance(app_dir: &Path) -> Result<File> {
    let pid_path = app_dir.join("daemon.pid");

    if let Some(existing_pid) = read_pid_file(&pid_path)? {
        if process_exists_unix(existing_pid) {
            return Err(anyhow!("daemon already running with pid {}", existing_pid));
        } else {
            let _ = fs::remove_file(&pid_path);
        }
    }

    let pid = std::process::id() as i32;
    write_pid_file(&pid_path, pid)?;
    let f = File::open(&pid_path).context("reopen pid file")?;
    Ok(f)
}

pub fn start_daemon(app_dir: PathBuf) -> Result<()> {
    fs::create_dir_all(&app_dir).with_context(|| format!("create app dir {:?}", app_dir))?;
    let _lock_file = ensure_single_instance(&app_dir)?;

    let image_path = app_dir.join("current_wallpaper.jpg");
    let base = Url::parse("https://www.bing.com").expect("valid base url");

    loop {
        match run_once(&base, &image_path) {
            Ok(()) => {}
            Err(err) => eprintln!("daemon tick error: {:#}", err),
        }
        thread::sleep(CHECK_INTERVAL);
    }
}

fn run_once(base: &Url, image_path: &Path) -> Result<()> {
    let data = fetch_bing_image_json()?;
    let first = data
        .images
        .get(0)
        .ok_or_else(|| anyhow!("no image found in bing response"))?;
    let joined = base.join(&first.url).context("join base with image url")?;
    download_image(&joined, image_path)?;
    set_gnome_wallpaper(image_path)?;
    Ok(())
}