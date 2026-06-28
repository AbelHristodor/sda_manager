//! Reusable library-loading pipeline shared by app boot and "force sync":
//! ensure the default library is present (clone/pull), index all enabled
//! libraries, and maintain the on-disk cache.

use crate::index::{load_cache, refresh_index, save_cache};
use crate::library::{
    default_library_dir, index_cache_path, Config, Library, DEFAULT_REPO_HYMNS_SUBDIR,
};
use crate::model::HymnEntry;
use crate::sync::{sync_default_library, SyncOutcome};
use log::{debug, info, warn};
use std::path::Path;

/// Delete the default git-managed library clone and the index cache, given
/// explicit paths. Missing paths are not an error. Split out from `force_clean`
/// so it is unit-testable without touching real app directories.
pub fn force_clean_paths(
    clone_dir: Option<&Path>,
    cache_file: Option<&Path>,
) -> anyhow::Result<()> {
    if let Some(dir) = clone_dir {
        if dir.exists() {
            info!("force_clean: removing clone dir {}", dir.display());
            std::fs::remove_dir_all(dir)?;
        }
    }
    if let Some(file) = cache_file {
        if file.exists() {
            info!("force_clean: removing cache file {}", file.display());
            std::fs::remove_file(file)?;
        }
    }
    Ok(())
}

/// Force-clean using the standard app directories.
pub fn force_clean(_cfg: &Config) -> anyhow::Result<()> {
    force_clean_paths(default_library_dir().as_deref(), index_cache_path().as_deref())
}

/// Register the default git-managed library in `cfg` (path only; does NOT touch
/// the network). Adds the entry if no git-managed library is present yet.
/// Returns false if the default library dir can't be determined.
fn register_default_library(cfg: &mut Config) -> bool {
    let Some(dir) = default_library_dir() else {
        warn!("could not determine default library dir");
        return false;
    };
    if !cfg.libraries.iter().any(|l| l.managed_by_git) {
        let hymns = dir.join(DEFAULT_REPO_HYMNS_SUBDIR);
        debug!("registering default library at {}", hymns.display());
        cfg.libraries.push(Library {
            name: "Imnuri Creștine".into(),
            path: hymns.to_string_lossy().to_string(),
            enabled: true,
            managed_by_git: true,
        });
    }
    true
}

/// Clone the default library if it's missing, otherwise fast-forward pull.
/// Returns the sync outcome (Cloned/Updated/Unchanged), or None on error/no dir.
/// This is the network step — call it off the UI thread.
pub fn sync_default(cfg: &Config) -> Option<SyncOutcome> {
    let dir = default_library_dir()?;
    let fresh = !dir.join(".git").is_dir();
    info!(
        "{} default library from {} -> {}",
        if fresh { "cloning" } else { "updating" },
        cfg.default_repo_url,
        dir.display()
    );
    match sync_default_library(&cfg.default_repo_url, &dir) {
        Ok(outcome) => {
            info!("sync ok: {outcome:?}");
            Some(outcome)
        }
        Err(e) => {
            warn!("clone/sync failed: {e}");
            None
        }
    }
}

/// Index from whatever is already on disk — NO network. Use on boot for an
/// instant time-to-searchable; a background [`sync_default`] + re-index can
/// follow if it reports changes. If the default library has never been cloned
/// (first run), this returns whatever other libraries yield (often empty),
/// and the caller should fall back to [`load_library`].
pub fn load_local(mut cfg: Config) -> Vec<HymnEntry> {
    register_default_library(&mut cfg);
    index_enabled(&cfg, false)
}

/// True if the default git-managed library has been cloned at least once.
pub fn default_library_present() -> bool {
    default_library_dir()
        .map(|d| d.join(".git").is_dir())
        .unwrap_or(false)
}

/// Load (and cache) the hymn index for all enabled libraries in `cfg`. When
/// `force` is true the on-disk cache is ignored, forcing a full re-parse.
/// This performs the network sync first (clone/pull) — use for first run and
/// force-sync, not the fast boot path.
pub fn load_library(mut cfg: Config, force: bool) -> Vec<HymnEntry> {
    let _ = sync_default(&cfg);
    register_default_library(&mut cfg);
    index_enabled(&cfg, force)
}

/// Index every enabled library in `cfg`, using the on-disk cache unless `force`.
fn index_enabled(cfg: &Config, force: bool) -> Vec<HymnEntry> {
    let cache = index_cache_path();
    let cached = if force {
        Vec::new()
    } else {
        cache.as_ref().and_then(|p| load_cache(p)).unwrap_or_default()
    };
    debug!("loaded {} cached entries (force={force})", cached.len());

    let mut entries = Vec::new();
    for lib in cfg.libraries.iter().filter(|l| l.enabled) {
        let root = Path::new(&lib.path);
        let before = entries.len();
        entries.extend(refresh_index(root, &lib.name, &cached));
        info!(
            "indexed library '{}' at {} -> {} hymns",
            lib.name,
            lib.path,
            entries.len() - before
        );
    }
    if entries.is_empty() && !force {
        warn!("no entries indexed; falling back to {} cached", cached.len());
        entries = cached;
    }
    info!("total {} hymns indexed", entries.len());

    if let Some(p) = cache {
        match save_cache(&p, &entries) {
            Ok(()) => debug!("wrote index cache to {}", p.display()),
            Err(e) => warn!("failed to write index cache: {e}"),
        }
    }
    entries
}
