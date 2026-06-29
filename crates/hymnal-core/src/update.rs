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
    /// The check could not be performed for a transient reason (offline,
    /// GitHub API rate limit / 403, timeout). Not a real error — try later.
    CheckUnavailable,
}

/// Classify an update-check error message as transient (network/rate-limit)
/// vs. a genuine failure. Transient ones are expected and should be reported
/// calmly ("try later"), not as alarming failures. Matched on the message
/// text because `self_update` erases the underlying error type.
pub fn is_transient_failure(msg: &str) -> bool {
    let m = msg.to_lowercase();
    // GitHub rate limit (403) / too-many-requests (429), and its prose form.
    m.contains("403")
        || m.contains("429")
        || m.contains("rate limit")
        // Common offline/network phrasings from ureq/std.
        || m.contains("dns")
        || m.contains("timed out")
        || m.contains("timeout")
        || m.contains("unreachable")
        || m.contains("connection")
}

/// Check releases and, if a newer version exists, download + stage it.
/// Returns the outcome; errors are returned (callers on boot should log+ignore).
pub fn check_and_stage_update() -> Result<UpdateOutcome> {
    let current = env!("CARGO_PKG_VERSION");
    let result = (|| {
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
        anyhow::Ok(status)
    })();

    match result {
        Ok(status) if status.updated() => Ok(UpdateOutcome::Updated {
            version: status.version().to_string(),
        }),
        Ok(_) => Ok(UpdateOutcome::UpToDate),
        // Transient (offline / rate-limit / 403) → soft outcome, not an error.
        Err(e) if is_transient_failure(&e.to_string()) => Ok(UpdateOutcome::CheckUnavailable),
        Err(e) => Err(e),
    }
}
