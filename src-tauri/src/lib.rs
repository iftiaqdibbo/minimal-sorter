mod sorter;

use std::path::{Path, PathBuf};

use sorter::CustomGroup;
use tauri::Manager;

#[tauri::command]
fn pick_folder() -> Option<String> {
    rfd::FileDialog::new()
        .pick_folder()
        .map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
fn sort_files(path: String, custom_groups: Vec<CustomGroup>) -> Result<sorter::SortReport, String> {
    sorter::sort_files_in_dir(Path::new(&path), &custom_groups)
}

fn config_file(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Cannot resolve config directory: {e}"))?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Cannot create config directory: {e}"))?;
    Ok(dir.join("groups.json"))
}

#[tauri::command]
fn load_groups(app: tauri::AppHandle) -> Vec<CustomGroup> {
    let Ok(file) = config_file(&app) else {
        return Vec::new();
    };
    if !file.exists() {
        return Vec::new();
    }
    std::fs::read_to_string(file)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

#[tauri::command]
fn save_groups(app: tauri::AppHandle, groups: Vec<CustomGroup>) -> Result<(), String> {
    let file = config_file(&app)?;
    let json = serde_json::to_string_pretty(&groups).map_err(|e| e.to_string())?;
    std::fs::write(file, json).map_err(|e| format!("Cannot save groups: {e}"))
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            pick_folder,
            sort_files,
            load_groups,
            save_groups
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
