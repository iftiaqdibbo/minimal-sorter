const { invoke } = window.__TAURI__.core;

const pathInput = document.getElementById("path");
const browseBtn = document.getElementById("browse");
const openBtn = document.getElementById("open");
const previewBtn = document.getElementById("preview-btn");
const sortBtn = document.getElementById("sort");
const groupForm = document.getElementById("add-group-form");
const groupNameInput = document.getElementById("group-name");
const groupExtsInput = document.getElementById("group-exts");
const addGroupBtn = document.getElementById("add-group");
const groupsList = document.getElementById("groups");
const statusEl = document.getElementById("status");
const previewBox = document.getElementById("preview-box");
const previewSummary = document.getElementById("preview-summary");
const previewList = document.getElementById("preview-list");
const previewNote = document.getElementById("preview-note");
const undoBtn = document.getElementById("undo");
const removeEmptyInput = document.getElementById("remove-empty");
const excludeInput = document.getElementById("exclude");
const cancelEditBtn = document.getElementById("cancel-edit");
const copyPreviewBtn = document.getElementById("copy-preview");

let selectedPath = null;
let customGroups = [];
let editingIndex = null;
let lastPreviewText = "";

function setStatus(text) {
  statusEl.textContent = text;
}

function normalizeExt(raw) {
  return raw.trim().replace(/^\.+/, "").toLowerCase();
}

function parseExtensions(value) {
  const seen = new Set();
  const extensions = [];
  for (const raw of value.split(",")) {
    const ext = normalizeExt(raw);
    if (ext && !seen.has(ext)) {
      seen.add(ext);
      extensions.push(ext);
    }
  }
  return extensions;
}

function parseExcluded() {
  return parseExtensions(excludeInput.value);
}

function groupInputState() {
  return {
    folder: groupNameInput.value.trim(),
    extensions: parseExtensions(groupExtsInput.value),
  };
}

function updateAddButton() {
  const { folder, extensions } = groupInputState();
  addGroupBtn.disabled = !folder || extensions.length === 0;
}

function exitEditMode() {
  editingIndex = null;
  groupNameInput.value = "";
  groupExtsInput.value = "";
  addGroupBtn.textContent = "Add group";
  cancelEditBtn.hidden = true;
  updateAddButton();
}

function renderGroups() {
  groupsList.textContent = "";
  for (let i = 0; i < customGroups.length; i++) {
    const group = customGroups[i];
    const li = document.createElement("li");
    const span = document.createElement("span");
    const extensions = group.extensions.map(normalizeExt).filter(Boolean).join(", ");
    span.textContent = `${group.folder} — ${extensions}`;

    const actions = document.createElement("span");
    actions.className = "actions";

    const edit = document.createElement("button");
    edit.type = "button";
    edit.textContent = "Edit";
    edit.addEventListener("click", () => {
      editingIndex = i;
      groupNameInput.value = group.folder;
      groupExtsInput.value = group.extensions.map(normalizeExt).filter(Boolean).join(", ");
      addGroupBtn.textContent = "Save";
      cancelEditBtn.hidden = false;
      updateAddButton();
    });

    const remove = document.createElement("button");
    remove.type = "button";
    remove.textContent = "Remove";
    remove.addEventListener("click", () => {
      customGroups = customGroups.filter((g) => g !== group);
      exitEditMode();
      setStatus("");
      persistGroups();
    });

    actions.append(edit, remove);
    li.append(span, actions);
    groupsList.append(li);
  }
}

function clearPreview() {
  previewBox.hidden = true;
  previewSummary.textContent = "";
  previewList.textContent = "";
  previewNote.textContent = "";
  lastPreviewText = "";
  copyPreviewBtn.hidden = true;
}

function updateButtons() {
  openBtn.disabled = !selectedPath;
  previewBtn.disabled = !selectedPath;
  sortBtn.disabled = !selectedPath;
}

async function persistGroups() {
  renderGroups();
  clearPreview();
  updateButtons();
  try {
    await invoke("save_groups", { groups: customGroups });
  } catch (err) {
    setStatus("Error saving groups: " + err);
  }
}

browseBtn.addEventListener("click", async () => {
  try {
    const picked = await invoke("pick_folder");
    if (picked) {
      selectedPath = picked;
      pathInput.value = picked;
      clearPreview();
      setStatus("");
      try {
        await invoke("save_last_path", { path: picked });
      } catch {
        // Remembering the folder is a convenience, not a hard requirement.
      }
      updateButtons();
    }
  } catch (err) {
    setStatus("Error: " + err);
  }
});

openBtn.addEventListener("click", async () => {
  if (!selectedPath) return;
  try {
    await invoke("open_folder", { path: selectedPath });
  } catch (err) {
    setStatus("Error: " + err);
  }
});

function previewRow(m) {
  const prefix =
    selectedPath.endsWith("/") || selectedPath.endsWith("\\")
      ? selectedPath
      : selectedPath + "/";
  const from = m.from.startsWith(prefix) ? m.from.slice(prefix.length) : m.from;
  const to = m.to.startsWith(prefix) ? m.to.slice(prefix.length) : m.to;
  return `${from} → ${to}`;
}

function renderPreview(report) {
  const cap = 200;
  const rows = report.moves.map(previewRow);
  previewList.textContent = "";
  for (const row of rows.slice(0, cap)) {
    const li = document.createElement("li");
    li.textContent = row;
    previewList.append(li);
  }
  const summary =
    report.moves.length === 0
      ? "Nothing to move — folder already tidy."
      : `Will move ${report.moves.length} file(s) into ${report.folders_used.length} folder(s).`;
  const note =
    report.renamed.length > 0
      ? `${report.renamed.length} file(s) will be renamed with a number — the newest keeps the original name.`
      : "";
  previewSummary.textContent = summary;
  previewNote.textContent = note;
  lastPreviewText = [summary, ...rows, note].filter(Boolean).join("\n");
  copyPreviewBtn.hidden = report.moves.length === 0;
  previewBox.hidden = false;
}

previewBtn.addEventListener("click", async () => {
  if (!selectedPath) return;
  previewBtn.disabled = true;
  try {
    const report = await invoke("preview_sort", {
      path: selectedPath,
      customGroups,
      excludedExtensions: parseExcluded(),
    });
    renderPreview(report);
    setStatus("");
  } catch (err) {
    setStatus("Preview failed: " + err);
  } finally {
    updateButtons();
  }
});

function conflictingExtensions(folder, extensions, skipIndex = -1) {
  const claimed = new Map();
  for (let i = 0; i < customGroups.length; i++) {
    if (i === skipIndex) {
      continue;
    }
    const group = customGroups[i];
    if (group.folder === folder) {
      continue;
    }
    for (const raw of group.extensions) {
      const ext = normalizeExt(raw);
      if (ext && !claimed.has(ext)) {
        claimed.set(ext, group.folder);
      }
    }
  }
  return extensions.filter((ext) => claimed.has(ext)).map((ext) => [ext, claimed.get(ext)]);
}

groupForm.addEventListener("submit", (event) => {
  event.preventDefault();
  const { folder, extensions } = groupInputState();
  if (!folder || extensions.length === 0) {
    setStatus("Enter a folder name and at least one extension.");
    return;
  }
  const conflicts = conflictingExtensions(folder, extensions, editingIndex);
  if (editingIndex !== null) {
    customGroups[editingIndex] = { folder, extensions };
  } else {
    customGroups.push({ folder, extensions });
  }
  exitEditMode();
  setStatus(
    conflicts
      .map(([ext, other]) => `"${ext}" now maps to "${folder}" instead of "${other}".`)
      .join(" ")
  );
  persistGroups();
});

cancelEditBtn.addEventListener("click", () => {
  exitEditMode();
  setStatus("");
});

groupNameInput.addEventListener("input", updateAddButton);
groupExtsInput.addEventListener("input", updateAddButton);

excludeInput.addEventListener("change", async () => {
  clearPreview();
  try {
    await invoke("save_excluded", { extensions: parseExcluded() });
  } catch (err) {
    setStatus("Error saving exclusions: " + err);
  }
});

sortBtn.addEventListener("click", async () => {
  sortBtn.disabled = true;
  try {
    const report = await invoke("sort_files", {
      path: selectedPath,
      customGroups,
      excludedExtensions: parseExcluded(),
      removeEmptyFolders: removeEmptyInput.checked,
    });
    let text =
      report.files_sorted === 0
        ? "Folder already tidy."
        : `Moved ${report.files_sorted} file(s) into ${report.folders_used.length} folder(s).`;
    if (report.renamed.length > 0) {
      text += ` Renamed ${report.renamed.length} (newest kept its name).`;
    }
    if (report.empty_folders_removed > 0) {
      text += ` Removed ${report.empty_folders_removed} empty folder(s).`;
    }
    if (report.failed.length > 0) {
      text += ` ${report.failed.length} file(s) could not be moved.`;
    }
    setStatus(text);
    if (report.files_sorted > 0) {
      undoBtn.disabled = false;
    }
  } catch (err) {
    setStatus("Error: " + err);
  } finally {
    clearPreview();
    updateButtons();
  }
});

undoBtn.addEventListener("click", async () => {
  undoBtn.disabled = true;
  try {
    const report = await invoke("undo_last_sort");
    let text = report.undone === 0 ? "Nothing to undo." : `Undid ${report.undone} move(s).`;
    if (report.failed.length > 0) {
      text += ` ${report.failed.length} could not be undone.`;
      undoBtn.disabled = false;
    }
    setStatus(text);
    clearPreview();
  } catch (err) {
    setStatus("Error: " + err);
    undoBtn.disabled = false;
  } finally {
    updateButtons();
  }
});

copyPreviewBtn.addEventListener("click", async () => {
  if (!lastPreviewText) {
    setStatus("Nothing to copy.");
    return;
  }
  try {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      await navigator.clipboard.writeText(lastPreviewText);
    } else {
      const ta = document.createElement("textarea");
      ta.value = lastPreviewText;
      document.body.appendChild(ta);
      ta.select();
      document.execCommand("copy");
      document.body.removeChild(ta);
    }
    setStatus("Preview copied to clipboard.");
  } catch (err) {
    setStatus("Could not copy: " + err);
  }
});

async function setupDragAndDrop() {
  try {
    const { getCurrentWebview } = window.__TAURI__.webview;
    await getCurrentWebview().onDragDropEvent(async (event) => {
      const payload = event.payload || event;
      if (payload.type !== "drop") return;
      const paths = payload.paths || [];
      if (paths.length === 0) return;
      const dropped = paths[0];
      const isDir = await invoke("is_directory", { path: dropped });
      if (!isDir) {
        setStatus("Drop a folder, not a file.");
        return;
      }
      selectedPath = dropped;
      pathInput.value = dropped;
      clearPreview();
      setStatus("");
      try {
        await invoke("save_last_path", { path: dropped });
      } catch {
        // Remembering the folder is a convenience; skip if it fails.
      }
      updateButtons();
    });
  } catch {
    // Drag-and-drop is a convenience; ignore if unavailable.
  }
}

(async () => {
  try {
    customGroups = await invoke("load_groups");
  } catch (err) {
    setStatus("Error loading groups: " + err);
  }
  renderGroups();
  updateAddButton();

  try {
    const excluded = await invoke("load_excluded");
    excludeInput.value = excluded.join(", ");
  } catch (err) {
    setStatus("Error loading exclusions: " + err);
  }

  try {
    const lastPath = await invoke("load_last_path");
    if (lastPath) {
      selectedPath = lastPath;
      pathInput.value = lastPath;
      updateButtons();
    }
  } catch {
    // Remembering the last folder is a convenience; skip if it fails.
  }

  try {
    undoBtn.disabled = !(await invoke("has_undo"));
  } catch {
    undoBtn.disabled = true;
  }

  setupDragAndDrop();
})();
