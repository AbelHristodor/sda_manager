# Two-Tab UI + Video Downloader Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restructure the Slint GUI into a dark two-tab sidebar app (Library + Video Downloader) and add a yt-dlp-backed YouTube downloader that saves best-quality video to a user-chosen folder.

**Architecture:** Download logic lives in `hymnal-core` as a pure, testable `downloader` module that resolves/auto-downloads the `yt-dlp` binary, spawns it as a child process, and streams parsed progress events over a channel. The Slint UI is restructured into a `Theme` global plus `Sidebar`, `LibraryPanel`, and `DownloaderPanel` components. `main.rs` reuses the existing worker-thread + 200ms `Timer` poll to bridge download events to UI properties on the event-loop thread.

**Tech Stack:** Rust, Slint 1.8, `yt-dlp`/`ffmpeg` (runtime binaries auto-fetched), `rfd` (folder dialog), `directories`/`dirs` (paths), `ureq` (binary download), serde/toml (config).

---

## File Structure

- `crates/hymnal-core/src/downloader.rs` — **NEW**: tool resolution, progress parsing, download execution. One responsibility: turn a URL + folder into a stream of `DownloadEvent`s.
- `crates/hymnal-core/src/lib.rs` — **MODIFY**: register `pub mod downloader;`.
- `crates/hymnal-core/src/library.rs` — **MODIFY**: add `download_dir: Option<String>` to `Config`; add `downloads_dir()` helper.
- `crates/hymnal-core/Cargo.toml` — **MODIFY**: add `ureq`, `dirs`.
- `crates/hymnal-gui/ui/app.slint` — **MODIFY**: restructure into `Theme` global + `Sidebar` + `LibraryPanel` + `DownloaderPanel` + `AppWindow`.
- `crates/hymnal-gui/src/main.rs` — **MODIFY**: add downloader wiring (thread + progress channel drained by the existing Timer).
- `crates/hymnal-gui/Cargo.toml` — **MODIFY**: add `rfd`.

---

## Task 1: Add `download_dir` to Config

**Files:**
- Modify: `crates/hymnal-core/src/library.rs`
- Modify: `crates/hymnal-core/Cargo.toml`

- [ ] **Step 1: Add `dirs` dependency**

In `crates/hymnal-core/Cargo.toml`, under `[dependencies]`, add:

```toml
dirs = "5"
```

- [ ] **Step 2: Write the failing test**

Add to the `tests` module at the bottom of `crates/hymnal-core/src/library.rs`:

```rust
    #[test]
    fn config_persists_download_dir() {
        let cfg = Config {
            default_repo_url: "https://example.com/hymns.git".into(),
            libraries: vec![],
            download_dir: Some("/home/user/Videos".into()),
        };
        let back = Config::from_toml(&cfg.to_toml().unwrap()).unwrap();
        assert_eq!(back.download_dir, Some("/home/user/Videos".into()));
    }

    #[test]
    fn config_download_dir_defaults_to_none() {
        let cfg = Config::default();
        assert_eq!(cfg.download_dir, None);
    }
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p hymnal-core config_persists_download_dir`
Expected: FAIL — compile error, `Config` has no field `download_dir`, and existing struct literals in `library.rs` are missing the field.

- [ ] **Step 4: Add the field and helper**

In `crates/hymnal-core/src/library.rs`, add the field to `Config`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub default_repo_url: String,
    pub libraries: Vec<Library>,
    /// User's chosen download folder. `None` => OS Downloads directory.
    #[serde(default)]
    pub download_dir: Option<String>,
}
```

Update the `Default` impl to include the field:

```rust
impl Default for Config {
    fn default() -> Self {
        Config {
            default_repo_url: DEFAULT_REPO_URL.to_string(),
            libraries: Vec::new(),
            download_dir: None,
        }
    }
}
```

Update the existing `config_toml_round_trips` test's struct literal to add `download_dir: None,`.

Add a helper function near the other path helpers:

```rust
/// Resolve the effective download directory: the configured one, or the OS
/// Downloads folder, or the home dir as a last resort.
pub fn downloads_dir(cfg: &Config) -> std::path::PathBuf {
    if let Some(d) = &cfg.download_dir {
        return std::path::PathBuf::from(d);
    }
    dirs::download_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p hymnal-core library`
Expected: PASS — all `library` module tests green.

- [ ] **Step 6: Commit**

```bash
git add crates/hymnal-core/src/library.rs crates/hymnal-core/Cargo.toml
git commit -m "feat(core): add download_dir to Config with Downloads fallback"
```

---

## Task 2: Progress parsing in downloader module

We run yt-dlp with a fixed progress template so each progress line looks like:

```
PROGRESS|42.0%|3.20MiB/s|00:18
```

(Template: `download:PROGRESS|%(progress._percent_str)s|%(progress._speed_str)s|%(progress._eta_str)s`.)

**Files:**
- Create: `crates/hymnal-core/src/downloader.rs`
- Modify: `crates/hymnal-core/src/lib.rs`

- [ ] **Step 1: Register the module**

In `crates/hymnal-core/src/lib.rs`, add (keep alphabetical-ish ordering with the others):

```rust
pub mod downloader;
```

- [ ] **Step 2: Write the failing test**

Create `crates/hymnal-core/src/downloader.rs` with only the types and a parser stub plus tests:

```rust
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
/// line emitted by our template (`PROGRESS|<pct>|<speed>|<eta>`). Returns
/// `None` for any other line.
pub fn parse_progress_line(line: &str) -> Option<DownloadProgress> {
    todo!()
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
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test -p hymnal-core parses_a_progress_line`
Expected: FAIL — `parse_progress_line` panics with `not yet implemented` (todo!).

- [ ] **Step 4: Implement the parser**

Replace the `parse_progress_line` body in `crates/hymnal-core/src/downloader.rs`:

```rust
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
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p hymnal-core downloader`
Expected: PASS — all four parser tests green.

- [ ] **Step 6: Commit**

```bash
git add crates/hymnal-core/src/downloader.rs crates/hymnal-core/src/lib.rs
git commit -m "feat(core): add downloader event types and progress-line parser"
```

---

## Task 3: Tool resolution (locate yt-dlp/ffmpeg)

**Files:**
- Modify: `crates/hymnal-core/src/downloader.rs`

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `crates/hymnal-core/src/downloader.rs`:

```rust
    use std::fs;

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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p hymnal-core resolves_binary_in_data_dir_first`
Expected: FAIL — `resolve_in_dir` / `binary_name` not defined.

- [ ] **Step 3: Implement the resolvers**

Add to `crates/hymnal-core/src/downloader.rs` (module level, above `tests`):

```rust
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p hymnal-core downloader`
Expected: PASS — parser + resolver tests all green.

- [ ] **Step 5: Commit**

```bash
git add crates/hymnal-core/src/downloader.rs
git commit -m "feat(core): add yt-dlp/ffmpeg tool resolution helpers"
```

---

## Task 4: Auto-download yt-dlp and run the download

This task adds the networked pieces: fetching the yt-dlp binary if missing, and
spawning it to perform a download while streaming events. The network paths are
not unit-tested (they hit GitHub/YouTube); they are verified manually in Task 7.

**Files:**
- Modify: `crates/hymnal-core/src/downloader.rs`
- Modify: `crates/hymnal-core/Cargo.toml`

- [ ] **Step 1: Add the HTTP dependency**

In `crates/hymnal-core/Cargo.toml`, under `[dependencies]`, add:

```toml
ureq = "2"
```

- [ ] **Step 2: Implement ensure-yt-dlp (download if missing)**

Add to `crates/hymnal-core/src/downloader.rs`:

```rust
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
    let resp = ureq::get(&url).call()?;
    let mut reader = resp.into_reader();
    let mut file = std::fs::File::create(&dest)?;
    std::io::copy(&mut reader, &mut file)?;
    drop(file);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dest)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dest, &perms)?;
    }
    Ok(dest)
}
```

- [ ] **Step 3: Implement the download runner**

Add to `crates/hymnal-core/src/downloader.rs`:

```rust
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::mpsc::Sender;

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
    let stderr_tail = child
        .stderr
        .take()
        .map(|e| {
            BufReader::new(e)
                .lines()
                .map_while(Result::ok)
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();

    match status {
        Ok(s) if s.success() => {
            let _ = tx.send(DownloadEvent::Done {
                path: dir.to_path_buf(),
            });
        }
        Ok(_) => {
            let msg = stderr_tail
                .lines()
                .rev()
                .find(|l| l.contains("ERROR"))
                .unwrap_or("download failed")
                .to_string();
            let _ = tx.send(DownloadEvent::Failed { message: msg });
        }
        Err(e) => {
            let _ = tx.send(DownloadEvent::Failed {
                message: format!("yt-dlp did not run: {e}"),
            });
        }
    }
}
```

- [ ] **Step 4: Verify it compiles and existing tests still pass**

Run: `cargo test -p hymnal-core`
Expected: PASS — no network test added; all existing + parser/resolver tests green. The crate compiles with the new `download`/`ensure_ytdlp` functions.

- [ ] **Step 5: Commit**

```bash
git add crates/hymnal-core/src/downloader.rs crates/hymnal-core/Cargo.toml
git commit -m "feat(core): auto-download yt-dlp and run a video download with progress"
```

---

## Task 5: Restructure the Slint UI (Theme + Sidebar + two panels)

**Files:**
- Modify: `crates/hymnal-gui/ui/app.slint`

> **IMPORTANT — current code differs from earlier plan drafts.** The Library UI
> already uses `StandardListView` with keyboard navigation (arrow keys, Enter to
> open), a `FocusScope`, and these exact member names:
> `results: [StandardListViewItem]`, `preview-title`, `preview-body`, `status`,
> `current-index` (in-out), callbacks `query-changed`, `open-current`,
> `reveal-current`, `current-changed`. **These names and the keyboard nav MUST be
> preserved exactly** — `main.rs` depends on them and they are real UX. This task
> WRAPS the existing library content in a sidebar shell; it does NOT rename
> members or replace `StandardListView` with custom rows.

- [ ] **Step 1: Replace `app.slint` with the restructured UI**

Overwrite `crates/hymnal-gui/ui/app.slint` with the following. Note the Library
content is the existing markup moved verbatim into a `LibraryPanel` component
(same `StandardListView`, same `FocusScope` keyboard handling, same member
names), now styled with the Slate Dark `Theme` and wrapped by a `Sidebar`.

```slint
import { LineEdit, StandardListView, Button, ScrollView, VerticalBox, HorizontalBox } from "std-widgets.slint";

export struct DownloadState {
    // "" idle | "resolving" | "downloading" | "done" | "failed"
    status: string,
    percent: float,
    speed: string,
    eta: string,
    title: string,
    message: string,
}

global Theme {
    out property <color> bg: #0f172a;
    out property <color> rail: #111827;
    out property <color> panel: #162033;
    out property <color> panel-border: #1f2c44;
    out property <color> field: #1e293b;
    out property <color> field-border: #334155;
    out property <color> accent: #0ea5e9;
    out property <color> accent-soft: #38bdf8;
    out property <color> text: #e2e8f0;
    out property <color> text-dim: #94a3b8;
    out property <color> nav-sel: #1e293b;
    out property <length> radius: 9px;
    out property <length> gap: 12px;
}

component NavItem inherits Rectangle {
    in property <string> label;
    in property <bool> selected;
    callback clicked();
    height: 40px;
    border-radius: Theme.radius;
    background: selected ? Theme.nav-sel : transparent;
    HorizontalBox {
        padding-left: 12px;
        alignment: start;
        spacing: 10px;
        Rectangle {
            width: 16px; height: 16px;
            border-radius: 5px;
            background: selected ? Theme.accent : Theme.accent-soft;
            opacity: selected ? 1.0 : 0.6;
        }
        Text {
            text: label;
            color: selected ? Theme.text : Theme.text-dim;
            font-weight: selected ? 700 : 400;
            vertical-alignment: center;
        }
    }
    TouchArea { clicked => { root.clicked(); } }
}

component Sidebar inherits Rectangle {
    in-out property <int> active-tab;
    width: 150px;
    background: Theme.rail;
    VerticalBox {
        padding: 14px;
        spacing: 6px;
        alignment: start;
        Text {
            text: "SDA MANAGER";
            color: Theme.accent-soft;
            font-size: 12px;
            font-weight: 700;
            letter-spacing: 0.5px;
        }
        Rectangle { height: 8px; }
        NavItem {
            label: "Library";
            selected: root.active-tab == 0;
            clicked => { root.active-tab = 0; }
        }
        NavItem {
            label: "Video Downloader";
            selected: root.active-tab == 1;
            clicked => { root.active-tab = 1; }
        }
    }
}

// Library panel: existing fzf-style finder with keyboard nav, preserved exactly,
// wrapped in the dark theme. Member names match what main.rs expects.
component LibraryPanel inherits Rectangle {
    in property <[StandardListViewItem]> results;
    in property <string> preview-title;
    in property <string> preview-body;
    in property <string> status;
    in-out property <int> current-index;

    callback query-changed(string);
    callback open-current();
    callback reveal-current();
    callback current-changed(int);

    // Typing focus lives in the search field; the surrounding FocusScope catches
    // arrow keys the single-line field ignores and drives the list.
    forward-focus: search;
    background: Theme.bg;

    key-handler := FocusScope {
        key-pressed(event) => {
            if (event.text == Key.UpArrow) {
                if (root.current-index > 0) {
                    list.set-current-item(root.current-index - 1);
                }
                return accept;
            }
            if (event.text == Key.DownArrow) {
                if (root.current-index < root.results.length - 1) {
                    list.set-current-item(root.current-index + 1);
                }
                return accept;
            }
            if (event.text == Key.Return) {
                root.open-current();
                return accept;
            }
            return reject;
        }

        VerticalBox {
            padding: 16px;
            spacing: Theme.gap;
            Text { text: root.status; color: Theme.text-dim; }

            search := LineEdit {
                placeholder-text: "Search by number, title, or lyrics…";
                edited(text) => { root.query-changed(text); }
                accepted(text) => { root.open-current(); }
            }

            HorizontalBox {
                spacing: Theme.gap;
                list := StandardListView {
                    width: 40%;
                    model: root.results;
                    current-item <=> root.current-index;
                    current-item-changed(index) => {
                        root.current-changed(index);
                    }
                }

                Rectangle {
                    background: Theme.panel;
                    border-radius: Theme.radius;
                    border-width: 1px;
                    border-color: Theme.panel-border;
                    VerticalBox {
                        padding: 14px;
                        spacing: 10px;
                        Text {
                            text: root.preview-title;
                            color: Theme.text;
                            font-size: 18px;
                            font-weight: 700;
                            wrap: word-wrap;
                        }
                        ScrollView {
                            Text {
                                text: root.preview-body;
                                color: Theme.text;
                                wrap: word-wrap;
                            }
                        }
                        HorizontalBox {
                            height: 40px;
                            alignment: start;
                            spacing: 8px;
                            Button {
                                text: "Open in PowerPoint";
                                clicked => { root.open-current(); }
                            }
                            Button {
                                text: "Reveal in folder";
                                clicked => { root.reveal-current(); }
                            }
                        }
                    }
                }
            }
        }
    }
}

component DownloaderPanel inherits Rectangle {
    in property <DownloadState> state;
    in-out property <string> url;
    in property <string> dir;
    callback choose-folder();
    callback start-download();
    callback reveal-download();

    property <bool> busy: root.state.status == "resolving" || root.state.status == "downloading";

    background: Theme.bg;
    VerticalBox {
        padding: 16px;
        spacing: Theme.gap;
        alignment: start;
        Text {
            text: "Video Downloader";
            color: Theme.text;
            font-size: 20px;
            font-weight: 700;
        }
        Text {
            text: "Paste a YouTube link and choose where to save it.";
            color: Theme.text-dim;
        }
        LineEdit {
            placeholder-text: "Paste a YouTube URL…";
            text <=> root.url;
            enabled: !root.busy;
        }
        HorizontalBox {
            spacing: 8px;
            Rectangle {
                background: Theme.field;
                border-radius: Theme.radius;
                border-width: 1px;
                border-color: Theme.field-border;
                height: 36px;
                horizontal-stretch: 1;
                Text {
                    x: 12px;
                    width: parent.width - 24px;
                    text: root.dir;
                    color: Theme.text-dim;
                    vertical-alignment: center;
                    overflow: elide;
                }
            }
            Button {
                text: "Choose…";
                enabled: !root.busy;
                clicked => { root.choose-folder(); }
            }
        }
        Button {
            text: root.busy ? "Downloading…" : "Download";
            enabled: !root.busy && root.url != "";
            clicked => { root.start-download(); }
        }

        if root.state.status != "": Rectangle {
            background: Theme.panel;
            border-radius: Theme.radius;
            border-width: 1px;
            border-color: Theme.panel-border;
            VerticalBox {
                padding: 14px;
                spacing: 8px;
                Text {
                    text: root.state.title != "" ? root.state.title : "Preparing…";
                    color: Theme.text;
                    font-weight: 600;
                    overflow: elide;
                }
                if root.state.status == "resolving": Text {
                    text: "Setting up downloader…";
                    color: Theme.text-dim;
                }
                if root.state.status == "downloading": Rectangle {
                    height: 8px;
                    border-radius: 4px;
                    background: Theme.field;
                    Rectangle {
                        x: 0;
                        height: parent.height;
                        width: parent.width * (root.state.percent / 100);
                        border-radius: 4px;
                        background: Theme.accent;
                    }
                }
                if root.state.status == "downloading": Text {
                    text: Math.round(root.state.percent) + "%  ·  " + root.state.speed + "  ·  ETA " + root.state.eta;
                    color: Theme.text-dim;
                    font-size: 12px;
                }
                if root.state.status == "done": HorizontalBox {
                    alignment: start;
                    spacing: 10px;
                    Text { text: "✓ Download complete"; color: Theme.accent-soft; vertical-alignment: center; }
                    Button {
                        text: "Reveal in folder";
                        clicked => { root.reveal-download(); }
                    }
                }
                if root.state.status == "failed": Text {
                    text: "Download failed: " + root.state.message;
                    color: #f87171;
                    wrap: word-wrap;
                }
            }
        }
    }
}

export component AppWindow inherits Window {
    title: "SDA Manager";
    preferred-width: 980px;
    preferred-height: 620px;
    background: Theme.bg;

    in-out property <int> active-tab: 0;

    // Library (existing contract — names unchanged)
    in property <[StandardListViewItem]> results;
    in property <string> preview-title;
    in property <string> preview-body;
    in property <string> status;
    in-out property <int> current-index: -1;
    callback query-changed(string);
    callback open-current();
    callback reveal-current();
    callback current-changed(int);

    // Downloader
    in property <DownloadState> download;
    in-out property <string> download-url;
    in property <string> download-dir;
    callback choose-folder();
    callback start-download();
    callback reveal-download();

    HorizontalBox {
        padding: 0;
        spacing: 0;
        Sidebar { active-tab <=> root.active-tab; }
        if root.active-tab == 0: LibraryPanel {
            horizontal-stretch: 1;
            results: root.results;
            preview-title: root.preview-title;
            preview-body: root.preview-body;
            status: root.status;
            current-index <=> root.current-index;
            query-changed(q) => { root.query-changed(q); }
            open-current => { root.open-current(); }
            reveal-current => { root.reveal-current(); }
            current-changed(i) => { root.current-changed(i); }
        }
        if root.active-tab == 1: DownloaderPanel {
            horizontal-stretch: 1;
            state: root.download;
            url <=> root.download-url;
            dir: root.download-dir;
            choose-folder => { root.choose-folder(); }
            start-download => { root.start-download(); }
            reveal-download => { root.reveal-download(); }
        }
    }
}
```

- [ ] **Step 2: Verify the UI compiles**

Run: `cargo build -p hymnal-gui`
Expected: build FAILS only with Rust errors in `main.rs` about the new
downloader callbacks/properties not being handled — NOT Slint parse errors. The
existing library callbacks (`query-changed`, `open-current`, etc.) are unchanged,
so they still wire up. If you see `.slint` syntax errors, fix the file before
proceeding.

Note: forwarding `current-index <=> root.current-index` two-way through the
`LibraryPanel` boundary preserves the keyboard-nav behavior; the `FocusScope`
lives inside `LibraryPanel` and calls `list.set-current-item(...)` exactly as
before.

- [ ] **Step 3: Commit**

```bash
git add crates/hymnal-gui/ui/app.slint
git commit -m "feat(gui): wrap library in dark sidebar shell, add downloader panel"
```

---

## Task 6: Wire the downloader in main.rs

**Files:**
- Modify: `crates/hymnal-gui/src/main.rs`
- Modify: `crates/hymnal-gui/Cargo.toml`

> The existing library wiring (`on_query_changed`, `on_current_changed`,
> `on_open_current`, `on_reveal_current`, the worker thread, and the 200ms Timer)
> is UNCHANGED. This task only ADDS downloader wiring. Do not rename or remove any
> existing handler.

- [ ] **Step 1: Add the folder-dialog dependency**

In `crates/hymnal-gui/Cargo.toml`, under `[dependencies]`, add:

```toml
rfd = "0.14"
```

- [ ] **Step 2: Extend imports**

In `crates/hymnal-gui/src/main.rs`, add the downloader import and extend the
`library` import to include `downloads_dir`. The current import line is:

```rust
use hymnal_core::library::{default_library_dir, index_cache_path, Config, Library};
```

Replace it with:

```rust
use hymnal_core::downloader::{self, DownloadEvent};
use hymnal_core::library::{
    default_library_dir, downloads_dir, index_cache_path, Config, Library,
};
```

- [ ] **Step 3: Initialize config handle, download dir, and event channel**

Immediately after `let ui = AppWindow::new()?;` (around line 30), add:

```rust
    // Shared config so folder choices persist across the session and to disk.
    let cfg_path = hymnal_core::library::config_path();
    let dl_cfg = std::rc::Rc::new(std::cell::RefCell::new(
        cfg_path
            .as_ref()
            .map(|p| Config::load(p).unwrap_or_default())
            .unwrap_or_default(),
    ));
    let initial_dir = downloads_dir(&dl_cfg.borrow());
    ui.set_download_dir(initial_dir.to_string_lossy().to_string().into());

    // Channel carrying download events from the worker thread to the UI thread.
    let (dl_tx, dl_rx) = mpsc::channel::<DownloadEvent>();
```

(The worker thread below loads its own `Config` independently for indexing; this
separate `dl_cfg` handle is only for the download folder. That duplication is
acceptable and avoids entangling the indexing thread's ownership.)

- [ ] **Step 4: Drain download events inside the existing Timer**

The existing Timer closure currently contains only the index `rx.try_recv()`
block. Add a second drain loop inside the SAME closure, right after the existing
`if let Ok(entries) = rx.try_recv() { ... }` block and before the closure's
closing `},`:

```rust
            while let Ok(ev) = dl_rx.try_recv() {
                if let Some(ui) = weak2.upgrade() {
                    let mut s = ui.get_download();
                    match ev {
                        DownloadEvent::Resolving => {
                            s.status = "resolving".into();
                        }
                        DownloadEvent::Title(t) => {
                            s.title = t.into();
                        }
                        DownloadEvent::Progress(p) => {
                            s.status = "downloading".into();
                            s.percent = p.percent;
                            s.speed = p.speed.into();
                            s.eta = p.eta.into();
                        }
                        DownloadEvent::Done { .. } => {
                            s.status = "done".into();
                            s.percent = 100.0;
                        }
                        DownloadEvent::Failed { message } => {
                            s.status = "failed".into();
                            s.message = message.into();
                        }
                    }
                    ui.set_download(s);
                }
            }
```

`weak2` is already captured by this closure (used for the index channel), so it
is in scope. `dl_rx` is moved into the closure, which is already `move`.

- [ ] **Step 5: Wire choose-folder**

Before the `let _timer = timer;` line near the end of `main`, add:

```rust
    let weak_choose = ui.as_weak();
    let cfg_choose = dl_cfg.clone();
    let cfg_path_choose = cfg_path.clone();
    ui.on_choose_folder(move || {
        let Some(ui) = weak_choose.upgrade() else { return };
        let start = ui.get_download_dir().to_string();
        if let Some(folder) = rfd::FileDialog::new()
            .set_directory(if start.is_empty() { ".".into() } else { start })
            .pick_folder()
        {
            let s = folder.to_string_lossy().to_string();
            ui.set_download_dir(s.clone().into());
            cfg_choose.borrow_mut().download_dir = Some(s);
            if let Some(p) = &cfg_path_choose {
                if let Err(e) = cfg_choose.borrow().save(p) {
                    warn!("failed to save config: {e}");
                }
            }
        }
    });
```

- [ ] **Step 6: Wire start-download**

Add after the choose-folder handler:

```rust
    let weak_start = ui.as_weak();
    let dl_tx_start = dl_tx.clone();
    ui.on_start_download(move || {
        let Some(ui) = weak_start.upgrade() else { return };
        let url = ui.get_download_url().to_string();
        if url.trim().is_empty() {
            return;
        }
        let dir = std::path::PathBuf::from(ui.get_download_dir().to_string());
        info!("starting download: {url} -> {}", dir.display());
        // Show "resolving" immediately for responsiveness.
        ui.set_download(DownloadState {
            status: "resolving".into(),
            title: "".into(),
            message: "".into(),
            speed: "".into(),
            eta: "".into(),
            percent: 0.0,
        });
        let tx = dl_tx_start.clone();
        std::thread::spawn(move || {
            downloader::download(&url, &dir, &tx);
        });
    });
```

- [ ] **Step 7: Wire reveal-download**

Add after the start-download handler:

```rust
    let weak_reveal = ui.as_weak();
    ui.on_reveal_download(move || {
        let Some(ui) = weak_reveal.upgrade() else { return };
        let dir = ui.get_download_dir().to_string();
        if !dir.is_empty() {
            if let Err(e) = open::that(&dir) {
                warn!("failed to reveal {dir}: {e}");
            }
        }
    });
```

- [ ] **Step 8: Build**

Run: `cargo build`
Expected: PASS — compiles cleanly. (`DownloadState` is generated from the Slint
struct; `info!`/`warn!` are already imported.)

- [ ] **Step 9: Manual verification of a real download**

Run: `cargo run -p hymnal-gui`
1. Confirm sidebar shows Library + Video Downloader; Library search, arrow-key
   navigation, Enter-to-open, and both buttons still work (regression check).
2. Switch to Video Downloader. Confirm the folder defaults to your Downloads dir.
3. Paste a short public YouTube URL, click Download.
4. Confirm: "Setting up downloader…" on first run (yt-dlp fetch), then a title,
   then a moving progress bar with percent · speed · ETA, then "✓ Download
   complete".
5. Click "Reveal in folder" and confirm the video file is present and plays.
6. Choose a different folder, restart the app, confirm it persisted.

Expected: video downloads to the chosen folder; folder choice survives restart;
library is regression-free.

- [ ] **Step 10: Commit**

```bash
git add crates/hymnal-gui/src/main.rs crates/hymnal-gui/Cargo.toml
git commit -m "feat(gui): wire video downloader tab to yt-dlp worker with live progress"
```

---

## Task 7: Final verification

- [ ] **Step 1: Run the full test suite**

Run: `cargo test`
Expected: PASS — all `hymnal-core` tests (library, downloader parser/resolver, plus existing pptx/index/search/sync) green.

- [ ] **Step 2: Lint**

Run: `cargo clippy --all-targets`
Expected: no errors (warnings acceptable if pre-existing).

- [ ] **Step 3: Confirm Library regression-free**

Launch the app, verify search, selection, preview, "Open in PowerPoint", and
"Reveal in folder" all still work as before the restructure.

---

## Notes / Decisions baked in

- **ffmpeg is detected, not auto-downloaded in v1.** If absent, the download
  falls back to the best pre-merged single-file format (`-f b`) so it still
  works. Auto-downloading ffmpeg is deferred (heavier, platform-specific
  archives) — the spec allowed this fallback path. If full-quality merging is
  required without a system ffmpeg, add an `ensure_ffmpeg()` mirroring
  `ensure_ytdlp()` in a follow-up.
- **Single video only:** `--no-playlist` guarantees a playlist URL grabs just the
  referenced video.
- **Threading rule:** the worker thread only sends over `dl_tx`; every UI mutation
  happens in the Timer callback on the event-loop thread.
```

