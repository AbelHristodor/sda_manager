slint::include_modules!();

use hymnal_core::index::{load_cache, refresh_index, save_cache};
use hymnal_core::library::{default_library_dir, index_cache_path, Config, Library};
use hymnal_core::model::HymnEntry;
use hymnal_core::search::Searcher;
use hymnal_core::sync::sync_default_library;
use slint::{ModelRc, SharedString, VecModel};
use std::rc::Rc;
use std::sync::mpsc;

fn main() -> anyhow::Result<()> {
    let ui = AppWindow::new()?;
    let (tx, rx) = mpsc::channel::<Vec<HymnEntry>>();

    let weak = ui.as_weak();
    std::thread::spawn(move || {
        let mut cfg = Config::default();
        if let Some(p) = hymnal_core::library::config_path() {
            cfg = Config::load(&p).unwrap_or_default();
        }
        if let Some(dir) = default_library_dir() {
            if !dir.join(".git").is_dir() {
                let _ = sync_default_library(&cfg.default_repo_url, &dir);
            }
            if !cfg.libraries.iter().any(|l| l.managed_by_git) {
                cfg.libraries.push(Library {
                    name: "Imnuri Creștine".into(),
                    path: dir.to_string_lossy().to_string(),
                    enabled: true,
                    managed_by_git: true,
                });
            }
        }
        let cache = index_cache_path();
        let cached = cache.as_ref().and_then(|p| load_cache(p)).unwrap_or_default();
        let mut entries = Vec::new();
        for lib in cfg.libraries.iter().filter(|l| l.enabled) {
            let root = std::path::Path::new(&lib.path);
            entries.extend(refresh_index(root, &lib.name, &cached));
        }
        if entries.is_empty() {
            entries = cached;
        }
        if let Some(p) = cache {
            let _ = save_cache(&p, &entries);
        }
        let _ = tx.send(entries);
        let _ = weak.upgrade_in_event_loop(|ui| {
            ui.set_status("Library ready.".into());
        });
    });

    let searcher: Rc<std::cell::RefCell<Option<Searcher>>> =
        Rc::new(std::cell::RefCell::new(None));
    let last_hits: Rc<std::cell::RefCell<Vec<HymnEntry>>> =
        Rc::new(std::cell::RefCell::new(Vec::new()));

    ui.set_status("Loading hymn library…".into());

    let weak2 = ui.as_weak();
    let searcher_for_timer = searcher.clone();
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(200),
        move || {
            if let Ok(entries) = rx.try_recv() {
                *searcher_for_timer.borrow_mut() = Some(Searcher::new(entries));
                if let Some(ui) = weak2.upgrade() {
                    ui.set_status("Library ready.".into());
                    ui.invoke_query_changed("".into());
                }
            }
        },
    );

    let searcher_for_query = searcher.clone();
    let hits_for_query = last_hits.clone();
    let weak3 = ui.as_weak();
    ui.on_query_changed(move |q| {
        let guard = searcher_for_query.borrow();
        let Some(s) = guard.as_ref() else { return };
        let hits = s.search(&q);
        let rows: Vec<HymnRow> = hits
            .iter()
            .take(200)
            .map(|h| HymnRow {
                number: h
                    .entry
                    .number
                    .map(|n| n.to_string())
                    .unwrap_or_default()
                    .into(),
                title: h.entry.title.clone().into(),
                library: h.entry.library.clone().into(),
            })
            .collect();
        *hits_for_query.borrow_mut() =
            hits.into_iter().take(200).map(|h| h.entry).collect();
        if let Some(ui) = weak3.upgrade() {
            ui.set_results(ModelRc::from(Rc::new(VecModel::from(rows))));
            ui.set_selected_index(-1);
            ui.set_preview_title("".into());
            ui.set_preview_body("".into());
        }
    });

    let hits_for_sel = last_hits.clone();
    let weak6 = ui.as_weak();
    ui.on_selection_changed(move |idx| {
        let Some(ui) = weak6.upgrade() else { return };
        if idx < 0 {
            return;
        }
        if let Some(entry) = hits_for_sel.borrow().get(idx as usize) {
            let title = format!(
                "{}{}",
                entry.number.map(|n| format!("{n}. ")).unwrap_or_default(),
                entry.title
            );
            ui.set_preview_title(SharedString::from(title));
            ui.set_preview_body(SharedString::from(entry.body.clone()));
        }
    });

    let hits_for_open = last_hits.clone();
    let weak4 = ui.as_weak();
    ui.on_open_selected(move || {
        let Some(ui) = weak4.upgrade() else { return };
        let idx = ui.get_selected_index();
        if idx < 0 {
            return;
        }
        if let Some(entry) = hits_for_open.borrow().get(idx as usize) {
            let _ = open::that(&entry.path);
        }
    });

    let hits_for_reveal = last_hits.clone();
    let weak5 = ui.as_weak();
    ui.on_reveal_selected(move || {
        let Some(ui) = weak5.upgrade() else { return };
        let idx = ui.get_selected_index();
        if idx < 0 {
            return;
        }
        if let Some(entry) = hits_for_reveal.borrow().get(idx as usize) {
            if let Some(parent) = entry.path.parent() {
                let _ = open::that(parent);
            }
        }
    });

    // Keep the timer alive for the lifetime of the application; dropping it
    // would stop the channel polling.
    let _timer = timer;

    ui.run()?;
    Ok(())
}
