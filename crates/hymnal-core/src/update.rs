//! Binary self-update from GitHub Releases. Compares the running version
//! (`CARGO_PKG_VERSION`) against the latest release tag and, if newer,
//! downloads the asset for this target triple and replaces the executable in
//! place. The running process keeps old code until the next restart.

use anyhow::Result;

const REPO_OWNER: &str = "AbelHristodor";
const REPO_NAME: &str = "sda_manager";
const BIN_NAME: &str = "hymnal-gui";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateOutcome {
    UpToDate,
    Updated { version: String },
}

/// Check releases and, if a newer version exists, download + stage it.
/// Returns the outcome; errors are returned (callers on boot should log+ignore).
pub fn check_and_stage_update() -> Result<UpdateOutcome> {
    let current = env!("CARGO_PKG_VERSION");
    let status = self_update::backends::github::Update::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .bin_name(BIN_NAME)
        // Release archives nest the binary one level deep, as
        // `hymnal-gui-<target>/hymnal-gui` (see .github/workflows/release.yml).
        // Without this, self_update looks for `hymnal-gui` at the archive root
        // and fails with "Could not find the required path in the archive".
        .bin_path_in_archive("{{ bin }}-{{ target }}/{{ bin }}")
        .current_version(current)
        .no_confirm(true)
        .show_download_progress(false)
        .show_output(false)
        .build()?
        .update()?;

    if status.updated() {
        Ok(UpdateOutcome::Updated {
            version: status.version().to_string(),
        })
    } else {
        Ok(UpdateOutcome::UpToDate)
    }
}
