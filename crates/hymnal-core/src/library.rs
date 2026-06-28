use serde::{Deserialize, Serialize};

/// The default hymns repository, baked in but overridable in config.
pub const DEFAULT_REPO_URL: &str =
    "https://github.com/AbelHristodor/sda_manager.git";

/// Subdirectory within the default repo that holds the hymn .pptx folders.
/// The repo also contains application code, so the indexer points here rather
/// than at the clone root (which would double-index the test fixtures).
pub const DEFAULT_REPO_HYMNS_SUBDIR: &str = "assets/920";

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

/// Path for the serialized index cache.
pub fn index_cache_path() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("org", "hymnal", "HymnFinder")
        .map(|d| d.cache_dir().join("index.bin"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_toml_round_trips() {
        let cfg = Config {
            default_repo_url: "https://example.com/hymns.git".into(),
            libraries: vec![Library {
                name: "Imnuri".into(),
                path: "/data/imnuri".into(),
                enabled: true,
                managed_by_git: true,
            }],
            download_dir: None,
            language: None,
        };
        let text = cfg.to_toml().unwrap();
        let back = Config::from_toml(&text).unwrap();
        assert_eq!(back.libraries.len(), 1);
        assert_eq!(back.libraries[0].name, "Imnuri");
        assert!(back.libraries[0].managed_by_git);
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
}
