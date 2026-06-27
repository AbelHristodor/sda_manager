# Two-Tab UI + Video Downloader — Design

**Date:** 2026-06-27
**Status:** Approved (pending spec review)

## Summary

Restructure the `hymnal-gui` Slint app into a modern two-section interface with a
dark sidebar navigation ("Slate Dark" theme). The first section is the existing
hymn Library (search bar, results list, preview, open/reveal). The second is a
new **Video Downloader** that downloads a YouTube video from a pasted URL into a
user-chosen folder, defaulting to the OS Downloads directory.

The downloader delegates to the `yt-dlp` binary (and `ffmpeg` for merging),
auto-downloaded into the app data directory on first use for zero-setup
cross-platform redistribution.

## Goals

- Modern, good-looking UI with two tabs in a dark sidebar shell.
- Keep the existing Library functionality fully intact.
- Add a simple, reliable YouTube downloader: paste URL → pick folder → download.
- Cross-platform and redistributable without requiring the user to install tools.

## Non-Goals (YAGNI)

- No quality/format picker, no audio-only mode — always best-quality merged video.
- No download queue or batch URLs — one video at a time.
- No playlist expansion — a playlist URL downloads only the referenced single video.
- No light/dark toggle — Slate Dark only.

## Decisions (from brainstorming)

| Topic | Decision |
|---|---|
| Download engine | Auto-download `yt-dlp` into app data dir on first use; **periodic** self-update via `yt-dlp -U` (at most once per day). `ffmpeg` is **auto-downloaded** into the app data dir too (per-OS archive), so best-quality merged video works with zero setup; if the fetch fails, fall back to the best pre-merged single-file format and note "best single-file quality" in the UI. |
| Navigation | Dark vertical **sidebar** rail with brand + two nav items. |
| Visual style | **Slate Dark** — dark slate backgrounds, sky-blue (`#0ea5e9`/`#38bdf8`) accents. |
| Download scope | **Simple** — always best quality video (video+audio merged). |
| Multiple videos | **One at a time** — single URL field. |
| Playlist URL | **Single video only** — ignore the playlist portion. |
| Progress detail | **Rich** — title, progress bar, percent · speed · ETA. |
| Folder memory | **Remember last folder**; default to OS Downloads on first run. |

## Architecture

Preserve the existing two-crate split: `hymnal-core` (pure, testable logic) and
`hymnal-gui` (Slint UI + wiring).

```
crates/
  hymnal-core/src/
    downloader.rs   ← NEW: tool resolution, spawn yt-dlp, progress parsing
    library.rs      ← extend Config with download_dir
  hymnal-gui/
    ui/app.slint    ← restructured: Theme global + Sidebar + LibraryPanel + DownloaderPanel
    src/main.rs     ← add downloader wiring (worker thread + progress channel)
```

### `hymnal-core/src/downloader.rs`

Three responsibilities:

1. **Tool resolution** — locate `yt-dlp` and `ffmpeg`:
   - Look in the app data dir first, then `PATH`.
   - If absent, download the correct binary for the current OS/arch from the
     official GitHub releases into the app data dir (set executable bit on Unix).
   - `yt-dlp` self-updates via its `-U` flag.
2. **Download execution** — spawn `yt-dlp` as a child process:
   - Output template targets the chosen folder.
   - Request best video+audio merged to MP4 (`-f bv*+ba/b --merge-output-format mp4`).
   - `--no-playlist` so a playlist URL grabs only the single video.
   - If `ffmpeg` is unavailable and cannot be fetched, fall back to a pre-merged
     single-file format and report "best single-file quality".
3. **Progress parsing** — run yt-dlp with `--newline` and a `--progress-template`
   emitting machine-readable fields; parse each stdout line into:

```rust
pub struct DownloadProgress {
    pub percent: f32,        // 0.0 – 100.0
    pub speed: String,       // e.g. "3.2 MB/s"
    pub eta: String,         // e.g. "00:18"
    pub title: String,       // video title once known
}

pub enum DownloadEvent {
    Resolving,               // fetching/locating yt-dlp/ffmpeg
    Progress(DownloadProgress),
    Done { path: PathBuf },
    Failed { message: String },
}
```

The download function accepts a `Sender<DownloadEvent>` and streams events back.
The worker thread never touches Slint.

### Config change

Extend `Config` in `library.rs`:

```rust
pub download_dir: Option<String>,   // None => OS Downloads dir
```

`None` resolves to the OS Downloads directory (via `dirs`/`directories`). After a
successful folder pick, persist the chosen path to `config.toml`.

## UI structure (Slint)

`app.slint` is restructured from a monolithic `AppWindow` into composed components:

```
app.slint
├── global Theme               (Slate Dark colors, spacing, radii — single source of truth)
├── struct HymnRow             (existing, unchanged)
├── struct DownloadState       (NEW: status enum-as-int or string, percent, speed, eta, title, message, dir)
├── component Sidebar          (brand + two nav buttons; drives active-tab)
├── component LibraryPanel     (existing Library UI relocated; same property/callback contract)
├── component DownloaderPanel  (URL field, folder picker, Download button, progress area)
└── component AppWindow
      HorizontalBox {
        Sidebar { active-tab <=> root.active-tab; }
        if root.active-tab == 0: LibraryPanel { ...forwarded props/callbacks... }
        if root.active-tab == 1: DownloaderPanel { ...forwarded props/callbacks... }
      }
```

- **Theme global**: Slint has no CSS; a global singleton holds the palette so the
  Slate Dark look is defined once and reused everywhere.
- **Tab switching**: a single `in-out property <int> active-tab` on `AppWindow`.
  Slint's `if` mounts/unmounts the active panel — no manual show/hide.
- **Library panel**: keeps its existing properties and callbacks
  (`results`, `preview-title`, `preview-body`, `status`, `selected-index`,
  `query-changed`, `open-selected`, `reveal-selected`, `selection-changed`),
  forwarded from `AppWindow` so `main.rs` wiring is essentially unchanged.

### DownloaderPanel layout

- Heading "Video Downloader".
- URL `LineEdit`, placeholder "Paste a YouTube URL…".
- Folder row: read-only field showing destination + "Choose…" button (native
  folder dialog via `rfd`).
- "Download" button — disabled while a download runs or the URL is empty.
- Rich progress area: video title, progress bar, `42% · 3.2 MB/s · ETA 00:18`;
  on success a row with "Reveal in folder"; on failure a red error message.

New `AppWindow` members for the downloader:

```
in-out property <int> active-tab: 0;
in property <DownloadState> download;        // current status/progress
in-out property <string> download-url;
in-out property <string> download-dir;       // display path
callback choose-folder();
callback start-download();
callback reveal-download();
```

## Data flow

Reuses the existing worker-thread + channel + 200ms `Timer` poll pattern in
`main.rs`:

```
[Download click] (UI thread)
   main.rs spawns a thread → downloader::download(url, dir, event_tx)
        │  yt-dlp child process, stdout parsed line-by-line
        ▼
   event_tx.send(DownloadEvent::..)  ──►  existing slint Timer (200ms)
                                            drains rx, maps to DownloadState, calls setters
```

The same repeating `Timer` that polls the index channel gains a second `try_recv`
for download events. No async runtime, no second polling mechanism. All UI
mutation stays on the event-loop thread (Slint's threading rule); the worker
communicates only via the channel.

Tool resolution surfaces as a `DownloadEvent::Resolving` → "Setting up
downloader…" state, so the UI needs no special-casing for first-run setup.

## Error handling

- No network / yt-dlp fetch fails → error message in panel, Download re-enabled.
- Invalid URL or yt-dlp non-zero exit → capture stderr tail → "Download failed: <reason>".
- Missing ffmpeg and cannot fetch → fall back to pre-merged format, note "best single-file quality".
- Destination folder not writable → error before spawning the child process.

## Dependencies

- `rfd` — native folder picker dialog.
- `dirs` (or reuse `directories`) — OS Downloads directory resolution.
- Runtime binaries (not crates): `yt-dlp`, `ffmpeg`, auto-downloaded into the app data dir.

## Testing

In `hymnal-core` (no network):

- **Progress parsing** — feed captured yt-dlp `--progress-template` output lines;
  assert parsed `DownloadProgress` fields (percent, speed, eta, title).
- **Config round-trip** — extend existing test to confirm `download_dir` survives
  TOML serialize/deserialize.
- **Tool-path resolution** — given a fake data dir with and without the binary
  present, assert the resolver picks the right path (data dir vs `PATH`).

The live network download is **not** unit-tested (it hits YouTube); it is verified
manually.

## Open questions

None — all resolved during brainstorming.
