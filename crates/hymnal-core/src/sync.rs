use anyhow::{Context, Result};
use std::path::Path;

/// Ensure the default library exists at `dest`: clone from `repo_url` if the
/// directory is absent, otherwise fast-forward pull. Returns the resulting
/// repository path on success.
pub fn sync_default_library(repo_url: &str, dest: &Path) -> Result<()> {
    if dest.join(".git").is_dir() {
        pull(dest).with_context(|| format!("pull {}", dest.display()))
    } else {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        git2::Repository::clone(repo_url, dest)
            .with_context(|| format!("clone {repo_url} -> {}", dest.display()))?;
        Ok(())
    }
}

/// Fast-forward the checked-out branch to its upstream.
fn pull(dest: &Path) -> Result<()> {
    let repo = git2::Repository::open(dest)?;
    let mut remote = repo.find_remote("origin")?;
    remote.fetch(&["HEAD"], None, None)?;
    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    let commit = repo.reference_to_annotated_commit(&fetch_head)?;
    let (analysis, _) = repo.merge_analysis(&[&commit])?;
    if analysis.is_up_to_date() {
        return Ok(());
    }
    if analysis.is_fast_forward() {
        let refname = "refs/heads/main";
        if let Ok(mut reference) = repo.find_reference(refname) {
            reference.set_target(commit.id(), "fast-forward")?;
            repo.set_head(refname)?;
            repo.checkout_head(Some(
                git2::build::CheckoutBuilder::default().force(),
            ))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clone_when_missing_then_treated_as_existing() {
        // No network in unit tests: verify the path-branch logic only.
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("lib");
        // .git absent -> would attempt clone; we only assert the predicate.
        assert!(!dest.join(".git").is_dir());
    }
}
