# Release CI & Install Scripts — Design

Date: 2026-06-27

## Goal

Provide downloadable, ready-to-run executables of the Hymn Finder GUI
(`hymnal-gui`) for end users on macOS, Windows, and Linux, plus a one-liner
install command per platform.

## Trigger

Manual only: `workflow_dispatch` with a required `version` input (e.g.
`v0.1.0`). That value becomes the Git tag and the GitHub Release name. No
automatic build on tag push.

Implication: no release exists until the workflow is run once. The install
one-liners fetch `releases/latest`, so they only work after the first manual
release.

## Build matrix (native runners, no cross-compilation)

| Rust target               | Runner          | Archive                                          |
|---------------------------|-----------------|--------------------------------------------------|
| `aarch64-apple-darwin`    | `macos-14`      | `hymnal-gui-aarch64-apple-darwin.tar.gz`         |
| `x86_64-pc-windows-msvc`  | `windows-latest`| `hymnal-gui-x86_64-pc-windows-msvc.zip`          |
| `x86_64-unknown-linux-gnu`| `ubuntu-latest` | `hymnal-gui-x86_64-unknown-linux-gnu.tar.gz`     |

Native runners avoid the mingw-w64 / Slint-renderer issues that cross-compiling
would introduce. Each binary is built by its platform's own toolchain.

### Per-job steps

1. Checkout.
2. Install Rust stable + the job's target (`dtolnay/rust-toolchain`).
3. Cache cargo registry + target dir (`Swatinem/rust-cache`).
4. Linux only: install Slint runtime/build deps
   (`libxcb`, `libxkbcommon`, fontconfig, etc.).
5. `cargo build --release -p hymnal-gui --target <target>`.
6. Package the binary (`hymnal-gui` / `hymnal-gui.exe`) into the archive
   (`.tar.gz` on unix, `.zip` on Windows).
7. Upload the archive as a build artifact.

### Release job

Depends on all matrix jobs. Downloads every artifact and publishes one GitHub
Release at the input `version` using `softprops/action-gh-release`, attaching
all three archives. Requires `permissions: contents: write`.

## `git2` change

Enable the `vendored-libgit2` feature in `crates/hymnal-core/Cargo.toml` so the
released binaries do not depend on a system libgit2. This makes the downloaded
executables self-contained and matches the README's existing recommendation for
portable builds.

## Install scripts

Both download from `https://github.com/AbelHristodor/sda_manager/releases/latest/download/<asset>`.

- `install.sh` (macOS/Linux): detect OS + arch, map to the asset name, download
  and extract `hymnal-gui` into `${BIN_DIR:-$HOME/.local/bin}`, `chmod +x`, warn
  if that dir is not on `PATH`. Errors clearly on unsupported OS/arch
  (e.g. Intel macOS, which is not built).
- `install.ps1` (Windows): download the `.zip`, extract `hymnal-gui.exe` into
  `%LOCALAPPDATA%\Programs\hymnal-gui`, add that dir to the user `PATH` if
  missing.

## README

New "Install" section with both one-liners:

- macOS/Linux: `curl -fsSL https://raw.githubusercontent.com/AbelHristodor/sda_manager/main/install.sh | sh`
- Windows (PowerShell): `irm https://raw.githubusercontent.com/AbelHristodor/sda_manager/main/install.ps1 | iex`

Document that macOS binaries are unsigned (first launch needs right-click →
Open to pass Gatekeeper) and that releases must be produced manually first.

## Out of scope (YAGNI)

- Code signing / notarization (macOS) and Authenticode (Windows).
- Intel macOS and ARM Linux targets.
- Homebrew tap / winget / package managers.
- Auto-release on tag push.
