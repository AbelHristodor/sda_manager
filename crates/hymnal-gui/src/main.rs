// On Windows, don't allocate a console window for this GUI app. Only affects
// release builds (debug keeps the console so `RUST_LOG` output is visible);
// no-op on macOS/Linux.
#![windows_subsystem = "windows"]

slint::include_modules!();

mod projector;
mod fonts;

use hymnal_core::downloader::{self, DownloadEvent};
use hymnal_core::i18n::{Language, Strings};
use hymnal_core::library::{downloads_dir, Config};
use hymnal_core::model::HymnEntry;
use hymnal_core::search::Searcher;
use log::{debug, info, warn};
use slint::{ModelRc, SharedString, StandardListViewItem, VecModel};
use std::rc::Rc;
use std::sync::mpsc;

/// Fill three `{}` placeholders in `fmt`, in order, with `a`, `b`, `c`.
fn fill3(fmt: &str, a: &str, b: &str, c: &str) -> String {
    fmt.replacen("{}", a, 1)
        .replacen("{}", b, 1)
        .replacen("{}", c, 1)
}

/// Fill the Slint `I18n` global from `Strings::for_language(lang)`. Returns the
/// built `Strings` so callers can reuse it for dynamic status messages.
fn apply_language(ui: &AppWindow, lang: Language) -> Strings {
    let s = Strings::for_language(lang);
    let g = ui.global::<I18n>();
    g.set_app_title(s.app_title.clone().into());
    g.set_nav_library(s.nav_library.clone().into());
    g.set_nav_downloader(s.nav_downloader.clone().into());
    g.set_nav_settings(s.nav_settings.clone().into());
    g.set_search_placeholder(s.search_placeholder.clone().into());
    g.set_prev_slide(s.prev_slide.clone().into());
    g.set_next_slide(s.next_slide.clone().into());
    g.set_open_in_powerpoint(s.open_in_powerpoint.clone().into());
    g.set_reveal_in_folder(s.reveal_in_folder.clone().into());
    g.set_downloader_heading(s.downloader_heading.clone().into());
    g.set_downloader_subtitle(s.downloader_subtitle.clone().into());
    g.set_downloader_url_placeholder(s.downloader_url_placeholder.clone().into());
    g.set_choose(s.choose.clone().into());
    g.set_setting_up_downloader(s.setting_up_downloader.clone().into());
    g.set_download_complete(s.download_complete.clone().into());
    g.set_download_failed_prefix(s.download_failed_prefix.clone().into());
    g.set_settings_heading(s.settings_heading.clone().into());
    g.set_version_prefix(s.version_prefix.clone().into());
    g.set_library_heading(s.library_heading.clone().into());
    g.set_force_sync_description(s.force_sync_description.clone().into());
    g.set_force_sync_button(s.force_sync_button.clone().into());
    g.set_syncing_button(s.syncing_button.clone().into());
    g.set_language_heading(s.language_heading.clone().into());
    g.set_user_libraries_heading(s.user_libraries_heading.clone().into());
    g.set_add_folder_button(s.add_folder_button.clone().into());
    g.set_user_libraries_description(s.user_libraries_description.clone().into());
    g.set_library_unavailable_suffix(s.library_unavailable_suffix.clone().into());
    g.set_nav_themes(s.nav_themes.clone().into());
    g.set_themes_heading(s.themes_heading.clone().into());
    g.set_theme_new(s.theme_new.clone().into());
    g.set_theme_save(s.theme_save.clone().into());
    g.set_theme_delete(s.theme_delete.clone().into());
    g.set_theme_preview_sample(s.theme_preview_sample.clone().into());
    g.set_theme_font_label(s.theme_font_label.clone().into());
    g.set_theme_size_label(s.theme_size_label.clone().into());
    g.set_theme_weight_label(s.theme_weight_label.clone().into());
    g.set_theme_align_label(s.theme_align_label.clone().into());
    g.set_theme_text_color_label(s.theme_text_color_label.clone().into());
    g.set_theme_bg_color_label(s.theme_bg_color_label.clone().into());
    g.set_theme_set_active(s.theme_set_active.clone().into());
    g.set_theme_name_label(s.theme_name_label.clone().into());
    g.set_nav_control(s.nav_control.clone().into());
    g.set_control_heading(s.control_heading.clone().into());
    g.set_control_start(s.control_start.clone().into());
    g.set_control_stop(s.control_stop.clone().into());
    g.set_control_blank(s.control_blank.clone().into());
    g.set_control_prev(s.control_prev.clone().into());
    g.set_control_next(s.control_next.clone().into());
    g.set_control_next_label(s.control_next_label.clone().into());
    g.set_control_live(s.control_live.clone().into());
    g.set_control_output(s.control_output.clone().into());
    g.set_control_theme(s.control_theme.clone().into());
    g.set_library_project(s.library_project.clone().into());
    s
}

/// Format one hymn as a single finder row: "150  Cerul, pământul  · Imnuri".
fn row_label(entry: &HymnEntry) -> String {
    let number = entry
        .number
        .as_deref()
        .map(|n| format!("{n}  "))
        .unwrap_or_default();
    format!("{number}{}  · {}", entry.title, entry.library)
}

use hymnal_core::theme::store;
use hymnal_core::theme::{Background, HAlign, Theme};

/// Convert a core Rgba to a Slint Color.
fn to_color(c: hymnal_core::theme::Rgba) -> slint::Color {
    slint::Color::from_argb_u8(c.a, c.r, c.g, c.b)
}

/// Weight dropdown options shown in the UI, paired with their numeric weight.
const WEIGHT_OPTIONS: &[(&str, u16)] = &[("Light", 300), ("Regular", 400), ("Medium", 500), ("Bold", 700), ("Black", 900)];

fn weight_index(w: u16) -> i32 {
    WEIGHT_OPTIONS.iter().position(|(_, v)| *v == w).unwrap_or(3) as i32
}

/// Push the theme list into the thumbnail-grid models (parallel arrays).
fn refresh_theme_list(ui: &AppWindow, themes: &Rc<std::cell::RefCell<Vec<Theme>>>, active_name: &str) {
    let dir = hymnal_core::library::themes_dir();
    let list = dir.as_deref().map(hymnal_core::theme::store::list_themes)
        .unwrap_or_else(|| vec![Theme::default()]);
    let names: Vec<slint::SharedString> = list.iter().map(|t| t.name.clone().into()).collect();
    let bgs: Vec<slint::Color> = list.iter().map(|t| match &t.background.kind {
        Background::Solid { color } => to_color(*color),
        Background::Gradient { from, .. } => to_color(*from),
        Background::Image { .. } => to_color(t.text.color),
    }).collect();
    let fgs: Vec<slint::Color> = list.iter().map(|t| to_color(t.text.color)).collect();
    let fonts: Vec<slint::SharedString> = list.iter().map(|t| t.text.font_family.clone().into()).collect();
    let active_idx = list.iter().position(|t| t.name == active_name).unwrap_or(0) as i32;
    ui.set_theme_names(slint::ModelRc::from(Rc::new(slint::VecModel::from(names))));
    ui.set_theme_bgs(slint::ModelRc::from(Rc::new(slint::VecModel::from(bgs))));
    ui.set_theme_fgs(slint::ModelRc::from(Rc::new(slint::VecModel::from(fgs))));
    ui.set_theme_fonts(slint::ModelRc::from(Rc::new(slint::VecModel::from(fonts))));
    ui.set_active_index(active_idx);
    *themes.borrow_mut() = list;
}

/// Load a theme's values into the editor's edit-* properties.
fn load_theme_into_editor(ui: &AppWindow, t: &Theme, families: &[String]) {
    let fidx = families.iter().position(|f| f.eq_ignore_ascii_case(&t.text.font_family)).unwrap_or(0) as i32;
    ui.set_edit_font_index(fidx);
    ui.set_edit_weight_index(weight_index(t.text.font_weight));
    ui.set_edit_font_size(t.text.font_size_pt.unwrap_or(44.0));
    ui.set_edit_text_color(to_color(t.text.color));
    ui.set_edit_text_hex(t.text.color.to_hex().into());
    let (bg_color, bg_rgba) = match &t.background.kind {
        Background::Solid { color } => (to_color(*color), *color),
        Background::Gradient { from, .. } => (to_color(*from), *from),
        Background::Image { .. } => (to_color(t.text.color), t.text.color),
    };
    ui.set_edit_bg_color(bg_color);
    ui.set_edit_bg_hex(bg_rgba.to_hex().into());
    ui.set_edit_h_align(match t.text.h_align {
        HAlign::Left => "left", HAlign::Center => "center", HAlign::Right => "right",
    }.into());
    ui.set_edit_name(t.name.clone().into());
}

/// Push a theme + slide text onto a ProjectorWindow's flattened properties.
fn apply_theme_to_projector(p: &ProjectorWindow, theme: &Theme, slide: &str, blank: bool) {
    p.set_slide_text(slide.into());
    p.set_blank(blank);
    p.set_text_color(to_color(theme.text.color));
    p.set_font_family(theme.text.font_family.clone().into());
    p.set_font_size(theme.text.font_size_pt.unwrap_or(44.0));
    p.set_font_weight(theme.text.font_weight as i32);
    p.set_margin(theme.layout.margin);
    p.set_h_align(match theme.text.h_align {
        HAlign::Left => "left",
        HAlign::Center => "center",
        HAlign::Right => "right",
    }.into());
    let bg = match &theme.background.kind {
        Background::Solid { color } => to_color(*color),
        Background::Gradient { from, .. } => to_color(*from),
        Background::Image { .. } => to_color(theme.text.color), // placeholder; image bg is a later task
    };
    p.set_bg_color(bg);
}

/// Mirror the presentation state into the Control tab's text properties.
fn refresh_control_view(ui: &AppWindow, p: &hymnal_core::present::PresentationState) {
    let number = p
        .number
        .as_deref()
        .map(|n| format!("{n}. "))
        .unwrap_or_default();
    ui.set_ctl_title(format!("{number}{}", p.title).into());
    ui.set_ctl_live(p.current_slide().unwrap_or("").into());
    ui.set_ctl_next(p.next_slide().unwrap_or("").into());
    ui.set_ctl_pos(
        if p.slide_count() > 0 {
            format!("{}/{}", p.index + 1, p.slide_count())
        } else {
            String::new()
        }
        .into(),
    );
    ui.set_ctl_blank(p.blank);
}

/// Push the current slide + theme onto the live projector window, if open.
fn push_to_projector(
    projector: &Rc<std::cell::RefCell<Option<ProjectorWindow>>>,
    theme: &Theme,
    p: &hymnal_core::present::PresentationState,
) {
    if let Some(win) = projector.borrow().as_ref() {
        apply_theme_to_projector(win, theme, p.current_slide().unwrap_or(""), p.blank);
    }
}

/// Build the Slint `LibraryRow` model from the config, ensuring the default
/// git-managed library appears even if it isn't yet written to the config on
/// disk (mirrors refresh::register_default_library's "add if no managed entry").
fn library_rows(cfg: &Config) -> Vec<LibraryRow> {
    use hymnal_core::library::library_available;
    let row_of = |l: &hymnal_core::library::Library| LibraryRow {
        name: l.name.clone().into(),
        path: l.path.clone().into(),
        enabled: l.enabled,
        removable: !l.managed_by_git,
        available: library_available(&l.path),
    };
    let mut rows: Vec<LibraryRow> = cfg.libraries.iter().map(row_of).collect();
    // Show the built-in library even before it's persisted to config, using the
    // same source of truth the indexer registers, so the row can't drift.
    if !cfg.libraries.iter().any(|l| l.managed_by_git) {
        if let Some(default) = hymnal_core::library::default_library() {
            rows.insert(0, row_of(&default));
        }
    }
    rows
}

/// Update the preview to show slide `idx` of `slides` for a hymn titled
/// `title` (numbered `number`). Clamps `idx` into range; sets slide text,
/// count, index, and the bottom status bar string.
fn show_slide(
    ui: &AppWindow,
    strings: &Strings,
    number: Option<&str>,
    title: &str,
    slides: &[String],
    idx: i32,
) {
    let count = slides.len() as i32;
    if count == 0 {
        ui.set_slide_text("".into());
        ui.set_slide_count(0);
        ui.set_slide_index(0);
        ui.set_preview_status(SharedString::from(
            strings.slide_zero_fmt.replace("{}", title),
        ));
        return;
    }
    let idx = idx.clamp(0, count - 1);
    let number_prefix = number.map(|n| format!("{n}. ")).unwrap_or_default();
    let titled = format!("{number_prefix}{title}");
    ui.set_slide_text(SharedString::from(slides[idx as usize].clone()));
    ui.set_slide_count(count);
    ui.set_slide_index(idx);
    ui.set_preview_status(SharedString::from(fill3(
        &strings.slide_counter_fmt,
        &titled,
        &(idx + 1).to_string(),
        &count.to_string(),
    )));
}

fn main() -> anyhow::Result<()> {
    // Logging: default to `info` for our crate; override with RUST_LOG=debug.
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("hymnal_gui=info,hymnal_core=info"),
    )
    .init();
    info!("starting Hymn Finder");

    let ui = AppWindow::new()?;

    // Shared config so folder choices persist across the session and to disk.
    let cfg_path = hymnal_core::library::config_path();
    let dl_cfg = std::rc::Rc::new(std::cell::RefCell::new(
        cfg_path
            .as_ref()
            .map(|p| Config::load(p).unwrap_or_default())
            .unwrap_or_default(),
    ));
    let initial_dir = downloads_dir(&dl_cfg.borrow());
    ui.set_download_dir(initial_dir.to_string_lossy().to_string().into());

    // Resolve language: saved choice, else OS locale, else English.
    let boot_lang = dl_cfg
        .borrow()
        .language
        .as_deref()
        .map(Language::from_code)
        .unwrap_or_else(|| {
            sys_locale::get_locale()
                .map(|l| Language::from_locale(&l))
                .unwrap_or(Language::En)
        });
    let strings = Rc::new(std::cell::RefCell::new(apply_language(&ui, boot_lang)));
    ui.set_active_language(boot_lang.code().into());

    ui.set_app_version(env!("CARGO_PKG_VERSION").into());
    ui.set_update_status(strings.borrow().update_checking.clone().into());
    ui.set_sync_status("".into());

    // Populate the Settings tab's library list from the (possibly default) config.
    ui.set_libraries(ModelRc::from(Rc::new(VecModel::from(library_rows(
        &dl_cfg.borrow(),
    )))));
    ui.set_library_status("".into());

    // ---- Themes editor state + initial population ----
    let themes: Rc<std::cell::RefCell<Vec<Theme>>> = Rc::new(std::cell::RefCell::new(Vec::new()));

    // ---- Control tab state (projection) ----
    use hymnal_core::present::PresentationState;
    let present = Rc::new(std::cell::RefCell::new(PresentationState::default()));
    let projector: Rc<std::cell::RefCell<Option<ProjectorWindow>>> =
        Rc::new(std::cell::RefCell::new(None));
    let displays = Rc::new(projector::list_displays());
    let active_theme = Rc::new(std::cell::RefCell::new(Theme::default()));

    // Default output display; a persisted choice (restored below) overrides it.
    ui.set_display_index(projector::default_display_index(&displays));

    // Restore active theme + output display from persisted config (before the
    // theme list is built so the active thumbnail is highlighted correctly).
    if let Some(p) = hymnal_core::library::config_path() {
        let cfg = Config::load(&p).unwrap_or_default();
        if let (Some(name), Some(dir)) =
            (cfg.active_theme.clone(), hymnal_core::library::themes_dir())
        {
            if let Ok(t) = store::load_theme(&dir, &name) {
                *active_theme.borrow_mut() = t;
            }
        }
        if let Some(d) = cfg.output_display {
            ui.set_display_index(d);
        }
    }

    // Font families + weight options for the editor dropdowns.
    let families: Rc<Vec<String>> = Rc::new(fonts::families());
    {
        let rows: Vec<slint::SharedString> = families.iter().map(|f| f.clone().into()).collect();
        ui.set_font_families(slint::ModelRc::from(Rc::new(slint::VecModel::from(rows))));
        let weights: Vec<slint::SharedString> = WEIGHT_OPTIONS.iter().map(|(l, _)| (*l).into()).collect();
        ui.set_weight_options(slint::ModelRc::from(Rc::new(slint::VecModel::from(weights))));
    }
    let draft = Rc::new(std::cell::RefCell::new(Theme::default()));
    refresh_theme_list(&ui, &themes, &active_theme.borrow().name);
    load_theme_into_editor(&ui, &draft.borrow(), &families);
    ui.set_selected_index(0);

    // Populate display picker (plain-string ComboBox model).
    {
        let rows: Vec<slint::SharedString> = displays
            .iter()
            .map(|d| slint::SharedString::from(d.label.clone()))
            .collect();
        ui.set_display_names(slint::ModelRc::from(Rc::new(slint::VecModel::from(rows))));
    }

    // Channel carrying download events from the worker thread to the UI thread.
    let (dl_tx, dl_rx) = mpsc::channel::<DownloadEvent>();

    // Guards against overlapping downloads: set true when a worker is spawned,
    // cleared when a terminal Done/Failed event is drained on the UI thread.
    let dl_in_flight = std::rc::Rc::new(std::cell::Cell::new(false));

    let (tx, rx) = mpsc::channel::<Vec<HymnEntry>>();

    // Force-sync delivers a freshly rebuilt index (or an error message).
    let (fs_tx, fs_rx) = mpsc::channel::<Result<Vec<HymnEntry>, String>>();

    // Local re-index results after add/remove/toggle of a library folder.
    let (lib_tx, lib_rx) = mpsc::channel::<Result<Vec<HymnEntry>, String>>();

    // ---- Worker thread: index-first (no network), then sync in background ----
    // Snapshot translated status strings to move into the worker (it can't touch
    // the UI-thread `strings` Rc).
    let s_ready = strings.borrow().status_library_ready.clone();
    let s_uptodate = strings.borrow().update_up_to_date.clone();
    let s_updatefail = strings.borrow().update_failed.clone();
    let s_staged = strings.borrow().update_staged_fmt.clone();
    let weak = ui.as_weak();
    std::thread::spawn(move || {
        let cfg = match hymnal_core::library::config_path() {
            Some(p) => Config::load(&p).unwrap_or_else(|e| {
                warn!("config load failed ({e}); using defaults");
                Config::default()
            }),
            None => Config::default(),
        };

        // 1) Make search ready ASAP from local clone + cache — no network.
        //    First run (never cloned) has nothing local, so fall back to the
        //    full clone-then-index path.
        if hymnal_core::refresh::default_library_present() {
            let local = hymnal_core::refresh::load_local(cfg.clone());
            let _ = tx.send(local);
            let s_ready1 = s_ready.clone();
            let _ = weak.upgrade_in_event_loop(move |ui| ui.set_status(s_ready1.into()));

            // 2) Pull updates in the background; only re-index if it changed.
            use hymnal_core::sync::SyncOutcome;
            if matches!(
                hymnal_core::refresh::sync_default(&cfg),
                Some(SyncOutcome::Updated | SyncOutcome::Cloned)
            ) {
                info!("library updated by background pull; re-indexing");
                let refreshed = hymnal_core::refresh::load_local(cfg);
                let _ = tx.send(refreshed);
            }
        } else {
            // First run: must clone before there's anything to show.
            let entries = hymnal_core::refresh::load_library(cfg, false);
            let _ = tx.send(entries);
            let s_ready2 = s_ready.clone();
            let _ = weak.upgrade_in_event_loop(move |ui| ui.set_status(s_ready2.into()));
        }

        // Background binary self-update check (errors logged, never block boot).
        match hymnal_core::update::check_and_stage_update() {
            Ok(hymnal_core::update::UpdateOutcome::UpToDate) => {
                let _ = weak.upgrade_in_event_loop(move |ui| ui.set_update_status(s_uptodate.into()));
            }
            Ok(hymnal_core::update::UpdateOutcome::Updated { version }) => {
                let msg = s_staged.replace("{}", &version);
                let _ = weak.upgrade_in_event_loop(move |ui| {
                    ui.set_update_status(msg.into());
                });
            }
            Err(e) => {
                warn!("update check failed: {e}");
                let _ = weak.upgrade_in_event_loop(move |ui| ui.set_update_status(s_updatefail.into()));
            }
        }
    });

    // ---- UI-thread state ----
    let searcher: Rc<std::cell::RefCell<Option<Searcher>>> =
        Rc::new(std::cell::RefCell::new(None));
    // Maps a visible result row -> the entry's index within the searcher, so we
    // never clone hymn bodies per keystroke; entries are looked up on demand.
    let row_to_entry: Rc<std::cell::RefCell<Vec<usize>>> =
        Rc::new(std::cell::RefCell::new(Vec::new()));

    ui.set_status(strings.borrow().status_loading.clone().into());

    // Poll the channel; install the searcher and run an initial search.
    let weak2 = ui.as_weak();
    let searcher_for_timer = searcher.clone();
    let dl_in_flight_timer = dl_in_flight.clone();
    let strings_timer = strings.clone();
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(200),
        move || {
            if let Ok(entries) = rx.try_recv() {
                info!("searcher ready with {} hymns", entries.len());
                *searcher_for_timer.borrow_mut() = Some(Searcher::new(entries));
                if let Some(ui) = weak2.upgrade() {
                    ui.set_status(strings_timer.borrow().status_library_ready.clone().into());
                    ui.invoke_query_changed("".into());
                }
            }
            while let Ok(ev) = dl_rx.try_recv() {
                if let Some(ui) = weak2.upgrade() {
                    let mut s = ui.get_download();
                    match ev {
                        DownloadEvent::Resolving => {
                            s.status = "resolving".into();
                        }
                        DownloadEvent::Title(t) => {
                            s.title = t.into();
                        }
                        DownloadEvent::Progress(p) => {
                            s.status = "downloading".into();
                            s.percent = p.percent;
                            s.speed = p.speed.into();
                            s.eta = p.eta.into();
                        }
                        DownloadEvent::Done { .. } => {
                            s.status = "done".into();
                            s.percent = 100.0;
                            dl_in_flight_timer.set(false);
                        }
                        DownloadEvent::Failed { message } => {
                            s.status = "failed".into();
                            s.message = message.into();
                            dl_in_flight_timer.set(false);
                        }
                    }
                    ui.set_download(s);
                }
            }
            if let Ok(result) = fs_rx.try_recv() {
                if let Some(ui) = weak2.upgrade() {
                    match result {
                        Ok(entries) => {
                            let n = entries.len();
                            *searcher_for_timer.borrow_mut() = Some(Searcher::new(entries));
                            ui.set_sync_status(
                                strings_timer
                                    .borrow()
                                    .status_synced_fmt
                                    .replace("{}", &n.to_string())
                                    .into(),
                            );
                            ui.invoke_query_changed("".into());
                        }
                        Err(e) => {
                            ui.set_sync_status(
                                strings_timer
                                    .borrow()
                                    .status_sync_failed_fmt
                                    .replace("{}", &e)
                                    .into(),
                            );
                        }
                    }
                    ui.set_syncing(false);
                }
            }
            if let Ok(result) = lib_rx.try_recv() {
                if let Some(ui) = weak2.upgrade() {
                    match result {
                        Ok(entries) => {
                            let n = entries.len();
                            *searcher_for_timer.borrow_mut() = Some(Searcher::new(entries));
                            ui.set_library_status(
                                strings_timer
                                    .borrow()
                                    .status_indexed_fmt
                                    .replacen("{}", &n.to_string(), 1)
                                    .into(),
                            );
                            ui.invoke_query_changed("".into());
                        }
                        Err(e) => {
                            ui.set_library_status(
                                strings_timer
                                    .borrow()
                                    .status_indexing_failed_fmt
                                    .replacen("{}", &e, 1)
                                    .into(),
                            );
                        }
                    }
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
        // stay in the searcher (no body cloning). Show every hit — StandardListView
        // virtualizes rendering, so the full library scrolls without a row cap.
        let mut rows: Vec<StandardListViewItem> = Vec::with_capacity(hits.len());
        let mut map: Vec<usize> = Vec::with_capacity(hits.len());
        for h in &hits {
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

    // ---- Highlight changed (keyboard arrows or click) -> show slide 0 ----
    let searcher_for_sel = searcher.clone();
    let rows_for_sel = row_to_entry.clone();
    let strings_sel = strings.clone();
    let weak6 = ui.as_weak();
    ui.on_current_changed(move |idx| {
        let Some(ui) = weak6.upgrade() else { return };
        if idx < 0 {
            ui.set_slide_text("".into());
            ui.set_slide_count(0);
            ui.set_preview_status("".into());
            return;
        }
        let guard = searcher_for_sel.borrow();
        let Some(s) = guard.as_ref() else { return };
        let entry = rows_for_sel
            .borrow()
            .get(idx as usize)
            .and_then(|&ei| s.entry(ei));
        if let Some(entry) = entry {
            debug!("preview #{:?} {} ({} slides)", entry.number, entry.title, entry.slides.len());
            show_slide(&ui, &strings_sel.borrow(), entry.number.as_deref(), &entry.title, &entry.slides, 0);
        }
    });

    // ---- Slide navigation: step within the highlighted hymn's slides ----
    let searcher_for_prev = searcher.clone();
    let rows_for_prev = row_to_entry.clone();
    let strings_prev = strings.clone();
    let weak_prev = ui.as_weak();
    ui.on_prev_slide(move || {
        let Some(ui) = weak_prev.upgrade() else { return };
        let row = ui.get_current_index();
        if row < 0 {
            return;
        }
        let guard = searcher_for_prev.borrow();
        let Some(s) = guard.as_ref() else { return };
        let entry = rows_for_prev.borrow().get(row as usize).and_then(|&ei| s.entry(ei));
        if let Some(entry) = entry {
            show_slide(&ui, &strings_prev.borrow(), entry.number.as_deref(), &entry.title, &entry.slides, ui.get_slide_index() - 1);
        }
    });

    let searcher_for_next = searcher.clone();
    let rows_for_next = row_to_entry.clone();
    let strings_next = strings.clone();
    let weak_next = ui.as_weak();
    ui.on_next_slide(move || {
        let Some(ui) = weak_next.upgrade() else { return };
        let row = ui.get_current_index();
        if row < 0 {
            return;
        }
        let guard = searcher_for_next.borrow();
        let Some(s) = guard.as_ref() else { return };
        let entry = rows_for_next.borrow().get(row as usize).and_then(|&ei| s.entry(ei));
        if let Some(entry) = entry {
            show_slide(&ui, &strings_next.borrow(), entry.number.as_deref(), &entry.title, &entry.slides, ui.get_slide_index() + 1);
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

    // ---- Force sync: wipe clone+cache, re-clone, reindex (off the UI thread) ----
    let weak_fs = ui.as_weak();
    let strings_fs = strings.clone();
    ui.on_force_sync(move || {
        let Some(ui) = weak_fs.upgrade() else { return };
        if ui.get_syncing() {
            return; // already running
        }
        ui.set_syncing(true);
        ui.set_sync_status(strings_fs.borrow().status_re_cloning.clone().into());
        let fs_tx = fs_tx.clone();
        std::thread::spawn(move || {
            let cfg = match hymnal_core::library::config_path() {
                Some(p) => Config::load(&p).unwrap_or_default(),
                None => Config::default(),
            };
            let result = match hymnal_core::refresh::force_clean(&cfg) {
                Ok(()) => Ok(hymnal_core::refresh::load_library(cfg, true)),
                Err(e) => Err(format!("force clean failed: {e}")),
            };
            let _ = fs_tx.send(result);
        });
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

    // ---- Switch UI language: re-fill I18n, persist the choice ----
    let weak_lang = ui.as_weak();
    let strings_lang = strings.clone();
    let cfg_lang = dl_cfg.clone();
    let cfg_path_lang = cfg_path.clone();
    ui.on_set_language(move |code| {
        let Some(ui) = weak_lang.upgrade() else { return };
        let lang = Language::from_code(&code);
        *strings_lang.borrow_mut() = apply_language(&ui, lang);
        ui.set_active_language(lang.code().into());
        // Persist; log on failure (matches force-sync/choose-folder handling).
        cfg_lang.borrow_mut().language = Some(lang.code().to_string());
        if let Some(p) = cfg_path_lang.as_ref() {
            if let Err(e) = cfg_lang.borrow().save(p) {
                warn!("failed to save language: {e}");
            }
        }
    });

    // ---- Choose download folder ----
    let weak_choose = ui.as_weak();
    let cfg_choose = dl_cfg.clone();
    let cfg_path_choose = cfg_path.clone();
    ui.on_choose_folder(move || {
        let Some(ui) = weak_choose.upgrade() else { return };
        let start = ui.get_download_dir().to_string();
        if let Some(folder) = rfd::FileDialog::new()
            .set_directory(if start.is_empty() { ".".into() } else { start })
            .pick_folder()
        {
            let s = folder.to_string_lossy().to_string();
            ui.set_download_dir(s.clone().into());
            cfg_choose.borrow_mut().download_dir = Some(s);
            if let Some(p) = &cfg_path_choose {
                if let Err(e) = cfg_choose.borrow().save(p) {
                    warn!("failed to save config: {e}");
                }
            }
        }
    });

    // ---- Start a download on the worker thread ----
    let weak_start = ui.as_weak();
    let dl_tx_start = dl_tx.clone();
    let dl_in_flight_start = dl_in_flight.clone();
    ui.on_start_download(move || {
        let Some(ui) = weak_start.upgrade() else { return };
        let url = ui.get_download_url().to_string();
        if url.trim().is_empty() {
            return;
        }
        if dl_in_flight_start.get() {
            return;
        }
        dl_in_flight_start.set(true);
        let dir = std::path::PathBuf::from(ui.get_download_dir().to_string());
        info!("starting download: {url} -> {}", dir.display());
        // Show "resolving" immediately for responsiveness.
        ui.set_download(DownloadState {
            status: "resolving".into(),
            title: "".into(),
            message: "".into(),
            speed: "".into(),
            eta: "".into(),
            percent: 0.0,
        });
        let tx = dl_tx_start.clone();
        std::thread::spawn(move || {
            downloader::download(&url, &dir, &tx);
        });
    });

    // ---- Reveal the download folder ----
    let weak_reveal = ui.as_weak();
    ui.on_reveal_download(move || {
        let Some(ui) = weak_reveal.upgrade() else { return };
        let dir = ui.get_download_dir().to_string();
        if !dir.is_empty() {
            if let Err(e) = open::that(&dir) {
                warn!("failed to reveal {dir}: {e}");
            }
        }
    });

    // ---- Library folder management (Settings tab) ----
    let reindex = {
        let lib_tx = lib_tx.clone();
        move |cfg: Config| {
            let lib_tx = lib_tx.clone();
            std::thread::spawn(move || {
                let entries = hymnal_core::refresh::load_local(cfg);
                let _ = lib_tx.send(Ok(entries));
            });
        }
    };

    let weak_addlib = ui.as_weak();
    let cfg_addlib = dl_cfg.clone();
    let cfg_path_addlib = cfg_path.clone();
    let reindex_add = reindex.clone();
    let strings_addlib = strings.clone();
    ui.on_add_library(move || {
        let Some(ui) = weak_addlib.upgrade() else {
            return;
        };
        let Some(folder) = rfd::FileDialog::new().pick_folder() else {
            return;
        };
        let mut cfg = cfg_addlib.borrow_mut();
        match hymnal_core::library::add_user_library(&mut cfg, &folder) {
            Ok(()) => {
                if let Some(p) = &cfg_path_addlib {
                    if let Err(e) = cfg.save(p) {
                        warn!("config save failed: {e}");
                    }
                }
                ui.set_libraries(ModelRc::from(Rc::new(VecModel::from(library_rows(&cfg)))));
                ui.set_library_status(strings_addlib.borrow().status_indexing.clone().into());
                reindex_add(cfg.clone());
            }
            Err(e) => {
                ui.set_library_status(format!("{e}").into());
            }
        }
    });

    let weak_rmlib = ui.as_weak();
    let cfg_rmlib = dl_cfg.clone();
    let cfg_path_rmlib = cfg_path.clone();
    let reindex_rm = reindex.clone();
    let strings_rmlib = strings.clone();
    ui.on_remove_library(move |path| {
        let Some(ui) = weak_rmlib.upgrade() else {
            return;
        };
        let mut cfg = cfg_rmlib.borrow_mut();
        hymnal_core::library::remove_user_library(&mut cfg, &path);
        if let Some(p) = &cfg_path_rmlib {
            if let Err(e) = cfg.save(p) {
                warn!("config save failed: {e}");
            }
        }
        ui.set_libraries(ModelRc::from(Rc::new(VecModel::from(library_rows(&cfg)))));
        ui.set_library_status(strings_rmlib.borrow().status_indexing.clone().into());
        reindex_rm(cfg.clone());
    });

    let weak_togglib = ui.as_weak();
    let cfg_togglib = dl_cfg.clone();
    let cfg_path_togglib = cfg_path.clone();
    let reindex_tog = reindex.clone();
    let strings_togglib = strings.clone();
    ui.on_set_library_enabled(move |path, enabled| {
        let Some(ui) = weak_togglib.upgrade() else {
            return;
        };
        let mut cfg = cfg_togglib.borrow_mut();
        hymnal_core::library::set_library_enabled(&mut cfg, &path, enabled);
        if let Some(p) = &cfg_path_togglib {
            if let Err(e) = cfg.save(p) {
                warn!("config save failed: {e}");
            }
        }
        ui.set_libraries(ModelRc::from(Rc::new(VecModel::from(library_rows(&cfg)))));
        ui.set_library_status(strings_togglib.borrow().status_indexing.clone().into());
        reindex_tog(cfg.clone());
    });

    // ---- Themes editor handlers ----
    // Select a thumbnail -> load into the editor draft.
    {
        let themes = themes.clone(); let draft = draft.clone();
        let families = families.clone(); let weak = ui.as_weak();
        ui.on_theme_selected(move |i| {
            let Some(ui) = weak.upgrade() else { return };
            if let Some(t) = themes.borrow().get(i.max(0) as usize) {
                *draft.borrow_mut() = t.clone();
                load_theme_into_editor(&ui, t, &families);
                ui.set_selected_index(i);
            }
        });
    }
    {
        let draft = draft.clone(); let families = families.clone(); let weak = ui.as_weak();
        ui.on_font_picked(move |i| {
            let Some(ui) = weak.upgrade() else { return };
            if let Some(f) = families.get(i.max(0) as usize) {
                draft.borrow_mut().text.font_family = f.clone();
                ui.set_edit_font_index(i);
            }
        });
    }
    {
        let draft = draft.clone(); let weak = ui.as_weak();
        ui.on_weight_picked(move |i| {
            let Some(ui) = weak.upgrade() else { return };
            let w = WEIGHT_OPTIONS.get(i.max(0) as usize).map(|(_, v)| *v).unwrap_or(400);
            draft.borrow_mut().text.font_weight = w;
            ui.set_edit_weight_index(i);
        });
    }
    {
        let draft = draft.clone(); let weak = ui.as_weak();
        ui.on_size_changed(move |v| {
            let Some(ui) = weak.upgrade() else { return };
            draft.borrow_mut().text.font_size_pt = Some(v);
            ui.set_edit_font_size(v);
        });
    }
    {
        let draft = draft.clone(); let weak = ui.as_weak();
        ui.on_align_picked(move |a| {
            let Some(ui) = weak.upgrade() else { return };
            draft.borrow_mut().text.h_align = match a.as_str() {
                "left" => HAlign::Left, "right" => HAlign::Right, _ => HAlign::Center,
            };
            ui.set_edit_h_align(a);
        });
    }
    {
        let draft = draft.clone(); let weak = ui.as_weak();
        ui.on_text_hex_edited(move |t| {
            let Some(ui) = weak.upgrade() else { return };
            if let Some(rgba) = hymnal_core::theme::Rgba::from_hex(&t) {
                draft.borrow_mut().text.color = rgba;
                ui.set_edit_text_color(to_color(rgba));
                ui.set_edit_text_hex(rgba.to_hex().into());
            }
        });
    }
    {
        let draft = draft.clone(); let weak = ui.as_weak();
        ui.on_text_preset(move |c| {
            let Some(ui) = weak.upgrade() else { return };
            let rgba = hymnal_core::theme::Rgba::new(c.red(), c.green(), c.blue(), 255);
            draft.borrow_mut().text.color = rgba;
            ui.set_edit_text_color(c);
            ui.set_edit_text_hex(rgba.to_hex().into());
        });
    }
    {
        let draft = draft.clone(); let weak = ui.as_weak();
        ui.on_bg_hex_edited(move |t| {
            let Some(ui) = weak.upgrade() else { return };
            if let Some(rgba) = hymnal_core::theme::Rgba::from_hex(&t) {
                draft.borrow_mut().background.kind = Background::Solid { color: rgba };
                ui.set_edit_bg_color(to_color(rgba));
                ui.set_edit_bg_hex(rgba.to_hex().into());
            }
        });
    }
    {
        let draft = draft.clone(); let weak = ui.as_weak();
        ui.on_bg_preset(move |c| {
            let Some(ui) = weak.upgrade() else { return };
            let rgba = hymnal_core::theme::Rgba::new(c.red(), c.green(), c.blue(), 255);
            draft.borrow_mut().background.kind = Background::Solid { color: rgba };
            ui.set_edit_bg_color(c);
            ui.set_edit_bg_hex(rgba.to_hex().into());
        });
    }
    {
        let draft = draft.clone(); let families = families.clone(); let weak = ui.as_weak();
        ui.on_new_theme(move || {
            let Some(ui) = weak.upgrade() else { return };
            let t = Theme { name: "Custom".into(), ..Theme::default() };
            *draft.borrow_mut() = t.clone();
            load_theme_into_editor(&ui, &t, &families);
        });
    }
    {
        let themes = themes.clone(); let draft = draft.clone();
        let active_theme = active_theme.clone(); let weak = ui.as_weak();
        ui.on_save_theme(move || {
            let Some(ui) = weak.upgrade() else { return };
            let mut t = draft.borrow().clone();
            t.name = ui.get_edit_name().to_string();
            if t.name.is_empty() || t.name == "Default" { t.name = "Custom".into(); }
            if let Some(dir) = hymnal_core::library::themes_dir() {
                match hymnal_core::theme::store::save_theme(&dir, &t) {
                    Ok(()) => info!("saved theme {}", t.name),
                    Err(e) => warn!("save theme failed: {e}"),
                }
            }
            *draft.borrow_mut() = t.clone();
            refresh_theme_list(&ui, &themes, &active_theme.borrow().name);
        });
    }
    {
        let themes = themes.clone(); let active_theme = active_theme.clone();
        let projector = projector.clone(); let present = present.clone(); let weak = ui.as_weak();
        ui.on_delete_theme(move || {
            let Some(ui) = weak.upgrade() else { return };
            let idx = ui.get_selected_index().max(0) as usize;
            let name = themes.borrow().get(idx).map(|t| t.name.clone());
            if let Some(name) = name {
                if let Some(dir) = hymnal_core::library::themes_dir() {
                    if let Err(e) = hymnal_core::theme::store::delete_theme(&dir, &name) {
                        warn!("delete theme failed: {e}");
                    }
                }
                if active_theme.borrow().name == name {
                    *active_theme.borrow_mut() = Theme::default();
                    if let Some(p) = hymnal_core::library::config_path() {
                        let mut cfg = Config::load(&p).unwrap_or_default();
                        cfg.active_theme = Some("Default".into());
                        let _ = cfg.save(&p);
                    }
                    push_to_projector(&projector, &active_theme.borrow(), &present.borrow());
                }
            }
            refresh_theme_list(&ui, &themes, &active_theme.borrow().name);
        });
    }
    {
        let themes = themes.clone(); let active_theme = active_theme.clone();
        let projector = projector.clone(); let present = present.clone(); let weak = ui.as_weak();
        ui.on_set_active(move || {
            let Some(ui) = weak.upgrade() else { return };
            let idx = ui.get_selected_index().max(0) as usize;
            if let Some(t) = themes.borrow().get(idx).cloned() {
                *active_theme.borrow_mut() = t.clone();
                if let Some(p) = hymnal_core::library::config_path() {
                    let mut cfg = Config::load(&p).unwrap_or_default();
                    cfg.active_theme = Some(t.name.clone());
                    let _ = cfg.save(&p);
                }
                push_to_projector(&projector, &active_theme.borrow(), &present.borrow());
                refresh_theme_list(&ui, &themes, &t.name);
            }
        });
    }

    // ---- Control tab handlers ----
    // Start projecting.
    {
        let projector = projector.clone();
        let displays = displays.clone();
        let present = present.clone();
        let active_theme = active_theme.clone();
        let weak = ui.as_weak();
        ui.on_ctl_start(move || {
            let Some(ui) = weak.upgrade() else { return };
            if projector.borrow().is_some() {
                return;
            }
            let target = ui.get_display_index();
            if let Some(win) = projector::open_projector(&displays, target) {
                *projector.borrow_mut() = Some(win);
                ui.set_ctl_projecting(true);
                push_to_projector(&projector, &active_theme.borrow(), &present.borrow());
            }
        });
    }
    // Stop.
    {
        let projector = projector.clone();
        let weak = ui.as_weak();
        ui.on_ctl_stop(move || {
            let Some(ui) = weak.upgrade() else { return };
            if let Some(win) = projector.borrow_mut().take() {
                use slint::ComponentHandle;
                let _ = win.hide();
            }
            ui.set_ctl_projecting(false);
        });
    }
    // Next.
    {
        let present = present.clone();
        let projector = projector.clone();
        let active_theme = active_theme.clone();
        let weak = ui.as_weak();
        ui.on_ctl_go_next(move || {
            let Some(ui) = weak.upgrade() else { return };
            present.borrow_mut().next();
            refresh_control_view(&ui, &present.borrow());
            push_to_projector(&projector, &active_theme.borrow(), &present.borrow());
        });
    }
    // Prev.
    {
        let present = present.clone();
        let projector = projector.clone();
        let active_theme = active_theme.clone();
        let weak = ui.as_weak();
        ui.on_ctl_prev(move || {
            let Some(ui) = weak.upgrade() else { return };
            present.borrow_mut().prev();
            refresh_control_view(&ui, &present.borrow());
            push_to_projector(&projector, &active_theme.borrow(), &present.borrow());
        });
    }
    // Blank toggle.
    {
        let present = present.clone();
        let projector = projector.clone();
        let active_theme = active_theme.clone();
        let weak = ui.as_weak();
        ui.on_ctl_blank_toggle(move || {
            let Some(ui) = weak.upgrade() else { return };
            present.borrow_mut().toggle_blank();
            refresh_control_view(&ui, &present.borrow());
            push_to_projector(&projector, &active_theme.borrow(), &present.borrow());
        });
    }
    // Display picked: persist.
    {
        ui.on_ctl_display_picked(move |i| {
            if let Some(p) = hymnal_core::library::config_path() {
                let mut cfg = Config::load(&p).unwrap_or_default();
                cfg.output_display = Some(i);
                let _ = cfg.save(&p);
            }
        });
    }
    // Control search (reuses Searcher); maps row -> entry index.
    let ctl_rows: Rc<std::cell::RefCell<Vec<usize>>> = Rc::new(std::cell::RefCell::new(Vec::new()));
    {
        let searcher = searcher.clone();
        let ctl_rows = ctl_rows.clone();
        let weak = ui.as_weak();
        ui.on_ctl_search_changed(move |q| {
            let Some(ui) = weak.upgrade() else { return };
            let guard = searcher.borrow();
            let Some(s) = guard.as_ref() else { return };
            let hits = s.search(&q);
            let mut rows = Vec::new();
            let mut map = Vec::new();
            for h in &hits {
                rows.push(slint::StandardListViewItem::from(slint::SharedString::from(
                    row_label(h.entry),
                )));
                map.push(h.index);
            }
            *ctl_rows.borrow_mut() = map;
            ui.set_ctl_search_results(slint::ModelRc::from(Rc::new(slint::VecModel::from(rows))));
        });
    }
    // Load highlighted hymn into the presentation.
    {
        let searcher = searcher.clone();
        let ctl_rows = ctl_rows.clone();
        let present = present.clone();
        let projector = projector.clone();
        let active_theme = active_theme.clone();
        let weak = ui.as_weak();
        ui.on_ctl_search_activated(move |i| {
            let Some(ui) = weak.upgrade() else { return };
            if i < 0 {
                return;
            }
            let guard = searcher.borrow();
            let Some(s) = guard.as_ref() else { return };
            let entry = ctl_rows
                .borrow()
                .get(i as usize)
                .and_then(|&ei| s.entry(ei))
                .cloned();
            if let Some(e) = entry {
                present
                    .borrow_mut()
                    .load_hymn(e.number.clone(), e.title.clone(), e.slides.clone());
                refresh_control_view(&ui, &present.borrow());
                push_to_projector(&projector, &active_theme.borrow(), &present.borrow());
            }
        });
    }
    // Library "Project" button: load highlighted Library hymn, switch to Control (tab 3).
    {
        let searcher = searcher.clone();
        let row_to_entry = row_to_entry.clone();
        let present = present.clone();
        let projector = projector.clone();
        let active_theme = active_theme.clone();
        let weak = ui.as_weak();
        ui.on_project_current(move || {
            let Some(ui) = weak.upgrade() else { return };
            let idx = ui.get_current_index();
            if idx < 0 {
                return;
            }
            let guard = searcher.borrow();
            let Some(s) = guard.as_ref() else { return };
            let entry = row_to_entry
                .borrow()
                .get(idx as usize)
                .and_then(|&ei| s.entry(ei))
                .cloned();
            if let Some(e) = entry {
                present
                    .borrow_mut()
                    .load_hymn(e.number.clone(), e.title.clone(), e.slides.clone());
                refresh_control_view(&ui, &present.borrow());
                push_to_projector(&projector, &active_theme.borrow(), &present.borrow());
                ui.set_active_tab(3);
            }
        });
    }

    // Keep the timer alive for the lifetime of the application; dropping it
    // would stop the channel polling.
    let _timer = timer;

    ui.run()?;
    Ok(())
}
