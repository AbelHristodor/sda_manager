//! YouTube downloader: resolves the yt-dlp binary, spawns it, and streams
//! parsed progress events back over a channel. Pure logic lives here so it can
//! be unit-tested without a GUI or network access.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;

/// A single progress update parsed from yt-dlp's output.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DownloadProgress {
    pub percent: f32,
    pub speed: String,
    pub eta: String,
    pub title: String,
}

/// Events streamed from the download worker to the UI.
#[derive(Debug, Clone, PartialEq)]
pub enum DownloadEvent {
    /// Locating or auto-downloading yt-dlp/ffmpeg.
    Resolving,
    /// The resolved video title, sent once known.
    Title(String),
    /// Incremental download progress.
    Progress(DownloadProgress),
    /// Finished successfully; the saved file (or its folder).
    Done { path: PathBuf },
    /// Failed with a human-readable reason.
    Failed { message: String },
}

/// Parse one yt-dlp output line into a `DownloadProgress`, if it is a progress
/// line emitted by our template (`PROGRESS|<pct>%|<speed>|<eta>`). Returns
/// `None` for any other line.
pub fn parse_progress_line(line: &str) -> Option<DownloadProgress> {
    let rest = line.trim().strip_prefix("PROGRESS|")?;
    let mut parts = rest.split('|');
    let pct_raw = parts.next()?.trim().trim_end_matches('%').trim();
    let speed = parts.next()?.trim().to_string();
    let eta = parts.next()?.trim().to_string();
    let percent = pct_raw.parse::<f32>().ok()?; // "N/A" => None
    Some(DownloadProgress {
        percent,
        speed,
        eta,
        title: String::new(),
    })
}

/// Platform-correct executable file name (adds `.exe` on Windows).
pub fn binary_name(stem: &str) -> String {
    if cfg!(windows) {
        format!("{stem}.exe")
    } else {
        stem.to_string()
    }
}

/// Return the path to `stem` inside `dir` if it exists there, else `None`.
pub fn resolve_in_dir(dir: &Path, stem: &str) -> Option<PathBuf> {
    let candidate = dir.join(binary_name(stem));
    if candidate.is_file() {
        Some(candidate)
    } else {
        None
    }
}

/// Directory where auto-downloaded tool binaries are stored.
pub fn tools_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("org", "hymnal", "HymnFinder")
        .map(|d| d.data_dir().join("tools"))
}

/// Locate `stem` (e.g. "yt-dlp"): prefer the app tools dir, then `PATH`.
/// Returns `None` if not found anywhere.
pub fn find_existing(stem: &str) -> Option<PathBuf> {
    if let Some(dir) = tools_dir() {
        if let Some(p) = resolve_in_dir(&dir, stem) {
            return Some(p);
        }
    }
    which_in_path(stem)
}

/// Minimal `PATH` lookup (avoids pulling in a crate for one function).
fn which_in_path(stem: &str) -> Option<PathBuf> {
    let name = binary_name(stem);
    let paths = std::env::var_os("PATH")?;
    std::env::split_paths(&paths)
        .map(|p| p.join(&name))
        .find(|p| p.is_file())
}

/// GitHub release asset name for yt-dlp on the current platform.
fn ytdlp_asset_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "yt-dlp.exe"
    } else if cfg!(target_os = "macos") {
        "yt-dlp_macos"
    } else {
        "yt-dlp" // linux x86_64; the universal binary
    }
}

/// Ensure a yt-dlp binary exists, downloading the latest release into the
/// tools dir if necessary. Returns the path to the binary.
pub fn ensure_ytdlp() -> anyhow::Result<PathBuf> {
    if let Some(p) = find_existing("yt-dlp") {
        return Ok(p);
    }
    let dir = tools_dir().ok_or_else(|| anyhow::anyhow!("no data directory available"))?;
    std::fs::create_dir_all(&dir)?;
    let dest = dir.join(binary_name("yt-dlp"));
    let url = format!(
        "https://github.com/yt-dlp/yt-dlp/releases/latest/download/{}",
        ytdlp_asset_name()
    );
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(120))
        .build();
    let resp = agent.get(&url).call()?;
    let tmp = dir.join(format!("{}.part", binary_name("yt-dlp")));
    {
        let mut reader = resp.into_reader();
        let mut file = std::fs::File::create(&tmp)?;
        std::io::copy(&mut reader, &mut file)?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&tmp)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&tmp, perms)?;
    }
    std::fs::rename(&tmp, &dest)?;
    Ok(dest)
}

/// yt-dlp progress template that produces lines our parser understands.
const PROGRESS_TEMPLATE: &str =
    "download:PROGRESS|%(progress._percent_str)s|%(progress._speed_str)s|%(progress._eta_str)s";

/// Download a single video from `url` into `dir`, streaming `DownloadEvent`s on
/// `tx`. Blocks until the child process exits, so call it on a worker thread.
pub fn download(url: &str, dir: &Path, tx: &Sender<DownloadEvent>) {
    let _ = tx.send(DownloadEvent::Resolving);
    let ytdlp = match ensure_ytdlp() {
        Ok(p) => p,
        Err(e) => {
            let _ = tx.send(DownloadEvent::Failed {
                message: format!("Could not set up downloader: {e}"),
            });
            return;
        }
    };

    let have_ffmpeg = find_existing("ffmpeg").is_some();
    // With ffmpeg we can merge best video+audio; without it, take the best
    // pre-merged single-file format so the download still succeeds.
    let format = if have_ffmpeg { "bv*+ba/b" } else { "b" };

    let output_template = dir.join("%(title)s.%(ext)s");
    let mut cmd = Command::new(&ytdlp);
    cmd.arg("--no-playlist")
        .arg("--newline")
        .args(["-f", format])
        .args(["--progress-template", PROGRESS_TEMPLATE])
        .args(["--print", "before_dl:TITLE|%(title)s"])
        .arg("-o")
        .arg(&output_template)
        .arg(url)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if have_ffmpeg {
        cmd.args(["--merge-output-format", "mp4"]);
    }

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            let _ = tx.send(DownloadEvent::Failed {
                message: format!("Failed to start yt-dlp: {e}"),
            });
            return;
        }
    };

    // Drain stderr concurrently on its own thread to avoid a pipe-buffer
    // deadlock if yt-dlp fills stderr while we're blocked reading stdout.
    let stderr_handle = child.stderr.take().map(|e| {
        std::thread::spawn(move || {
            BufReader::new(e)
                .lines()
                .map_while(Result::ok)
                .collect::<Vec<_>>()
                .join("\n")
        })
    });

    if let Some(out) = child.stdout.take() {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            if let Some(title) = line.trim().strip_prefix("TITLE|") {
                let _ = tx.send(DownloadEvent::Title(title.to_string()));
            } else if let Some(p) = parse_progress_line(&line) {
                let _ = tx.send(DownloadEvent::Progress(p));
            }
        }
    }

    let status = child.wait();
    let stderr_tail = stderr_handle
        .map(|h| h.join().unwrap_or_default())
        .unwrap_or_default();

    match status {
        Ok(s) if s.success() => {
            let _ = tx.send(DownloadEvent::Done {
                path: dir.to_path_buf(),
            });
        }
        Ok(s) => {
            let msg = stderr_tail
                .lines()
                .rev()
                .find(|l| l.contains("ERROR"))
                .map(|l| l.to_string())
                .unwrap_or_else(|| format!("yt-dlp exited with {s}"));
            let _ = tx.send(DownloadEvent::Failed { message: msg });
        }
        Err(e) => {
            let _ = tx.send(DownloadEvent::Failed {
                message: format!("yt-dlp did not run: {e}"),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn parses_a_progress_line() {
        let p = parse_progress_line("PROGRESS|42.0%|3.20MiB/s|00:18").unwrap();
        assert_eq!(p.percent, 42.0);
        assert_eq!(p.speed, "3.20MiB/s");
        assert_eq!(p.eta, "00:18");
    }

    #[test]
    fn handles_whitespace_and_percent_sign() {
        let p = parse_progress_line("PROGRESS| 7.5%| 1.00KiB/s | 01:02 ").unwrap();
        assert_eq!(p.percent, 7.5);
        assert_eq!(p.speed, "1.00KiB/s");
        assert_eq!(p.eta, "01:02");
    }

    #[test]
    fn ignores_non_progress_lines() {
        assert!(parse_progress_line("[youtube] Extracting URL").is_none());
        assert!(parse_progress_line("").is_none());
    }

    #[test]
    fn ignores_unknown_percent() {
        // yt-dlp emits "Unknown" / "N/A" for some fields early on.
        let p = parse_progress_line("PROGRESS|N/A%|Unknown|Unknown");
        assert!(p.is_none());
    }

    #[test]
    fn resolves_binary_in_data_dir_first() {
        let dir = tempfile::tempdir().unwrap();
        let name = if cfg!(windows) { "yt-dlp.exe" } else { "yt-dlp" };
        let bin = dir.path().join(name);
        fs::write(&bin, b"#!/bin/sh\n").unwrap();
        let found = resolve_in_dir(dir.path(), "yt-dlp");
        assert_eq!(found, Some(bin));
    }

    #[test]
    fn returns_none_when_absent_from_data_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(resolve_in_dir(dir.path(), "yt-dlp"), None);
    }

    #[test]
    fn binary_name_has_exe_on_windows() {
        let name = binary_name("yt-dlp");
        if cfg!(windows) {
            assert_eq!(name, "yt-dlp.exe");
        } else {
            assert_eq!(name, "yt-dlp");
        }
    }
}
