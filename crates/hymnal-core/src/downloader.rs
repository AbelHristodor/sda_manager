//! YouTube downloader: resolves the yt-dlp binary, spawns it, and streams
//! parsed progress events back over a channel. Pure logic lives here so it can
//! be unit-tested without a GUI or network access.

use std::path::PathBuf;

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

use std::path::Path;

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
