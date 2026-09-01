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

#[tauri::command]
fn preview_sort(path: String, custom_groups: Vec<CustomGroup>) -> Result<sorter::PreviewReport, String> {
    sorter::preview_sort_files(Path::new(&path), &custom_groups)
}

#[tauri::command]
fn open_folder(path: String) -> Result<(), String> {
    if !Path::new(&path).is_dir() {
        return Err(format!("Not a directory: {path}"));
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Cannot open folder: {e}"))?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Cannot open folder: {e}"))?;
    }
    Ok(())
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
fn load_groups(app: tauri::AppHandle) -> Result<Vec<CustomGroup>, String> {
    let file = config_file(&app)?;
    if !file.exists() {
        return Ok(Vec::new());
    }
    let content =
        std::fs::read_to_string(&file).map_err(|e| format!("Cannot read groups file: {e}"))?;
    match serde_json::from_str::<Vec<CustomGroup>>(&content) {
        Ok(groups) => Ok(groups),
        Err(e) => {
            let backup = file.with_extension("json.bak");
            let _ = std::fs::remove_file(&backup);
            if std::fs::rename(&file, &backup).is_ok() {
                Err(format!(
                    "Saved groups could not be read ({e}). The old file was kept as {}.",
                    backup.display()
                ))
            } else {
                Err(format!("Saved groups could not be read ({e})."))
            }
        }
    }
}

#[tauri::command]
fn save_groups(app: tauri::AppHandle, groups: Vec<CustomGroup>) -> Result<(), String> {
    let file = config_file(&app)?;
    let json = serde_json::to_string_pretty(&groups).map_err(|e| e.to_string())?;
    std::fs::write(file, json).map_err(|e| format!("Cannot save groups: {e}"))
}

fn last_path_file(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Cannot resolve config directory: {e}"))?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Cannot create config directory: {e}"))?;
    Ok(dir.join("last-path.json"))
}

#[tauri::command]
fn load_last_path(app: tauri::AppHandle) -> Option<String> {
    let file = last_path_file(&app).unwrap_or_else(|_| PathBuf::new());
    if !file.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&file).unwrap_or_else(|_| String::new());
    let path = content.trim();
    if path.is_empty() {
        None
    } else {
        Some(path.to_owned())
    }
}

#[tauri::command]
fn save_last_path(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let file = last_path_file(&app)?;
    std::fs::write(file, path).map_err(|e| format!("Cannot save last path: {e}"))
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            pick_folder,
            sort_files,
            preview_sort,
            open_folder,
            load_groups,
            save_groups,
            load_last_path,
            save_last_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
