use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, serde::Serialize)]
pub struct SortReport {
    pub files_sorted: usize,
    pub folders_used: Vec<String>,
    pub skipped: Vec<String>,
    pub failed: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CustomGroup {
    pub folder: String,
    pub extensions: Vec<String>,
}

/// Predefined groups for common file types. Custom groups defined by the
/// user take precedence over these.
pub fn default_groups() -> Vec<(&'static str, &'static [&'static str])> {
    vec![
        (
            "Documents",
            &[
                "pdf", "docx", "doc", "txt", "md", "rtf", "odt", "odp", "ods", "csv",
                "xls", "xlsx", "ppt", "pptx", "tex", "epub", "pages", "key", "numbers",
            ],
        ),
        (
            "Images",
            &[
                "jpg", "jpeg", "png", "gif", "bmp", "svg", "webp", "tif", "tiff",
                "ico", "heic", "psd", "raw", "cr2",
            ],
        ),
        (
            "Audio",
            &["mp3", "wav", "flac", "aac", "ogg", "m4a", "wma", "opus", "mid", "midi"],
        ),
        (
            "Video",
            &["mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "m4v", "mpeg", "mpg"],
        ),
        (
            "Archives",
            &["zip", "rar", "7z", "tar", "gz", "bz2", "xz", "zst", "iso"],
        ),
        (
            "Applications",
            &["exe", "msi", "app", "apk", "dmg", "jar", "bat", "cmd"],
        ),
        (
            "Code",
            &[
                "js", "ts", "py", "rs", "c", "cpp", "h", "hpp", "java", "html", "css",
                "json", "xml", "yaml", "yml", "sh", "go", "rb", "php", "sql", "toml",
                "ini", "cfg",
            ],
        ),
        (
            "Fonts",
            &["ttf", "otf", "woff", "woff2", "eot"],
        ),
    ]
}

/// Normalizes a user-typed extension: strips dots/whitespace, lowercases.
fn normalize_ext(raw: &str) -> String {
    raw.trim().trim_start_matches('.').to_lowercase()
}

pub fn sort_files_in_dir(dir: &Path, custom_groups: &[CustomGroup]) -> Result<SortReport, String> {
    if !dir.is_dir() {
        return Err(format!("Not a directory: {}", dir.display()));
    }

    // extension -> folder name. Custom groups override defaults.
    let mut ext_to_folder: HashMap<String, String> = HashMap::new();
    for (folder, exts) in default_groups() {
        for ext in exts {
            ext_to_folder.insert(ext.to_string(), folder.to_string());
        }
    }
    for group in custom_groups {
        let folder = group.folder.trim().to_string();
        if folder.is_empty() {
            continue;
        }
        for ext in &group.extensions {
            let ext = normalize_ext(ext);
            if !ext.is_empty() {
                ext_to_folder.insert(ext, folder.clone());
            }
        }
    }

    // Collect file paths first: we mutate the directory (create folders, move
    // files) while processing, which would make a live read_dir iteration
    // skip entries on some filesystems.
    let entries = fs::read_dir(dir).map_err(|e| format!("Cannot read directory: {e}"))?;
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Cannot read entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_lowercase();
        if name == "desktop.ini" || name == "thumbs.db" {
            continue;
        }
        files.push(path);
    }

    let mut report = SortReport {
        files_sorted: 0,
        folders_used: Vec::new(),
        skipped: Vec::new(),
        failed: Vec::new(),
    };

    for path in files {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .filter(|e| !e.is_empty());

        let folder_name = match &ext {
            Some(e) => ext_to_folder.get(e).cloned().unwrap_or_else(|| e.clone()),
            None => String::from("no_extension"),
        };

        let target_dir = dir.join(&folder_name);
        if !target_dir.exists() && fs::create_dir(&target_dir).is_err() {
            report.failed.push(path.display().to_string());
            continue;
        }

        let dest = target_dir.join(path.file_name().expect("file name"));
        if dest.exists() {
            report.skipped.push(path.display().to_string());
            continue;
        }

        if fs::rename(&path, &dest).is_err() {
            report.failed.push(path.display().to_string());
            continue;
        }

        if !report.folders_used.contains(&folder_name) {
            report.folders_used.push(folder_name);
        }
        report.files_sorted += 1;
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn setup() -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "sorter-test-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn uses_default_groups() {
        let dir = setup();
        fs::write(dir.join("report.pdf"), "a").unwrap();
        fs::write(dir.join("photo.jpg"), "b").unwrap();
        fs::write(dir.join("song.mp3"), "c").unwrap();

        let report = sort_files_in_dir(&dir, &[]).unwrap();

        assert_eq!(report.files_sorted, 3);
        assert_eq!(report.folders_used.len(), 3);
        assert!(dir.join("Documents").join("report.pdf").exists());
        assert!(dir.join("Images").join("photo.jpg").exists());
        assert!(dir.join("Audio").join("song.mp3").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn custom_group_overrides_default() {
        let dir = setup();
        fs::write(dir.join("report.pdf"), "a").unwrap();
        fs::write(dir.join("photo.png"), "b").unwrap();

        let custom = vec![CustomGroup {
            folder: "PDFs".to_string(),
            extensions: vec!["pdf".to_string()],
        }];
        let report = sort_files_in_dir(&dir, &custom).unwrap();

        assert_eq!(report.files_sorted, 2);
        assert!(dir.join("PDFs").join("report.pdf").exists());
        assert!(dir.join("Images").join("photo.png").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn custom_group_extensions_are_normalized() {
        let dir = setup();
        fs::write(dir.join("note.TXT"), "a").unwrap();
        fs::write(dir.join("book.EPUB"), "b").unwrap();

        let custom = vec![CustomGroup {
            folder: "Reading".to_string(),
            extensions: vec![".TXT".to_string(), " epub ".to_string()],
        }];
        let report = sort_files_in_dir(&dir, &custom).unwrap();

        assert_eq!(report.files_sorted, 2);
        assert!(dir.join("Reading").join("note.TXT").exists());
        assert!(dir.join("Reading").join("book.EPUB").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn unknown_extension_falls_back_to_extension_folder() {
        let dir = setup();
        fs::write(dir.join("weird.xyzq"), "a").unwrap();

        let report = sort_files_in_dir(&dir, &[]).unwrap();

        assert_eq!(report.files_sorted, 1);
        assert!(dir.join("xyzq").join("weird.xyzq").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn no_extension_files_go_to_no_extension() {
        let dir = setup();
        fs::write(dir.join("README"), "a").unwrap();

        let report = sort_files_in_dir(&dir, &[]).unwrap();

        assert_eq!(report.files_sorted, 1);
        assert!(dir.join("no_extension").join("README").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn skips_subfolders() {
        let dir = setup();
        let sub = dir.join("already");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("keep.txt"), "keep").unwrap();

        let report = sort_files_in_dir(&dir, &[]).unwrap();

        assert_eq!(report.files_sorted, 0);
        assert!(sub.join("keep.txt").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn skips_when_destination_exists() {
        let dir = setup();
        fs::write(dir.join("a.xyzq"), "new").unwrap();
        let out = dir.join("xyzq");
        fs::create_dir(&out).unwrap();
        fs::write(out.join("a.xyzq"), "old").unwrap();

        let report = sort_files_in_dir(&dir, &[]).unwrap();

        assert_eq!(report.files_sorted, 0);
        assert_eq!(report.skipped.len(), 1);
        assert!(report.folders_used.is_empty());
        assert_eq!(fs::read_to_string(out.join("a.xyzq")).unwrap(), "old");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn ignores_windows_system_files() {
        let dir = setup();
        fs::write(dir.join("desktop.ini"), "a").unwrap();
        fs::write(dir.join("Thumbs.DB"), "b").unwrap();
        fs::write(dir.join("note.txt"), "c").unwrap();

        let report = sort_files_in_dir(&dir, &[]).unwrap();

        assert_eq!(report.files_sorted, 1);
        assert!(report.skipped.is_empty());
        assert!(report.failed.is_empty());
        assert!(dir.join("desktop.ini").exists());
        assert!(dir.join("Thumbs.DB").exists());
        assert!(!dir.join("no_extension").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn failed_moves_do_not_abort_the_sort() {
        let dir = setup();
        fs::write(dir.join("no_extension"), "blocker").unwrap();
        fs::write(dir.join("song.mp3"), "b").unwrap();

        let report = sort_files_in_dir(&dir, &[]).unwrap();

        assert_eq!(report.files_sorted, 1);
        assert_eq!(report.failed.len(), 1);
        assert!(!dir.join("no_extension").is_dir());
        assert!(dir.join("Audio").join("song.mp3").exists());
        assert_eq!(report.folders_used, vec!["Audio".to_string()]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn folders_used_counts_existing_folders() {
        let dir = setup();
        fs::create_dir(dir.join("Images")).unwrap();
        fs::write(dir.join("photo.jpg"), "a").unwrap();

        let report = sort_files_in_dir(&dir, &[]).unwrap();

        assert_eq!(report.files_sorted, 1);
        assert_eq!(report.folders_used, vec!["Images".to_string()]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn resorting_moves_nothing() {
        let dir = setup();
        fs::write(dir.join("photo.jpg"), "a").unwrap();
        sort_files_in_dir(&dir, &[]).unwrap();

        let report = sort_files_in_dir(&dir, &[]).unwrap();

        assert_eq!(report.files_sorted, 0);
        assert!(report.folders_used.is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }
}
