use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, serde::Serialize)]
pub struct SortReport {
    pub files_sorted: usize,
    pub folders_used: Vec<String>,
    pub renamed: Vec<String>,
    pub failed: Vec<String>,
    pub empty_folders_removed: usize,
}

#[derive(Debug, serde::Serialize)]
pub struct PreviewMove {
    pub from: String,
    pub to: String,
}

#[derive(Debug, serde::Serialize)]
pub struct PreviewReport {
    pub moves: Vec<PreviewMove>,
    pub folders_used: Vec<String>,
    pub renamed: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CustomGroup {
    pub folder: String,
    pub extensions: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RecordedMove {
    pub from: String,
    pub to: String,
}

#[derive(Debug, serde::Serialize)]
pub struct UndoReport {
    pub undone: usize,
    pub failed: Vec<String>,
}

struct MoveOp {
    from: PathBuf,
    to: PathBuf,
}

fn file_mtime(path: &Path) -> SystemTime {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .unwrap_or(UNIX_EPOCH)
}

fn stem_and_ext(path: &Path) -> (String, Option<String>) {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let ext = path.extension().map(|e| e.to_string_lossy().into_owned());
    (stem, ext)
}

fn target_folder_for(ext: &Option<String>, ext_to_folder: &HashMap<String, String>) -> String {
    match ext {
        Some(e) => ext_to_folder.get(e).cloned().unwrap_or_else(|| e.clone()),
        None => String::from("no_extension"),
    }
}

/// Next free "stem (k).ext" inside target_dir, skipping names that already
/// exist on disk or that an earlier planned move already claimed.
fn next_free_name(
    target_dir: &Path,
    stem: &str,
    ext: Option<&str>,
    reserved: &HashMap<PathBuf, usize>,
) -> PathBuf {
    let mut k: u64 = 1;
    loop {
        let name = match ext {
            Some(e) => format!("{stem} ({k}).{e}"),
            None => format!("{stem} ({k})"),
        };
        let candidate = target_dir.join(&name);
        if !candidate.exists() && !reserved.contains_key(&candidate) {
            return candidate;
        }
        k += 1;
    }
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
        (
            "eBooks",
            &["mobi", "azw", "azw3", "fb2", "djvu", "lit"],
        ),
        (
            "Subtitles",
            &["srt", "ass", "ssa", "sub", "vtt"],
        ),
        (
            "3D-CAD",
            &["stl", "obj", "fbx", "blend", "3ds", "step", "dxf", "dwg"],
        ),
        (
            "Disk-Images",
            &["img", "vmdk", "vhd", "vhdx", "qcow2"],
        ),
        (
            "Data",
            &["db", "sqlite", "mdb", "parquet", "arrow", "hdf"],
        ),
        (
            "Torrents",
            &["torrent"],
        ),
        (
            "Logs",
            &["log"],
        ),
    ]
}

/// Normalizes a user-typed extension: strips dots/whitespace, lowercases.
fn normalize_ext(raw: &str) -> String {
    raw.trim().trim_start_matches('.').to_lowercase()
}

/// Computes the collision-free move list. Planning only reads metadata; it
/// never mutates the directory, so the plan can be shown as a preview.
fn plan_moves(
    dir: &Path,
    custom_groups: &[CustomGroup],
    excluded_extensions: &[String],
) -> Result<Vec<MoveOp>, String> {
    if !dir.is_dir() {
        return Err(format!("Not a directory: {}", dir.display()));
    }

    let excluded: HashSet<String> = excluded_extensions
        .iter()
        .map(|e| normalize_ext(e))
        .filter(|e| !e.is_empty())
        .collect();

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

    let mut moves: Vec<MoveOp> = Vec::new();
    let mut reserved: HashMap<PathBuf, usize> = HashMap::new();

    for path in files {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .filter(|e| !e.is_empty());
        if ext.as_ref().map_or(false, |e| excluded.contains(e)) {
            continue;
        }
        let folder_name = target_folder_for(&ext, &ext_to_folder);
        let target_dir = dir.join(&folder_name);
        let file_name = path.file_name().expect("file name").to_string_lossy().into_owned();
        let base = target_dir.join(&file_name);
        let (stem, ext_string) = stem_and_ext(&path);

        // Who currently holds the base name: a move already planned in this
        // pass, an existing file, or nobody.
        let holder: Option<(Option<usize>, SystemTime)> = if let Some(&idx) = reserved.get(&base) {
            Some((Some(idx), file_mtime(&moves[idx].from)))
        } else if base.exists() {
            Some((None, file_mtime(&base)))
        } else {
            None
        };

        let dest = match holder {
            None => base,
            Some((planned, holder_mtime)) if file_mtime(&path) > holder_mtime => {
                let holder_new =
                    next_free_name(&target_dir, &stem, ext_string.as_deref(), &reserved);
                match planned {
                    Some(idx) => {
                        moves[idx].to = holder_new.clone();
                        reserved.insert(holder_new, idx);
                    }
                    None => {
                        reserved.insert(holder_new.clone(), moves.len());
                        moves.push(MoveOp { from: base.clone(), to: holder_new });
                    }
                }
                base
            }
            Some(_) => next_free_name(&target_dir, &stem, ext_string.as_deref(), &reserved),
        };

        let idx = moves.len();
        moves.push(MoveOp { from: path, to: dest.clone() });
        reserved.insert(dest, idx);
    }

    Ok(moves)
}

fn apply_moves(moves: &[MoveOp], mut report: SortReport) -> (SortReport, Vec<usize>) {
    let mut applied = Vec::new();
    for (i, m) in moves.iter().enumerate() {
        if let Some(parent) = m.to.parent() {
            if !parent.exists() && fs::create_dir(parent).is_err() {
                report.failed.push(m.from.display().to_string());
                continue;
            }
        }
        if fs::rename(&m.from, &m.to).is_err() {
            report.failed.push(m.from.display().to_string());
            continue;
        }
        let from_name = m
            .from
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let to_name = m
            .to
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if from_name != to_name {
            report.renamed.push(m.from.display().to_string());
        }
        let folder = m
            .to
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !report.folders_used.contains(&folder) {
            report.folders_used.push(folder);
        }
        report.files_sorted += 1;
        applied.push(i);
    }
    (report, applied)
}

pub fn sort_files_with_moves(
    dir: &Path,
    custom_groups: &[CustomGroup],
    excluded_extensions: &[String],
    remove_empty_folders: bool,
) -> Result<(SortReport, Vec<RecordedMove>), String> {
    let moves = plan_moves(dir, custom_groups, excluded_extensions)?;
    let report = SortReport {
        files_sorted: 0,
        folders_used: Vec::new(),
        renamed: Vec::new(),
        failed: Vec::new(),
        empty_folders_removed: 0,
    };
    let (mut report, applied) = apply_moves(&moves, report);
    let recorded = applied
        .iter()
        .map(|&i| RecordedMove {
            from: moves[i].from.display().to_string(),
            to: moves[i].to.display().to_string(),
        })
        .collect();
    if remove_empty_folders {
        report.empty_folders_removed = remove_empty_subfolders(dir).unwrap_or(0);
    }
    Ok((report, recorded))
}

pub fn preview_sort_files(
    dir: &Path,
    custom_groups: &[CustomGroup],
    excluded_extensions: &[String],
) -> Result<PreviewReport, String> {
    let moves = plan_moves(dir, custom_groups, excluded_extensions)?;
    let mut report = PreviewReport {
        moves: Vec::new(),
        folders_used: Vec::new(),
        renamed: Vec::new(),
    };
    for m in &moves {
        report.moves.push(PreviewMove {
            from: m.from.display().to_string(),
            to: m.to.display().to_string(),
        });
        let from_name = m
            .from
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let to_name = m
            .to
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if from_name != to_name {
            report.renamed.push(m.from.display().to_string());
        }
        let folder = m
            .to
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !report.folders_used.contains(&folder) {
            report.folders_used.push(folder);
        }
    }
    Ok(report)
}

pub fn remove_empty_subfolders(dir: &Path) -> Result<usize, String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("Cannot read directory: {e}"))?;
    let mut subdirs = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Cannot read entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            subdirs.push(path);
        }
    }
    let mut removed = 0;
    for sub in subdirs {
        let is_empty = fs::read_dir(&sub).map(|mut d| d.next().is_none()).unwrap_or(false);
        if is_empty && fs::remove_dir(&sub).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

pub fn undo_moves(moves: &[RecordedMove]) -> (usize, Vec<RecordedMove>) {
    let mut undone = 0;
    let mut failed = Vec::new();
    for m in moves.iter().rev() {
        let from = Path::new(&m.from);
        let to = Path::new(&m.to);
        if !to.exists() {
            continue;
        }
        if let Some(parent) = from.parent() {
            if !parent.exists() && fs::create_dir_all(parent).is_err() {
                failed.push(m.clone());
                continue;
            }
        }
        if fs::rename(to, from).is_err() {
            failed.push(m.clone());
            continue;
        }
        undone += 1;
    }
    (undone, failed)
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

    fn sort_report(
        dir: &Path,
        custom: &[CustomGroup],
        excluded: &[String],
        remove_empty: bool,
    ) -> Result<SortReport, String> {
        sort_files_with_moves(dir, custom, excluded, remove_empty).map(|(r, _)| r)
    }

    #[test]
    fn uses_default_groups() {
        let dir = setup();
        fs::write(dir.join("report.pdf"), "a").unwrap();
        fs::write(dir.join("photo.jpg"), "b").unwrap();
        fs::write(dir.join("song.mp3"), "c").unwrap();

        let report = sort_report(&dir, &[], &[], false).unwrap();

        assert_eq!(report.files_sorted, 3);
        assert_eq!(report.folders_used.len(), 3);
        assert!(dir.join("Documents").join("report.pdf").exists());
        assert!(dir.join("Images").join("photo.jpg").exists());
        assert!(dir.join("Audio").join("song.mp3").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn new_default_groups_work() {
        let dir = setup();
        fs::write(dir.join("book.mobi"), "a").unwrap();
        fs::write(dir.join("movie.srt"), "b").unwrap();
        fs::write(dir.join("model.stl"), "c").unwrap();
        fs::write(dir.join("disk.vmdk"), "d").unwrap();
        fs::write(dir.join("data.sqlite"), "e").unwrap();
        fs::write(dir.join("movie.torrent"), "f").unwrap();
        fs::write(dir.join("app.log"), "g").unwrap();

        let report = sort_report(&dir, &[], &[], false).unwrap();

        assert_eq!(report.files_sorted, 7);
        assert!(dir.join("eBooks").join("book.mobi").exists());
        assert!(dir.join("Subtitles").join("movie.srt").exists());
        assert!(dir.join("3D-CAD").join("model.stl").exists());
        assert!(dir.join("Disk-Images").join("disk.vmdk").exists());
        assert!(dir.join("Data").join("data.sqlite").exists());
        assert!(dir.join("Torrents").join("movie.torrent").exists());
        assert!(dir.join("Logs").join("app.log").exists());
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
        let report = sort_report(&dir, &custom, &[], false).unwrap();

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
        let report = sort_report(&dir, &custom, &[], false).unwrap();

        assert_eq!(report.files_sorted, 2);
        assert!(dir.join("Reading").join("note.TXT").exists());
        assert!(dir.join("Reading").join("book.EPUB").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn unknown_extension_falls_back_to_extension_folder() {
        let dir = setup();
        fs::write(dir.join("weird.xyzq"), "a").unwrap();

        let report = sort_report(&dir, &[], &[], false).unwrap();

        assert_eq!(report.files_sorted, 1);
        assert!(dir.join("xyzq").join("weird.xyzq").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn no_extension_files_go_to_no_extension() {
        let dir = setup();
        fs::write(dir.join("README"), "a").unwrap();

        let report = sort_report(&dir, &[], &[], false).unwrap();

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

        let report = sort_report(&dir, &[], &[], false).unwrap();

        assert_eq!(report.files_sorted, 0);
        assert!(sub.join("keep.txt").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn renames_collision_keeping_newest() {
        let dir = setup();
        let out = dir.join("xyzq");
        fs::create_dir(&out).unwrap();
        let existing = out.join("a.xyzq");
        fs::write(&existing, "old").unwrap();
        filetime::set_file_mtime(&existing, filetime::FileTime::from_unix_time(1000, 0)).unwrap();
        fs::write(dir.join("a.xyzq"), "new").unwrap();

        let report = sort_report(&dir, &[], &[], false).unwrap();

        assert_eq!(report.files_sorted, 2);
        assert_eq!(report.renamed.len(), 1);
        assert_eq!(fs::read_to_string(out.join("a.xyzq")).unwrap(), "new");
        assert_eq!(fs::read_to_string(out.join("a (1).xyzq")).unwrap(), "old");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn older_incoming_file_gets_the_suffix() {
        let dir = setup();
        let out = dir.join("xyzq");
        fs::create_dir(&out).unwrap();
        let existing = out.join("a.xyzq");
        fs::write(&existing, "existing").unwrap();
        filetime::set_file_mtime(&existing, filetime::FileTime::from_unix_time(5000, 0)).unwrap();
        let incoming = dir.join("a.xyzq");
        fs::write(&incoming, "incoming").unwrap();
        filetime::set_file_mtime(&incoming, filetime::FileTime::from_unix_time(1000, 0)).unwrap();

        let report = sort_report(&dir, &[], &[], false).unwrap();

        assert_eq!(report.files_sorted, 1);
        assert_eq!(report.renamed.len(), 1);
        assert_eq!(fs::read_to_string(out.join("a.xyzq")).unwrap(), "existing");
        assert_eq!(fs::read_to_string(out.join("a (1).xyzq")).unwrap(), "incoming");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn collision_numbers_increment_past_used_suffixes() {
        let dir = setup();
        let out = dir.join("xyzq");
        fs::create_dir(&out).unwrap();
        fs::write(out.join("a.xyzq"), "old base").unwrap();
        filetime::set_file_mtime(&out.join("a.xyzq"), filetime::FileTime::from_unix_time(1000, 0)).unwrap();
        fs::write(out.join("a (1).xyzq"), "mid").unwrap();
        filetime::set_file_mtime(&out.join("a (1).xyzq"), filetime::FileTime::from_unix_time(2000, 0)).unwrap();
        fs::write(dir.join("a.xyzq"), "new").unwrap();

        let report = sort_report(&dir, &[], &[], false).unwrap();

        assert_eq!(report.files_sorted, 2);
        assert_eq!(report.renamed.len(), 1);
        assert_eq!(fs::read_to_string(out.join("a.xyzq")).unwrap(), "new");
        assert_eq!(fs::read_to_string(out.join("a (1).xyzq")).unwrap(), "mid");
        assert_eq!(fs::read_to_string(out.join("a (2).xyzq")).unwrap(), "old base");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn preview_plans_without_moving_anything() {
        let dir = setup();
        fs::write(dir.join("report.pdf"), "a").unwrap();
        fs::write(dir.join("photo.jpg"), "b").unwrap();

        let report = preview_sort_files(&dir, &[], &[]).unwrap();

        assert_eq!(report.moves.len(), 2);
        assert_eq!(report.folders_used.len(), 2);
        assert!(report.folders_used.contains(&"Documents".to_string()));
        assert!(report.folders_used.contains(&"Images".to_string()));
        assert!(dir.join("report.pdf").exists());
        assert!(dir.join("photo.jpg").exists());
        assert!(!dir.join("Documents").exists());
        assert!(!dir.join("Images").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn ignores_windows_system_files() {
        let dir = setup();
        fs::write(dir.join("desktop.ini"), "a").unwrap();
        fs::write(dir.join("Thumbs.DB"), "b").unwrap();
        fs::write(dir.join("note.txt"), "c").unwrap();

        let report = sort_report(&dir, &[], &[], false).unwrap();

        assert_eq!(report.files_sorted, 1);
        assert!(report.renamed.is_empty());
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

        let report = sort_report(&dir, &[], &[], false).unwrap();

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

        let report = sort_report(&dir, &[], &[], false).unwrap();

        assert_eq!(report.files_sorted, 1);
        assert_eq!(report.folders_used, vec!["Images".to_string()]);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn resorting_moves_nothing() {
        let dir = setup();
        fs::write(dir.join("photo.jpg"), "a").unwrap();
        sort_report(&dir, &[], &[], false).unwrap();

        let report = sort_report(&dir, &[], &[], false).unwrap();

        assert_eq!(report.files_sorted, 0);
        assert!(report.folders_used.is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn excluded_extensions_are_not_moved() {
        let dir = setup();
        fs::write(dir.join("report.pdf"), "a").unwrap();
        fs::write(dir.join("temp.part"), "b").unwrap();

        let excluded = vec!["part".to_string()];
        let report = sort_report(&dir, &[], &excluded, false).unwrap();

        assert_eq!(report.files_sorted, 1);
        assert!(dir.join("Documents").join("report.pdf").exists());
        assert!(dir.join("temp.part").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn preview_respects_excluded_extensions() {
        let dir = setup();
        fs::write(dir.join("report.pdf"), "a").unwrap();
        fs::write(dir.join("temp.part"), "b").unwrap();

        let excluded = vec![".PART".to_string()];
        let report = preview_sort_files(&dir, &[], &excluded).unwrap();

        assert_eq!(report.moves.len(), 1);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn remove_empty_subfolders_removes_only_empty_dirs() {
        let dir = setup();
        let empty = dir.join("Empty");
        let nested = dir.join("HasFiles");
        fs::create_dir(&empty).unwrap();
        fs::create_dir(&nested).unwrap();
        fs::write(nested.join("keep.txt"), "a").unwrap();
        fs::write(dir.join("loose.txt"), "b").unwrap();

        let removed = remove_empty_subfolders(&dir).unwrap();

        assert_eq!(removed, 1);
        assert!(!empty.exists());
        assert!(nested.exists());
        assert!(dir.join("loose.txt").exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn sort_can_remove_empty_subfolders() {
        let dir = setup();
        let empty = dir.join("Empty");
        fs::create_dir(&empty).unwrap();
        fs::write(dir.join("report.pdf"), "a").unwrap();

        let report = sort_report(&dir, &[], &[], true).unwrap();

        assert_eq!(report.files_sorted, 1);
        assert_eq!(report.empty_folders_removed, 1);
        assert!(!empty.exists());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn undo_moves_restores_files_in_reverse() {
        let dir = setup();
        let out = dir.join("xyzq");
        fs::create_dir(&out).unwrap();
        let existing = out.join("a.xyzq");
        fs::write(&existing, "old").unwrap();
        filetime::set_file_mtime(&existing, filetime::FileTime::from_unix_time(1000, 0)).unwrap();
        fs::write(dir.join("a.xyzq"), "new").unwrap();

        let (_report, moves) = sort_files_with_moves(&dir, &[], &[], false).unwrap();
        assert_eq!(moves.len(), 2);

        let (undone, failed) = undo_moves(&moves);

        assert_eq!(undone, 2);
        assert!(failed.is_empty());
        assert_eq!(fs::read_to_string(dir.join("a.xyzq")).unwrap(), "new");
        assert_eq!(fs::read_to_string(out.join("a.xyzq")).unwrap(), "old");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn undo_moves_skips_already_missing_targets() {
        let dir = setup();
        let moves = vec![RecordedMove {
            from: dir.join("a.txt").display().to_string(),
            to: dir.join("Documents").join("a.txt").display().to_string(),
        }];

        let (undone, failed) = undo_moves(&moves);

        assert_eq!(undone, 0);
        assert!(failed.is_empty());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn sort_files_with_moves_omits_failed_moves() {
        let dir = setup();
        fs::write(dir.join("no_extension"), "blocker").unwrap();
        fs::write(dir.join("song.mp3"), "b").unwrap();

        let (report, moves) = sort_files_with_moves(&dir, &[], &[], false).unwrap();

        assert_eq!(report.files_sorted, 1);
        assert_eq!(moves.len(), 1);
        fs::remove_dir_all(&dir).unwrap();
    }
}
