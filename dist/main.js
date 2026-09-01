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

let selectedPath = null;
let customGroups = [];

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

function renderGroups() {
  groupsList.textContent = "";
  for (const group of customGroups) {
    const li = document.createElement("li");
    const span = document.createElement("span");
    const extensions = group.extensions.map(normalizeExt).filter(Boolean).join(", ");
    span.textContent = `${group.folder} — ${extensions}`;
    const remove = document.createElement("button");
    remove.type = "button";
    remove.textContent = "Remove";
    remove.addEventListener("click", () => {
      customGroups = customGroups.filter((g) => g !== group);
      setStatus("");
      persistGroups();
    });
    li.append(span, remove);
    groupsList.append(li);
  }
}

function clearPreview() {
  previewBox.hidden = true;
  previewSummary.textContent = "";
  previewList.textContent = "";
  previewNote.textContent = "";
  sortBtn.disabled = true;
}

function updateButtons() {
  openBtn.disabled = !selectedPath;
  previewBtn.disabled = !selectedPath;
}

async function persistGroups() {
  renderGroups();
  clearPreview();
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
  previewSummary.textContent =
    report.moves.length === 0
      ? "Nothing to move — folder already tidy."
      : `Will move ${report.moves.length} file(s) into ${report.folders_used.length} folder(s).`;
  previewNote.textContent =
    report.renamed.length > 0
      ? `${report.renamed.length} file(s) will be renamed with a number — the newest keeps the original name.`
      : "";
  previewBox.hidden = false;
}

previewBtn.addEventListener("click", async () => {
  if (!selectedPath) return;
  previewBtn.disabled = true;
  try {
    const report = await invoke("preview_sort", {
      path: selectedPath,
      customGroups,
    });
    renderPreview(report);
    sortBtn.disabled = report.moves.length === 0;
    setStatus("");
  } catch (err) {
    setStatus("Preview failed: " + err);
  } finally {
    updateButtons();
  }
});

function conflictingExtensions(folder, extensions) {
  const claimed = new Map();
  for (const group of customGroups) {
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
  const conflicts = conflictingExtensions(folder, extensions);
  customGroups.push({ folder, extensions });
  groupNameInput.value = "";
  groupExtsInput.value = "";
  updateAddButton();
  setStatus(
    conflicts
      .map(([ext, other]) => `"${ext}" now maps to "${folder}" instead of "${other}".`)
      .join(" ")
  );
  persistGroups();
});

groupNameInput.addEventListener("input", updateAddButton);
groupExtsInput.addEventListener("input", updateAddButton);

sortBtn.addEventListener("click", async () => {
  sortBtn.disabled = true;
  try {
    const report = await invoke("sort_files", {
      path: selectedPath,
      customGroups,
    });
    let text =
      report.files_sorted === 0
        ? "Folder already tidy."
        : `Moved ${report.files_sorted} file(s) into ${report.folders_used.length} folder(s).`;
    if (report.renamed.length > 0) {
      text += ` Renamed ${report.renamed.length} (newest kept its name).`;
    }
    if (report.failed.length > 0) {
      text += ` ${report.failed.length} file(s) could not be moved.`;
    }
    setStatus(text);
  } catch (err) {
    setStatus("Error: " + err);
  } finally {
    clearPreview();
  }
});

(async () => {
  try {
    customGroups = await invoke("load_groups");
  } catch (err) {
    setStatus("Error loading groups: " + err);
  }
  renderGroups();
  updateAddButton();

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
})();
