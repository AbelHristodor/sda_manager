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

#[cfg(test)]
mod tests {
    use super::*;

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
}
