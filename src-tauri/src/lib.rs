pub mod ballistics;
pub mod capture;
pub mod ocr;
pub mod parse;

use anyhow::{anyhow, Result};
use base64::Engine as _;
use ballistics::{Coord, Solution, Weapon};
use capture::CaptureRegion;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum PointKind {
    Artillery,
    Target,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Settings {
    hotkey_artillery: String,
    hotkey_target: String,
    weapon_id: String,
    region: CaptureRegion,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey_artillery: "F1".into(),
            hotkey_target: "F2".into(),
            weapon_id: ballistics::data().default_weapon.clone(),
            region: CaptureRegion::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CaptureReport {
    kind: PointKind,
    ok: bool,
    coord: Option<Coord>,
    text: String,
    image: Option<String>,
}

#[derive(Default)]
struct AppState {
    settings: Settings,
    artillery: Option<Coord>,
    target: Option<Coord>,
    last_capture: Option<CaptureReport>,
    hotkey_error: Option<String>,
    shortcut_ids: Vec<(u32, PointKind)>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    weapons: Vec<Weapon>,
    settings: Settings,
    artillery: Option<Coord>,
    target: Option<Coord>,
    solution: Option<Solution>,
    last_capture: Option<CaptureReport>,
    hotkey_error: Option<String>,
}

type Shared = Mutex<AppState>;

fn snapshot(state: &AppState) -> Snapshot {
    let solution = match (state.artillery, state.target, ballistics::weapon(&state.settings.weapon_id)) {
        (Some(a), Some(t), Some(w)) => Some(ballistics::solve(w, a, t)),
        _ => None,
    };
    Snapshot {
        weapons: ballistics::data().weapons.clone(),
        settings: state.settings.clone(),
        artillery: state.artillery,
        target: state.target,
        solution,
        last_capture: state.last_capture.clone(),
        hotkey_error: state.hotkey_error.clone(),
    }
}

fn broadcast(app: &AppHandle) {
    let snap = snapshot(&app.state::<Shared>().lock().unwrap());
    let _ = app.emit("state", snap);
}

fn settings_path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("settings.json"))
}

fn load_settings(app: &AppHandle) -> Settings {
    settings_path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_settings(app: &AppHandle, settings: &Settings) -> Result<()> {
    let path = settings_path(app).ok_or_else(|| anyhow!("no config dir"))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(settings)?)?;
    Ok(())
}

fn register_hotkeys(app: &AppHandle) -> Result<()> {
    let shared = app.state::<Shared>();
    let mut state = shared.lock().unwrap();
    let gs = app.global_shortcut();
    gs.unregister_all()?;
    state.shortcut_ids.clear();

    let binds = [
        (state.settings.hotkey_artillery.clone(), PointKind::Artillery),
        (state.settings.hotkey_target.clone(), PointKind::Target),
    ];
    for (key, kind) in binds {
        let shortcut: Shortcut = key.parse().map_err(|e| anyhow!("bad hotkey {key:?}: {e}"))?;
        gs.register(shortcut)?;
        state.shortcut_ids.push((shortcut.id(), kind));
    }
    Ok(())
}

fn png_data_url(img: &xcap::image::RgbaImage) -> Option<String> {
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, xcap::image::ImageFormat::Png).ok()?;
    Some(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(buf.into_inner())
    ))
}

const QUORUM: u32 = 2;

/// Stops once QUORUM passes agree on each axis; otherwise takes the most common value.
pub fn recognize_capture(cap: &capture::Capture) -> (Option<Coord>, String) {
    let engine = match ocr::Ocr::new() {
        Ok(e) => e,
        Err(e) => return (None, format!("ocr: {e:#}")),
    };

    let mut texts = Vec::new();
    let mut votes = parse::Votes::default();
    for variant in &cap.variants {
        let lines: Result<Vec<String>> = variant.iter().map(|line| engine.recognize(line)).collect();
        match lines {
            Ok(lines) => {
                let text = lines.join("\n");
                votes.add(&parse::readings(&text));
                texts.push(text);
                if let Some(c) = votes.decided(QUORUM) {
                    return (Some(c), texts.join("\n---\n"));
                }
            }
            Err(e) => texts.push(format!("ocr: {e:#}")),
        }
    }
    (votes.decided(1), texts.join("\n---\n"))
}

fn read_cursor_coord(region: &CaptureRegion) -> (Option<Coord>, String, Option<String>) {
    match capture::capture_at_cursor(region) {
        Ok(cap) => {
            let (coord, text) = recognize_capture(&cap);
            (coord, text, png_data_url(&cap.raw))
        }
        Err(e) => (None, format!("capture: {e:#}"), None),
    }
}

fn on_hotkey(app: AppHandle, kind: PointKind) {
    std::thread::spawn(move || {
        let region = app.state::<Shared>().lock().unwrap().settings.region.clone();

        let (coord, text, image) = read_cursor_coord(&region);

        {
            let shared = app.state::<Shared>();
            let mut s = shared.lock().unwrap();
            if let Some(c) = coord {
                match kind {
                    PointKind::Artillery => s.artillery = Some(c),
                    PointKind::Target => s.target = Some(c),
                }
            }
            s.last_capture = Some(CaptureReport { kind, ok: coord.is_some(), coord, text, image });
        }

        broadcast(&app);
    });
}

#[tauri::command]
fn get_state(state: State<Shared>) -> Snapshot {
    snapshot(&state.lock().unwrap())
}

#[tauri::command]
fn set_point(app: AppHandle, kind: PointKind, coord: Option<Coord>) {
    {
        let shared = app.state::<Shared>();
        let mut s = shared.lock().unwrap();
        match kind {
            PointKind::Artillery => s.artillery = coord,
            PointKind::Target => s.target = coord,
        }
    }
    broadcast(&app);
}

#[tauri::command]
fn swap_points(app: AppHandle) {
    {
        let shared = app.state::<Shared>();
        let mut s = shared.lock().unwrap();
        let s = &mut *s;
        std::mem::swap(&mut s.artillery, &mut s.target);
    }
    broadcast(&app);
}

#[tauri::command]
fn update_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    if ballistics::weapon(&settings.weapon_id).is_none() {
        return Err(format!("unknown weapon {:?}", settings.weapon_id));
    }
    let previous = {
        let shared = app.state::<Shared>();
        let mut s = shared.lock().unwrap();
        std::mem::replace(&mut s.settings, settings.clone())
    };

    if let Err(e) = register_hotkeys(&app) {
        app.state::<Shared>().lock().unwrap().settings = previous;
        let _ = register_hotkeys(&app);
        return Err(format!("{e:#}"));
    }

    app.state::<Shared>().lock().unwrap().hotkey_error = None;
    save_settings(&app, &settings).map_err(|e| format!("{e:#}"))?;
    broadcast(&app);
    Ok(())
}

#[tauri::command]
fn capture_now(app: AppHandle, kind: PointKind) {
    on_hotkey(app, kind);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    let kind = app
                        .state::<Shared>()
                        .lock()
                        .unwrap()
                        .shortcut_ids
                        .iter()
                        .find(|(id, _)| *id == shortcut.id())
                        .map(|(_, k)| *k);
                    if let Some(kind) = kind {
                        on_hotkey(app.clone(), kind);
                    }
                })
                .build(),
        )
        .manage(Shared::default())
        .setup(|app| {
            let handle = app.handle().clone();
            handle.state::<Shared>().lock().unwrap().settings = load_settings(&handle);
            if let Err(e) = register_hotkeys(&handle) {
                handle.state::<Shared>().lock().unwrap().hotkey_error =
                    Some(format!("Hotkeys unavailable ({e:#}). Pick other keys in settings."));
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            set_point,
            swap_points,
            update_settings,
            capture_now
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
