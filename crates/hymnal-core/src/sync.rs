use anyhow::{Context, Result};
use std::path::Path;

/// What `sync_default_library` did, so callers can skip re-indexing when the
/// local library is already up to date.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncOutcome {
    /// The repository was freshly cloned (was absent).
    Cloned,
    /// An existing clone was fast-forwarded to new commits.
    Updated,
    /// An existing clone was already up to date; nothing changed.
    Unchanged,
}

/// Ensure the default library exists at `dest`: clone from `repo_url` if the
/// directory is absent, otherwise fast-forward pull. Reports what happened.
pub fn sync_default_library(repo_url: &str, dest: &Path) -> Result<SyncOutcome> {
    if dest.join(".git").is_dir() {
        pull(dest).with_context(|| format!("pull {}", dest.display()))
    } else {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        git2::Repository::clone(repo_url, dest)
            .with_context(|| format!("clone {repo_url} -> {}", dest.display()))?;
        Ok(SyncOutcome::Cloned)
    }
}

/// Fast-forward the checked-out branch to its upstream. Returns `Updated` if a
/// fast-forward was applied, `Unchanged` if already current.
fn pull(dest: &Path) -> Result<SyncOutcome> {
    let repo = git2::Repository::open(dest)?;
    let mut remote = repo.find_remote("origin")?;
    remote.fetch(&["HEAD"], None, None)?;
    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    let commit = repo.reference_to_annotated_commit(&fetch_head)?;
    let (analysis, _) = repo.merge_analysis(&[&commit])?;
    if analysis.is_up_to_date() {
        return Ok(SyncOutcome::Unchanged);
    }
    if analysis.is_fast_forward() {
        let refname = "refs/heads/main";
        if let Ok(mut reference) = repo.find_reference(refname) {
            reference.set_target(commit.id(), "fast-forward")?;
            repo.set_head(refname)?;
            repo.checkout_head(Some(
                git2::build::CheckoutBuilder::default().force(),
            ))?;
            return Ok(SyncOutcome::Updated);
        }
    }
    // Non-fast-forward (diverged) — left as-is; treat as no local change.
    Ok(SyncOutcome::Unchanged)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Init a non-bare git repo at `path` with one committed file, on `main`.
    fn init_remote(path: &Path, file: &str, contents: &str) -> git2::Repository {
        let repo = git2::Repository::init(path).unwrap();
        // Ensure the branch is named "main".
        repo.set_head("refs/heads/main").ok();
        commit_file(&repo, file, contents, "first");
        repo
    }

    fn commit_file(repo: &git2::Repository, file: &str, contents: &str, msg: &str) {
        let workdir = repo.workdir().unwrap();
        std::fs::write(workdir.join(file), contents).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(file)).unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let parents = match repo.head().ok().and_then(|h| h.target()) {
            Some(oid) => vec![repo.find_commit(oid).unwrap()],
            None => vec![],
        };
        let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &parent_refs)
            .unwrap();
    }

    #[test]
    fn clones_when_missing_then_pulls_new_commits() {
        let tmp = tempfile::tempdir().unwrap();
        let remote_path = tmp.path().join("remote");
        let dest = tmp.path().join("clone");
        let remote = init_remote(&remote_path, "hymn.txt", "one");
        let url = remote_path.to_str().unwrap();

        // First sync: directory absent -> clone.
        assert_eq!(sync_default_library(url, &dest).unwrap(), SyncOutcome::Cloned);
        assert!(dest.join(".git").is_dir(), "should have cloned");
        assert_eq!(
            std::fs::read_to_string(dest.join("hymn.txt")).unwrap(),
            "one"
        );

        // Third state: clone exists, remote unchanged -> Unchanged.
        assert_eq!(
            sync_default_library(url, &dest).unwrap(),
            SyncOutcome::Unchanged
        );

        // New commit lands on the remote.
        commit_file(&remote, "hymn.txt", "two", "second");

        // Second sync: clone exists -> fast-forward pull picks up the change.
        assert_eq!(sync_default_library(url, &dest).unwrap(), SyncOutcome::Updated);
        assert_eq!(
            std::fs::read_to_string(dest.join("hymn.txt")).unwrap(),
            "two",
            "existing clone should fast-forward to new commit"
        );
    }
}
