# minimal-sorter

A minimal Windows app that sorts the loose files in a folder into subfolders
based on file type. Built with Rust and Tauri — a single small executable, no
installation required.

## Features

- Pick a folder (browse or drag-and-drop it onto the window), **Preview** the
  moves, then **Sort files**
- **Undo** the last sort to put files back where they were
- Sorts into ready-made groups for common file types:
  **Documents**, **Images**, **Audio**, **Video**, **Archives**,
  **Applications**, **Code**, **Fonts**, **eBooks**, **Subtitles**,
  **3D-CAD**, **Disk-Images**, **Data**, **Torrents**, and **Logs**
- Create and edit your own groups: map any extensions to a folder of your choice
- Custom groups override the defaults and are remembered between launches
- Exclude extensions you never want to move (e.g. `part`, `tmp`, `crdownload`)
- Copy the preview list to the clipboard
- Optionally remove empty subfolders after sorting
- The last folder you picked is remembered and pre-selected the next time you open the app
- Open the selected folder in Explorer with one click
- Subfolders are never touched (only empty ones are removed when you opt in)
- Windows system files (`desktop.ini`, `Thumbs.db`) are left alone
- If a file already exists in its target folder, it is renamed with a number
  (`report (1).pdf`, `report (2).pdf`, …) — the newest copy keeps its original name
- Files that cannot be moved (for example, open in another app) are reported
  and the rest of the folder is still sorted
- Simple, minimal UI that follows the system light/dark theme

## Requirements

- Windows 10 or 11 (the WebView2 runtime is preinstalled on virtually all
  current Windows machines)
- Nothing to install — download the executable and run it

## Usage

1. Download the latest `minimal-sorter.exe` from the Releases page.
2. Double-click it.
3. Click **Browse** — or drag a folder onto the window — to select the folder
   you want to sort.
4. Click **Preview** to see exactly what will move (and what will be renamed).
5. Click **Sort files**.

The last folder you picked is remembered between launches, so next time it is
already shown and ready to sort.

If you sort by mistake, click **Undo** to put the files back.

Optionally, add custom groups or excluded extensions first (see below).

## How sorting works

| Example file        | Ends up in            |
| ------------------- | --------------------- |
| `report.pdf`        | `Documents/report.pdf` |
| `photo.jpg`         | `Images/photo.jpg`     |
| `song.mp3`          | `Audio/song.mp3`       |
| `app.zip`           | `Archives/app.zip`     |
| `script.py`         | `Code/script.py`       |
| `book.mobi`         | `eBooks/book.mobi`     |
| `movie.srt`         | `Subtitles/movie.srt`  |
| `model.stl`         | `3D-CAD/model.stl`     |
| `disk.vmdk`         | `Disk-Images/disk.vmdk`|
| `data.sqlite`       | `Data/data.sqlite`     |
| `movie.torrent`     | `Torrents/movie.torrent`|
| `app.log`           | `Logs/app.log`         |
| `weird.xyzq`        | `xyzq/weird.xyzq`      |
| `README` (no ext.)  | `no_extension/README`  |

- Files whose extension matches no group go into a folder named after the
  extension itself.
- Subfolders inside the selected folder are left completely untouched.

## Custom groups

Use the **Custom groups** section in the app:

1. Enter a folder name (e.g. `Reports`).
2. Enter the extensions, comma-separated (e.g. `pdf, docx`).
3. Click **Add group**.

The rule is applied the next time you sort. Click **Edit** on a group to change
it, or **Remove** to delete it. Your groups are stored in
`%APPDATA%\dev.minimal-sorter.app\groups.json` and survive restarts. Excluded
extensions are stored in `excluded.json`, the last picked folder in
`last-path.json`, and the undo data in `last-sort.json` — all in the same
directory. If `groups.json` ever becomes unreadable, the app keeps a copy as
`groups.json.bak` and starts fresh instead of silently overwriting it.

## Excluding files

Use the **Exclude** field to list extensions that should never be moved, e.g.
`part, tmp, crdownload`. Files with those extensions are left exactly where they
are, even when everything else in the folder is sorted. This is handy for
in-progress downloads and temporary files.

## Downloads

Prebuilt binaries are published with each GitHub release — pick the latest
`minimal-sorter.exe`.
