const { invoke } = window.__TAURI__.core;

const pathInput = document.getElementById("path");
const browseBtn = document.getElementById("browse");
const sortBtn = document.getElementById("sort");
const groupNameInput = document.getElementById("group-name");
const groupExtsInput = document.getElementById("group-exts");
const addGroupBtn = document.getElementById("add-group");
const groupsList = document.getElementById("groups");
const statusEl = document.getElementById("status");

let selectedPath = null;
let customGroups = [];

function setStatus(text) {
  statusEl.textContent = text;
}

function renderGroups() {
  groupsList.textContent = "";
  for (const group of customGroups) {
    const li = document.createElement("li");
    const span = document.createElement("span");
    span.textContent = `${group.folder} — ${group.extensions.join(", ")}`;
    const remove = document.createElement("button");
    remove.type = "button";
    remove.textContent = "Remove";
    remove.addEventListener("click", () => {
      customGroups = customGroups.filter((g) => g !== group);
      persistGroups();
    });
    li.append(span, remove);
    groupsList.append(li);
  }
}

async function persistGroups() {
  renderGroups();
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
      sortBtn.disabled = false;
      setStatus("");
    }
  } catch (err) {
    setStatus("Error: " + err);
  }
});

addGroupBtn.addEventListener("click", () => {
  const folder = groupNameInput.value.trim();
  const extensions = groupExtsInput.value
    .split(",")
    .map((e) => e.trim())
    .filter((e) => e.length > 0);
  if (!folder || extensions.length === 0) {
    setStatus("Enter a folder name and at least one extension.");
    return;
  }
  customGroups.push({ folder, extensions });
  groupNameInput.value = "";
  groupExtsInput.value = "";
  setStatus("");
  persistGroups();
});

sortBtn.addEventListener("click", async () => {
  sortBtn.disabled = true;
  try {
    const report = await invoke("sort_files", {
      path: selectedPath,
      customGroups,
    });
    let text =
      `Moved ${report.files_sorted} file(s) into ` +
      `${report.folders_created.length} folder(s).`;
    if (report.skipped.length > 0) {
      text += ` Skipped ${report.skipped.length} (name already in use).`;
    }
    setStatus(text);
  } catch (err) {
    setStatus("Error: " + err);
  } finally {
    sortBtn.disabled = false;
  }
});

(async () => {
  try {
    customGroups = await invoke("load_groups");
  } catch (err) {
    setStatus("Error loading groups: " + err);
  }
  renderGroups();
})();
