# minimal-sorter

A minimal Windows app that sorts the loose files in a folder into subfolders
based on file type. Built with Rust and Tauri — a single small executable, no
installation required.

## Features

- Pick a folder, **Preview** the moves, then **Sort files**
- Sorts into ready-made groups for common file types:
  **Documents**, **Images**, **Audio**, **Video**, **Archives**,
  **Applications**, **Code**, and **Fonts**
- Create your own groups: map any extensions to a folder of your choice
- Custom groups override the defaults and are remembered between launches
- The last folder you picked is remembered and pre-selected the next time you open the app
- Open the selected folder in Explorer with one click
- Existing subfolders are never touched
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
3. Click **Browse**, select the folder you want to sort.
4. Click **Preview** to see exactly what will move (and what will be renamed).
5. Click **Sort files**.

The last folder you picked is remembered between launches, so next time it is
already shown and ready to sort.

Optionally, add custom groups first (see below).

## How sorting works

| Example file        | Ends up in            |
| ------------------- | --------------------- |
| `report.pdf`        | `Documents/report.pdf` |
| `photo.jpg`         | `Images/photo.jpg`     |
| `song.mp3`          | `Audio/song.mp3`       |
| `app.zip`           | `Archives/app.zip`     |
| `script.py`         | `Code/script.py`       |
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

The rule is applied the next time you sort. Your groups are stored in
`%APPDATA%\dev.minimal-sorter.app\groups.json` and survive restarts. The last
folder you picked is stored in `last-path.json` in the same directory. If that
file ever becomes unreadable, the app keeps a copy as `groups.json.bak` and
starts fresh instead of silently overwriting it.

## Downloads

Prebuilt binaries are published with each GitHub release — pick the latest
`minimal-sorter.exe`.
