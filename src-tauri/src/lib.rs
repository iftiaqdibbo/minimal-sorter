mod sorter;

use std::path::{Path, PathBuf};

use sorter::{CustomGroup, RecordedMove, UndoReport};
use tauri::Manager;

#[tauri::command]
fn pick_folder() -> Option<String> {
    rfd::FileDialog::new()
        .pick_folder()
        .map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
fn sort_files(
    app: tauri::AppHandle,
    path: String,
    custom_groups: Vec<CustomGroup>,
    excluded_extensions: Vec<String>,
    remove_empty_folders: bool,
) -> Result<sorter::SortReport, String> {
    let (report, moves) = sorter::sort_files_with_moves(
        Path::new(&path),
        &custom_groups,
        &excluded_extensions,
        remove_empty_folders,
    )?;
    save_last_sort(&app, &moves);
    Ok(report)
}

#[tauri::command]
fn preview_sort(
    path: String,
    custom_groups: Vec<CustomGroup>,
    excluded_extensions: Vec<String>,
) -> Result<sorter::PreviewReport, String> {
    sorter::preview_sort_files(Path::new(&path), &custom_groups, &excluded_extensions)
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

#[tauri::command]
fn is_directory(path: String) -> bool {
    Path::new(&path).is_dir()
}

fn config_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Cannot resolve config directory: {e}"))?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Cannot create config directory: {e}"))?;
    Ok(dir)
}

fn config_file(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(config_dir(app)?.join("groups.json"))
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
    Ok(config_dir(app)?.join("last-path.json"))
}

fn last_sort_file(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(config_dir(app)?.join("last-sort.json"))
}

fn excluded_file(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(config_dir(app)?.join("excluded.json"))
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

fn save_last_sort(app: &tauri::AppHandle, moves: &[RecordedMove]) {
    if moves.is_empty() {
        return;
    }
    if let Ok(file) = last_sort_file(app) {
        if let Ok(json) = serde_json::to_string_pretty(moves) {
            let _ = std::fs::write(file, json);
        }
    }
}

#[tauri::command]
fn load_excluded(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    let file = excluded_file(&app)?;
    if !file.exists() {
        return Ok(Vec::new());
    }
    let content =
        std::fs::read_to_string(&file).map_err(|e| format!("Cannot read excluded file: {e}"))?;
    serde_json::from_str::<Vec<String>>(&content)
        .map_err(|e| format!("Saved excluded extensions could not be read ({e})."))
}

#[tauri::command]
fn save_excluded(app: tauri::AppHandle, extensions: Vec<String>) -> Result<(), String> {
    let file = excluded_file(&app)?;
    let json = serde_json::to_string_pretty(&extensions).map_err(|e| e.to_string())?;
    std::fs::write(file, json).map_err(|e| format!("Cannot save excluded extensions: {e}"))
}

#[tauri::command]
fn undo_last_sort(app: tauri::AppHandle) -> Result<UndoReport, String> {
    let file = last_sort_file(&app)?;
    if !file.exists() {
        return Ok(UndoReport {
            undone: 0,
            failed: Vec::new(),
        });
    }
    let content =
        std::fs::read_to_string(&file).map_err(|e| format!("Cannot read undo file: {e}"))?;
    let moves: Vec<RecordedMove> = match serde_json::from_str(&content) {
        Ok(m) => m,
        Err(e) => {
            let _ = std::fs::remove_file(&file);
            return Err(format!("Undo data could not be read ({e})."));
        }
    };
    let (undone, failed) = sorter::undo_moves(&moves);
    if failed.is_empty() {
        let _ = std::fs::remove_file(&file);
    } else if let Ok(json) = serde_json::to_string_pretty(&failed) {
        let _ = std::fs::write(&file, json);
    }
    Ok(UndoReport {
        undone,
        failed: failed.iter().map(|m| m.to.clone()).collect(),
    })
}

#[tauri::command]
fn has_undo(app: tauri::AppHandle) -> bool {
    last_sort_file(&app).map(|file| file.exists()).unwrap_or(false)
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            pick_folder,
            sort_files,
            preview_sort,
            open_folder,
            is_directory,
            load_groups,
            save_groups,
            load_last_path,
            save_last_path,
            load_excluded,
            save_excluded,
            undo_last_sort,
            has_undo
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
