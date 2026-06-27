slint::include_modules!();

use hymnal_core::index::{load_cache, refresh_index, save_cache};
use hymnal_core::library::{default_library_dir, index_cache_path, Config, Library};
use hymnal_core::model::HymnEntry;
use hymnal_core::search::Searcher;
use hymnal_core::sync::sync_default_library;
use log::{debug, info, warn};
use slint::{ModelRc, SharedString, StandardListViewItem, VecModel};
use std::rc::Rc;
use std::sync::mpsc;

/// Format one hymn as a single finder row: "150  Cerul, pământul  · Imnuri".
fn row_label(entry: &HymnEntry) -> String {
    let number = entry
        .number
        .map(|n| format!("{n}  "))
        .unwrap_or_default();
    format!("{number}{}  · {}", entry.title, entry.library)
}

fn main() -> anyhow::Result<()> {
    // Logging: default to `info` for our crate; override with RUST_LOG=debug.
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("hymnal_gui=info,hymnal_core=info"),
    )
    .init();
    info!("starting Hymn Finder");

    let ui = AppWindow::new()?;
    let (tx, rx) = mpsc::channel::<Vec<HymnEntry>>();

    // ---- Worker thread: config -> sync -> index -> send entries to UI ----
    let weak = ui.as_weak();
    std::thread::spawn(move || {
        let mut cfg = Config::default();
        match hymnal_core::library::config_path() {
            Some(p) => {
                debug!("loading config from {}", p.display());
                cfg = Config::load(&p).unwrap_or_else(|e| {
                    warn!("config load failed ({e}); using defaults");
                    Config::default()
                });
            }
            None => warn!("no config path available; using defaults"),
        }
        info!(
            "config: repo_url={}, {} configured libraries",
            cfg.default_repo_url,
            cfg.libraries.len()
        );

        if let Some(dir) = default_library_dir() {
            // Always sync: clones if missing, fast-forward pulls if present, so
            // an existing checkout picks up newly published hymns on launch.
            let fresh = !dir.join(".git").is_dir();
            info!(
                "{} default library from {} -> {}",
                if fresh { "cloning" } else { "updating" },
                cfg.default_repo_url,
                dir.display()
            );
            match sync_default_library(&cfg.default_repo_url, &dir) {
                Ok(()) => info!("clone/sync ok"),
                Err(e) => warn!("clone/sync failed: {e}"),
            }
            if !cfg.libraries.iter().any(|l| l.managed_by_git) {
                // The default repo holds app code alongside the hymns, so index
                // the hymns subdirectory rather than the clone root.
                let hymns = dir.join(hymnal_core::library::DEFAULT_REPO_HYMNS_SUBDIR);
                debug!("registering default library at {}", hymns.display());
                cfg.libraries.push(Library {
                    name: "Imnuri Creștine".into(),
                    path: hymns.to_string_lossy().to_string(),
                    enabled: true,
                    managed_by_git: true,
                });
            }
        } else {
            warn!("could not determine default library dir");
        }

        let cache = index_cache_path();
        let cached = cache.as_ref().and_then(|p| load_cache(p)).unwrap_or_default();
        debug!("loaded {} cached entries", cached.len());

        let mut entries = Vec::new();
        for lib in cfg.libraries.iter().filter(|l| l.enabled) {
            let root = std::path::Path::new(&lib.path);
            let n_before = entries.len();
            entries.extend(refresh_index(root, &lib.name, &cached));
            info!(
                "indexed library '{}' at {} -> {} hymns",
                lib.name,
                lib.path,
                entries.len() - n_before
            );
        }
        if entries.is_empty() {
            warn!("no entries indexed; falling back to {} cached entries", cached.len());
            entries = cached;
        }
        info!("total {} hymns indexed", entries.len());

        if let Some(p) = cache {
            match save_cache(&p, &entries) {
                Ok(()) => debug!("wrote index cache to {}", p.display()),
                Err(e) => warn!("failed to write index cache: {e}"),
            }
        }
        if tx.send(entries).is_err() {
            warn!("UI gone before index delivered");
        }
        let _ = weak.upgrade_in_event_loop(|ui| {
            ui.set_status("Library ready.".into());
        });
    });

    // ---- UI-thread state ----
    let searcher: Rc<std::cell::RefCell<Option<Searcher>>> =
        Rc::new(std::cell::RefCell::new(None));
    // Maps a visible result row -> the entry's index within the searcher, so we
    // never clone hymn bodies per keystroke; entries are looked up on demand.
    let row_to_entry: Rc<std::cell::RefCell<Vec<usize>>> =
        Rc::new(std::cell::RefCell::new(Vec::new()));

    ui.set_status("Loading hymn library…".into());

    // Poll the channel; install the searcher and run an initial search.
    let weak2 = ui.as_weak();
    let searcher_for_timer = searcher.clone();
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(200),
        move || {
            if let Ok(entries) = rx.try_recv() {
                info!("searcher ready with {} hymns", entries.len());
                *searcher_for_timer.borrow_mut() = Some(Searcher::new(entries));
                if let Some(ui) = weak2.upgrade() {
                    ui.set_status("Library ready.".into());
                    ui.invoke_query_changed("".into());
                }
            }
        },
    );

    // ---- Search-on-edit: query -> hits -> rows, auto-select first row ----
    let searcher_for_query = searcher.clone();
    let rows_for_query = row_to_entry.clone();
    let weak3 = ui.as_weak();
    ui.on_query_changed(move |q| {
        let guard = searcher_for_query.borrow();
        let Some(s) = guard.as_ref() else {
            debug!("query '{q}' ignored: searcher not ready yet");
            return;
        };
        let hits = s.search(&q);
        debug!("query '{q}' -> {} hits", hits.len());

        // Build display rows and the row->entry-index map in one pass; entries
        // stay in the searcher (no body cloning).
        let mut rows: Vec<StandardListViewItem> = Vec::with_capacity(hits.len().min(200));
        let mut map: Vec<usize> = Vec::with_capacity(hits.len().min(200));
        for h in hits.iter().take(200) {
            rows.push(StandardListViewItem::from(SharedString::from(row_label(h.entry))));
            map.push(h.index);
        }
        let row_count = rows.len();
        *rows_for_query.borrow_mut() = map;

        if let Some(ui) = weak3.upgrade() {
            ui.set_results(ModelRc::from(Rc::new(VecModel::from(rows))));
            // fzf-style: highlight the top result so Enter opens it immediately.
            let sel = if row_count > 0 { 0 } else { -1 };
            ui.set_current_index(sel);
            ui.invoke_current_changed(sel);
        }
    });

    // ---- Highlight changed (keyboard arrows or click) -> update preview ----
    let searcher_for_sel = searcher.clone();
    let rows_for_sel = row_to_entry.clone();
    let weak6 = ui.as_weak();
    ui.on_current_changed(move |idx| {
        let Some(ui) = weak6.upgrade() else { return };
        if idx < 0 {
            ui.set_preview_title("".into());
            ui.set_preview_body("".into());
            return;
        }
        let guard = searcher_for_sel.borrow();
        let Some(s) = guard.as_ref() else { return };
        let entry = rows_for_sel
            .borrow()
            .get(idx as usize)
            .and_then(|&ei| s.entry(ei));
        if let Some(entry) = entry {
            debug!("preview #{:?} {}", entry.number, entry.title);
            let title = format!(
                "{}{}",
                entry.number.map(|n| format!("{n}. ")).unwrap_or_default(),
                entry.title
            );
            ui.set_preview_title(SharedString::from(title));
            ui.set_preview_body(SharedString::from(entry.body.clone()));
        }
    });

    // ---- Open the highlighted hymn externally (Enter / button) ----
    let searcher_for_open = searcher.clone();
    let rows_for_open = row_to_entry.clone();
    let weak4 = ui.as_weak();
    ui.on_open_current(move || {
        let Some(ui) = weak4.upgrade() else { return };
        let idx = ui.get_current_index();
        if idx < 0 {
            debug!("open ignored: no row highlighted");
            return;
        }
        let guard = searcher_for_open.borrow();
        let Some(s) = guard.as_ref() else { return };
        let path = rows_for_open
            .borrow()
            .get(idx as usize)
            .and_then(|&ei| s.entry(ei))
            .map(|e| e.path.clone());
        if let Some(path) = path {
            info!("opening {}", path.display());
            if let Err(e) = open::that(&path) {
                warn!("failed to open {}: {e}", path.display());
            }
        }
    });

    // ---- Reveal the highlighted hymn's folder ----
    let searcher_for_reveal = searcher.clone();
    let rows_for_reveal = row_to_entry.clone();
    let weak5 = ui.as_weak();
    ui.on_reveal_current(move || {
        let Some(ui) = weak5.upgrade() else { return };
        let idx = ui.get_current_index();
        if idx < 0 {
            return;
        }
        let guard = searcher_for_reveal.borrow();
        let Some(s) = guard.as_ref() else { return };
        let path = rows_for_reveal
            .borrow()
            .get(idx as usize)
            .and_then(|&ei| s.entry(ei))
            .map(|e| e.path.clone());
        if let Some(parent) = path.as_deref().and_then(|p| p.parent()) {
            info!("revealing {}", parent.display());
            if let Err(e) = open::that(parent) {
                warn!("failed to reveal {}: {e}", parent.display());
            }
        }
    });

    // Keep the timer alive for the lifetime of the application; dropping it
    // would stop the channel polling.
    let _timer = timer;

    ui.run()?;
    Ok(())
}
