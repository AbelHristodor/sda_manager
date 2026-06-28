use serde::{Deserialize, Serialize};

/// The default hymns repository, baked in but overridable in config.
pub const DEFAULT_REPO_URL: &str =
    "https://github.com/AbelHristodor/sda_manager.git";

/// Subdirectory within the default repo that holds the hymn .pptx folders.
/// The repo also contains application code, so the indexer points here rather
/// than at the clone root (which would double-index the test fixtures).
pub const DEFAULT_REPO_HYMNS_SUBDIR: &str = "assets/920";

/// Display name of the built-in (git-managed) library.
pub const DEFAULT_LIBRARY_NAME: &str = "Imnuri Creștine";

/// One library = a folder of .pptx files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Library {
    pub name: String,
    pub path: String,
    pub enabled: bool,
    /// True for the default library synced via git.
    pub managed_by_git: bool,
}

/// Persisted application configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub default_repo_url: String,
    pub libraries: Vec<Library>,
    /// User's chosen download folder. `None` => OS Downloads directory.
    #[serde(default)]
    pub download_dir: Option<String>,
    /// Selected UI language code ("en"/"it"/"ro"). `None` => not yet chosen
    /// (detect from OS locale on first run).
    #[serde(default)]
    pub language: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            default_repo_url: DEFAULT_REPO_URL.to_string(),
            libraries: Vec::new(),
            download_dir: None,
            language: None,
        }
    }
}

impl Config {
    pub fn to_toml(&self) -> anyhow::Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }

    pub fn from_toml(text: &str) -> anyhow::Result<Config> {
        Ok(toml::from_str(text)?)
    }

    /// Load config from `path`, or return the default if it doesn't exist.
    pub fn load(path: &std::path::Path) -> anyhow::Result<Config> {
        match std::fs::read_to_string(path) {
            Ok(text) => Config::from_toml(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Ok(Config::default())
            }
            Err(e) => Err(e.into()),
        }
    }

    pub fn save(&self, path: &std::path::Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.to_toml()?)?;
        Ok(())
    }
}

/// Standard config + data directories for this app.
pub fn config_path() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("org", "hymnal", "HymnFinder")
        .map(|d| d.config_dir().join("config.toml"))
}

/// Directory where the default git library is cloned.
pub fn default_library_dir() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("org", "hymnal", "HymnFinder")
        .map(|d| d.data_dir().join("default-library"))
}

/// The built-in git-managed library entry (name + hymns-subdir path), or `None`
/// if the data directory can't be determined. Single source of truth shared by
/// the indexer's registration and the Settings UI so they can't drift.
pub fn default_library() -> Option<Library> {
    default_library_dir().map(|dir| Library {
        name: DEFAULT_LIBRARY_NAME.to_string(),
        path: dir
            .join(DEFAULT_REPO_HYMNS_SUBDIR)
            .to_string_lossy()
            .to_string(),
        enabled: true,
        managed_by_git: true,
    })
}

/// Path for the serialized index cache.
pub fn index_cache_path() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("org", "hymnal", "HymnFinder")
        .map(|d| d.cache_dir().join("index.bin"))
}

/// Directory holding user theme JSON files (one per theme).
pub fn themes_dir() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("org", "hymnal", "HymnFinder")
        .map(|d| d.config_dir().join("themes"))
}

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

/// Whether a library folder is currently reachable on disk. Used for the
/// Settings "unavailable" marker; an unreachable folder is simply skipped at
/// index time (the crawl yields nothing) rather than being an error.
pub fn library_available(path: &str) -> bool {
    std::path::Path::new(path).is_dir()
}

/// Set the `enabled` flag of the library whose `path` matches. Works on any
/// library, including the default — the default may be disabled (just not
/// removed). No-op if no library has that path.
pub fn set_library_enabled(cfg: &mut Config, path: &str, enabled: bool) {
    for lib in cfg.libraries.iter_mut() {
        if lib.path == path {
            lib.enabled = enabled;
        }
    }
}

/// Canonicalize `path` to an absolute, symlink-resolved form for stable
/// comparison; falls back to the input as-is if canonicalization fails (e.g.
/// the folder is on an unmounted drive).
fn canonical_string(path: &std::path::Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_string()
}

/// Add a user folder as a `Library`. The display name is the folder's last
/// path component (falling back to the full path). `managed_by_git = false`,
/// `enabled = true`. The stored path is the canonicalized form. Returns `Err`
/// if the folder is already present (compared canonically) or does not exist.
pub fn add_user_library(cfg: &mut Config, path: &std::path::Path) -> anyhow::Result<()> {
    if !path.is_dir() {
        anyhow::bail!("not a folder: {}", path.display());
    }
    let canon = canonical_string(path);
    let exists = cfg
        .libraries
        .iter()
        .any(|l| canonical_string(std::path::Path::new(&l.path)) == canon);
    if exists {
        anyhow::bail!("folder already added");
    }
    // Derive the display name from the canonical path so a messy input (e.g.
    // ".../Dup/./..") or a Windows `\\?\`-prefixed path doesn't yield a weird
    // name like "." or the verbatim prefix.
    let name = std::path::Path::new(&canon)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| canon.clone());
    cfg.libraries.push(Library {
        name,
        path: canon,
        enabled: true,
        managed_by_git: false,
    });
    Ok(())
}

/// Remove the library whose `path` matches. Refuses to remove a
/// `managed_by_git` entry (the default library is locked against removal).
/// No-op if no matching removable library is found.
pub fn remove_user_library(cfg: &mut Config, path: &str) {
    cfg.libraries
        .retain(|l| l.managed_by_git || l.path != path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_toml_round_trips() {
        let cfg = Config {
            default_repo_url: "https://example.com/hymns.git".into(),
            libraries: vec![
                Library {
                    name: "Imnuri".into(),
                    path: "/data/imnuri".into(),
                    enabled: true,
                    managed_by_git: true,
                },
                Library {
                    name: "MyHymns".into(),
                    path: "/tmp/myhymns".into(),
                    enabled: true,
                    managed_by_git: false,
                },
            ],
            download_dir: None,
            language: None,
        };
        let text = cfg.to_toml().unwrap();
        let back = Config::from_toml(&text).unwrap();
        assert_eq!(back.libraries.len(), 2);
        assert_eq!(back.libraries[0].name, "Imnuri");
        assert!(back.libraries[0].managed_by_git);
        assert!(back
            .libraries
            .iter()
            .any(|l| !l.managed_by_git && l.name == "MyHymns"));
    }

    #[test]
    fn default_config_has_baked_in_repo() {
        let cfg = Config::default();
        assert!(!cfg.default_repo_url.is_empty());
    }

    #[test]
    fn config_persists_download_dir() {
        let cfg = Config {
            default_repo_url: "https://example.com/hymns.git".into(),
            libraries: vec![],
            download_dir: Some("/home/user/Videos".into()),
            language: None,
        };
        let back = Config::from_toml(&cfg.to_toml().unwrap()).unwrap();
        assert_eq!(back.download_dir, Some("/home/user/Videos".into()));
    }

    #[test]
    fn config_persists_language() {
        let cfg = Config {
            default_repo_url: "https://example.com/hymns.git".into(),
            libraries: vec![],
            download_dir: None,
            language: Some("ro".into()),
        };
        let back = Config::from_toml(&cfg.to_toml().unwrap()).unwrap();
        assert_eq!(back.language, Some("ro".into()));
    }

    #[test]
    fn config_download_dir_defaults_to_none() {
        let cfg = Config::default();
        assert_eq!(cfg.download_dir, None);
    }

    #[test]
    fn library_available_true_for_existing_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(library_available(&dir.path().to_string_lossy()));
    }

    #[test]
    fn library_available_false_for_missing_dir() {
        assert!(!library_available("/no/such/path/hopefully/12345"));
    }

    #[test]
    fn set_library_enabled_flips_flag() {
        let mut cfg = Config {
            default_repo_url: "x".into(),
            libraries: vec![Library {
                name: "U".into(), path: "/tmp/u".into(), enabled: true, managed_by_git: false,
            }],
            download_dir: None,
            language: None,
        };
        set_library_enabled(&mut cfg, "/tmp/u", false);
        assert!(!cfg.libraries[0].enabled);
        set_library_enabled(&mut cfg, "/tmp/u", true);
        assert!(cfg.libraries[0].enabled);
    }

    #[test]
    fn set_library_enabled_can_disable_default() {
        let mut cfg = Config {
            default_repo_url: "x".into(),
            libraries: vec![Library {
                name: "Default".into(), path: "/data/default".into(), enabled: true, managed_by_git: true,
            }],
            download_dir: None,
            language: None,
        };
        set_library_enabled(&mut cfg, "/data/default", false);
        assert!(!cfg.libraries[0].enabled);
    }

    #[test]
    fn add_user_library_uses_folder_name_and_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("MyHymns");
        std::fs::create_dir(&sub).unwrap();
        let mut cfg = Config::default();
        add_user_library(&mut cfg, &sub).unwrap();
        assert_eq!(cfg.libraries.len(), 1);
        let lib = &cfg.libraries[0];
        assert_eq!(lib.name, "MyHymns");
        assert!(lib.enabled);
        assert!(!lib.managed_by_git);
    }

    #[test]
    fn add_user_library_rejects_duplicate_path() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("Dup");
        std::fs::create_dir(&sub).unwrap();
        let mut cfg = Config::default();
        add_user_library(&mut cfg, &sub).unwrap();
        let messy = sub.join(".").join("..").join("Dup");
        assert!(add_user_library(&mut cfg, &messy).is_err());
        assert_eq!(cfg.libraries.len(), 1);
    }

    #[test]
    fn add_user_library_rejects_non_directory() {
        let mut cfg = Config::default();
        assert!(add_user_library(&mut cfg, std::path::Path::new("/no/such/dir/xyz")).is_err());
        assert!(cfg.libraries.is_empty());
    }

    #[test]
    fn remove_user_library_removes_user_but_not_default() {
        let mut cfg = Config {
            default_repo_url: "x".into(),
            libraries: vec![
                Library { name: "Default".into(), path: "/data/default".into(), enabled: true, managed_by_git: true },
                Library { name: "Mine".into(), path: "/tmp/mine".into(), enabled: true, managed_by_git: false },
            ],
            download_dir: None,
            language: None,
        };
        remove_user_library(&mut cfg, "/tmp/mine");
        assert_eq!(cfg.libraries.len(), 1);
        remove_user_library(&mut cfg, "/data/default");
        assert_eq!(cfg.libraries.len(), 1);
        assert!(cfg.libraries[0].managed_by_git);
    }
}
