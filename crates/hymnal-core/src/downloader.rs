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
    /// Video title, carried on every progress line (the template includes it).
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
/// line emitted by our template (`PROGRESS|<pct>%|<speed>|<eta>|<title>`).
/// Returns `None` for any other line. The title field is last and may itself
/// contain `|`, so it is taken as the remainder after the first four splits.
pub fn parse_progress_line(line: &str) -> Option<DownloadProgress> {
    let rest = line.trim().strip_prefix("PROGRESS|")?;
    // Split into at most 4 pieces: pct, speed, eta, then the title remainder.
    let mut parts = rest.splitn(4, '|');
    let pct_raw = parts.next()?.trim().trim_end_matches('%').trim();
    let speed = parts.next()?.trim().to_string();
    let eta = parts.next()?.trim().to_string();
    // Title is optional (older template had no title field).
    let title = parts.next().unwrap_or("").trim().to_string();
    let percent = pct_raw.parse::<f32>().ok()?; // "N/A" => None
    Some(DownloadProgress {
        percent,
        speed,
        eta,
        title,
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

/// Download `url` to `dest` using `agent`, removing a partial file on failure.
fn download_to(agent: &ureq::Agent, url: &str, dest: &Path) -> anyhow::Result<()> {
    let result = (|| -> anyhow::Result<()> {
        let resp = agent.get(url).call()?;
        let mut reader = resp.into_reader();
        let mut file = std::fs::File::create(dest)?;
        std::io::copy(&mut reader, &mut file)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(dest);
    }
    result
}

/// Mark `path` executable on Unix (no-op elsewhere).
fn make_executable(path: &Path) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(path, perms)?;
    }
    // On macOS a binary downloaded by the app inherits a quarantine attribute;
    // when the GUI app later spawns it, Gatekeeper can block execution. Strip
    // the quarantine flag so our own auto-downloaded tools run cleanly. Best
    // effort — ignore failures (e.g. attribute absent).
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("/usr/bin/xattr")
            .arg("-d")
            .arg("com.apple.quarantine")
            .arg(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = path; // silence unused warning on non-unix
    Ok(())
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
        maybe_self_update(&p);
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
    let tmp = dir.join(format!("{}.part", binary_name("yt-dlp")));
    // GitHub releases/latest/download issues a 302 to the CDN; ureq follows
    // redirects by default (do not disable that).
    download_to(&agent, &url, &tmp)?;
    make_executable(&tmp)?;
    std::fs::rename(&tmp, &dest)?;
    Ok(dest)
}

/// Download URL for a static ffmpeg build for the current platform.
fn ffmpeg_archive_url() -> &'static str {
    if cfg!(target_os = "windows") {
        "https://github.com/yt-dlp/FFmpeg-Builds/releases/latest/download/ffmpeg-master-latest-win64-gpl.zip"
    } else if cfg!(target_os = "macos") {
        // evermeet.cx is a third-party ffmpeg builder (yt-dlp's FFmpeg-Builds
        // has no macOS asset). HTTPS-only; no checksum verification.
        "https://evermeet.cx/ffmpeg/getrelease/zip"
    } else {
        "https://github.com/yt-dlp/FFmpeg-Builds/releases/latest/download/ffmpeg-master-latest-linux64-gpl.tar.xz"
    }
}

/// Extract the ffmpeg binary from a zip archive (Windows/macOS builds) into `dest`.
fn extract_ffmpeg_from_zip(archive: &Path, dest: &Path) -> anyhow::Result<()> {
    let file = std::fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file)?;
    let target = binary_name("ffmpeg");
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        if !entry.is_file() {
            continue;
        }
        let name = entry.name().to_string();
        // Match the ffmpeg binary whether it's at the archive root (macOS) or
        // nested under bin/ (Windows builds).
        if name == target || name.ends_with(&format!("/{target}")) {
            let mut out = std::fs::File::create(dest)?;
            std::io::copy(&mut entry, &mut out)?;
            return Ok(());
        }
    }
    anyhow::bail!("ffmpeg binary not found in zip archive")
}

/// Extract the ffmpeg binary from a tar.xz archive (Linux builds) into `dest`.
fn extract_ffmpeg_from_tar_xz(archive: &Path, dest: &Path) -> anyhow::Result<()> {
    let file = std::fs::File::open(archive)?;
    let xz = xz2::read::XzDecoder::new(file);
    let mut tar = tar::Archive::new(xz);
    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let is_ffmpeg = path.file_name().and_then(|n| n.to_str()) == Some("ffmpeg")
            && path.to_string_lossy().contains("/bin/");
        if is_ffmpeg {
            let mut out = std::fs::File::create(dest)?;
            std::io::copy(&mut entry, &mut out)?;
            return Ok(());
        }
    }
    anyhow::bail!("ffmpeg binary not found in tar.xz archive")
}

/// Ensure an ffmpeg binary exists, downloading a static build for the current
/// platform into the tools dir if necessary. Returns the path to the binary.
pub fn ensure_ffmpeg() -> anyhow::Result<PathBuf> {
    if let Some(p) = find_existing("ffmpeg") {
        return Ok(p);
    }
    let dir = tools_dir().ok_or_else(|| anyhow::anyhow!("no data directory available"))?;
    std::fs::create_dir_all(&dir)?;
    let dest = dir.join(binary_name("ffmpeg"));
    let url = ffmpeg_archive_url();
    // ffmpeg archives are tens of MB; allow a generous timeout.
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(300))
        .build();
    let archive = dir.join("ffmpeg-archive.part");
    // GitHub releases/latest/download (and evermeet.cx) redirect to a CDN;
    // ureq follows redirects by default.
    download_to(&agent, url, &archive)?;
    let tmp = dir.join(format!("{}.part", binary_name("ffmpeg")));
    let extracted = if url.ends_with(".tar.xz") {
        extract_ffmpeg_from_tar_xz(&archive, &tmp)
    } else {
        extract_ffmpeg_from_zip(&archive, &tmp)
    };
    let _ = std::fs::remove_file(&archive);
    if extracted.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    extracted?;
    make_executable(&tmp)?;
    std::fs::rename(&tmp, &dest)?;
    Ok(dest)
}

/// Path to the timestamp file recording the last yt-dlp self-update attempt.
fn ytdlp_update_stamp() -> Option<PathBuf> {
    tools_dir().map(|d| d.join(".yt-dlp-last-update"))
}

/// True if the last self-update was more than 24h ago (or never).
fn should_self_update(stamp: &Path) -> bool {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let last = std::fs::read_to_string(stamp)
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0);
    now.saturating_sub(last) > 24 * 3600
}

/// Best-effort `yt-dlp -U`, at most once per 24h, and only for the copy we
/// manage in the tools dir (never a system/PATH install).
///
/// Fire-and-forget: the update runs as a *detached* child and we never wait on
/// it, so it can NEVER block (or hang) a download. `yt-dlp -U` replaces the
/// binary via an atomic rename, so the current download keeps using whatever
/// version is on disk now and the update simply takes effect next time. The
/// timestamp is written up front so a slow or failed update doesn't re-trigger
/// on every download.
fn maybe_self_update(ytdlp: &Path) {
    let managed = tools_dir().map(|d| ytdlp.starts_with(&d)).unwrap_or(false);
    if !managed {
        return;
    }
    let Some(stamp) = ytdlp_update_stamp() else {
        return;
    };
    if !should_self_update(&stamp) {
        return;
    }
    // Write the stamp first: even if the spawn fails we won't retry until the
    // 24h window elapses.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let _ = std::fs::write(&stamp, now.to_string());
    // Spawn detached and do NOT wait — this must not gate the download.
    let _ = Command::new(ytdlp)
        .arg("-U")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

/// yt-dlp progress template that produces lines our parser understands. The
/// title rides on every progress line via `%(info.title)s` so we get it without
/// `--print` — `--print` puts yt-dlp in a quiet mode that SUPPRESSES progress
/// output entirely, which previously left the UI stuck on "Setting up…".
const PROGRESS_TEMPLATE: &str =
    "download:PROGRESS|%(progress._percent_str)s|%(progress._speed_str)s|%(progress._eta_str)s|%(info.title)s";

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

    // Ensure a destination we can actually write to before doing any work.
    if let Err(e) = std::fs::create_dir_all(dir) {
        let _ = tx.send(DownloadEvent::Failed {
            message: format!("Cannot use download folder: {e}"),
        });
        return;
    }

    // Auto-download ffmpeg so we can merge best video+audio. If it can't be
    // obtained, fall back to the best pre-merged single-file format.
    let ffmpeg = ensure_ffmpeg();
    let have_ffmpeg = ffmpeg.is_ok();
    // Cap at 1080p: 4K streams are huge and far more likely to need a PO token
    // (and 403). Prefer merged best <=1080p, then any pre-merged fallback.
    let format = if have_ffmpeg {
        "bv*[height<=1080]+ba/b[height<=1080]/bv*+ba/b"
    } else {
        "b[height<=1080]/b"
    };

    let output_template = dir.join("%(title)s.%(ext)s");
    let mut cmd = Command::new(&ytdlp);
    cmd.arg("--no-playlist")
        .arg("--newline")
        // YouTube now needs a JS runtime for the default web player's signature.
        // Without one (we don't bundle Deno), yt-dlp falls back to the
        // "android vr" client whose media URLs frequently return HTTP 403.
        // Forcing the `default` client set selects formats that download
        // reliably without a PO token. See yt-dlp wiki: PO-Token-Guide / EJS.
        .args(["--extractor-args", "youtube:player_client=default"])
        // Be resilient to transient throttling/errors mid-download.
        .args(["--retries", "10"])
        .args(["--fragment-retries", "10"])
        .args(["-f", format])
        .args(["--progress-template", PROGRESS_TEMPLATE])
        .arg("-o")
        .arg(&output_template)
        .arg(url)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Ok(ref ffmpeg_path) = ffmpeg {
        cmd.args(["--ffmpeg-location", &ffmpeg_path.to_string_lossy()]);
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
        let mut sent_title = false;
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            if let Some(p) = parse_progress_line(&line) {
                // The title rides on every progress line; emit it once, the
                // first time we see a non-empty one.
                if !sent_title && !p.title.is_empty() {
                    let _ = tx.send(DownloadEvent::Title(p.title.clone()));
                    sent_title = true;
                }
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
        let p = parse_progress_line("PROGRESS|42.0%|3.20MiB/s|00:18|My Song").unwrap();
        assert_eq!(p.percent, 42.0);
        assert_eq!(p.speed, "3.20MiB/s");
        assert_eq!(p.eta, "00:18");
        assert_eq!(p.title, "My Song");
    }

    #[test]
    fn parses_line_without_title_field() {
        // Older template (no title) must still parse; title defaults to empty.
        let p = parse_progress_line("PROGRESS|42.0%|3.20MiB/s|00:18").unwrap();
        assert_eq!(p.percent, 42.0);
        assert_eq!(p.title, "");
    }

    #[test]
    fn title_with_pipe_is_preserved() {
        // The title is the remainder after 4 splits, so internal `|` survives.
        let p = parse_progress_line("PROGRESS|10.0%|1MiB/s|00:30|A | B | C").unwrap();
        assert_eq!(p.title, "A | B | C");
    }

    #[test]
    fn handles_whitespace_and_percent_sign() {
        let p = parse_progress_line("PROGRESS| 7.5%| 1.00KiB/s | 01:02 | Tune ").unwrap();
        assert_eq!(p.percent, 7.5);
        assert_eq!(p.speed, "1.00KiB/s");
        assert_eq!(p.eta, "01:02");
        assert_eq!(p.title, "Tune");
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
    fn ffmpeg_url_matches_platform() {
        let url = ffmpeg_archive_url();
        assert!(!url.is_empty());
        if cfg!(target_os = "windows") {
            assert!(url.ends_with(".zip"));
        } else if cfg!(target_os = "macos") {
            assert!(url.ends_with("zip"));
        } else {
            assert!(url.ends_with(".tar.xz"));
        }
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
