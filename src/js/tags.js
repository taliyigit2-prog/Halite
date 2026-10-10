import { invoke } from "./api.js";
import { $, t, toast, featureError, confirmDialog } from "./ui.js";
const FIELDS = ["title", "artist", "album", "album_artist", "genre", "date", "track", "track_total", "disc", "disc_total", "composer", "copyright", "comment", "lyrics"];
let files = [], selected = new Set(), dirty = new Map(), coverPath = null, removeCover = false, busy = false;

function activeFiles() { return files.filter((file) => selected.has(file.path)); }
function renderEditor() {
  dirty.clear(); coverPath = null; removeCover = false;
  const active = activeFiles();
  $("#tag-editor").classList.toggle("hidden", active.length === 0);
  $("#tag-count").textContent = t("tags.selected", { count: active.length });
  for (const name of FIELDS) {
    const input = $(`#tag-${name}`);
    const values = new Set(active.map((file) => file.fields[name] || ""));
    input.value = values.size === 1 ? [...values][0] : "";
    input.placeholder = values.size > 1 ? t("tags.multiple") : "";
    input.disabled = busy || active.some((file) => !file.supported_fields.includes(name));
  }
  const first = active[0];
  const image = $("#tag-cover");
  image.classList.toggle("hidden", !first?.cover);
  if (first?.cover) image.src = first.cover; else image.removeAttribute("src");
  $("#tag-cover-info").textContent = first?.cover ? t("tags.coverPresent", { count: first.cover_count }) : t("tags.noCover");
  $("#tag-technical").textContent = active.length === 1 ? `${Math.round(first.duration)} s · ${first.sample_rate || "—"} Hz · ${first.channels || "—"} ch · ${first.bitrate || "—"} kbps` : "";
  $("#tag-restore").disabled = active.length !== 1 || !first?.has_backup;
  for (const id of ["#tag-cover-pick", "#tag-cover-remove"]) $(id).disabled = active.some((file) => !file.supports_cover);
  $("#tag-export-cover").disabled = active.length !== 1 || !first?.cover;
}
function renderList() {
  const list = $("#tag-files"); list.replaceChildren();
  for (const file of files) {
    const label = document.createElement("label"); label.className = "file-chip";
    const checkbox = document.createElement("input"); checkbox.type = "checkbox"; checkbox.checked = selected.has(file.path); checkbox.disabled = busy;
    checkbox.addEventListener("change", () => { if (checkbox.checked) selected.add(file.path); else selected.delete(file.path); renderEditor(); });
    const name = document.createElement("span"); name.textContent = file.name; label.append(checkbox, name); list.append(label);
  }
  renderEditor();
}
function setBusy(value) {
  busy = value;
  document.querySelectorAll("#tab-tags button, #tab-tags input, #tab-tags textarea").forEach((element) => { element.disabled = value; });
  $("#tag-status").textContent = value ? t("tags.saving") : "";
  if (!value) {
    const active = activeFiles();
    for (const name of FIELDS) $(`#tag-${name}`).disabled = active.some((file) => !file.supported_fields.includes(name));
    for (const id of ["#tag-cover-pick", "#tag-cover-remove"]) $(id).disabled = active.some((file) => !file.supports_cover);
    $("#tag-restore").disabled = active.length !== 1 || !active[0]?.has_backup;
    $("#tag-export-cover").disabled = active.length !== 1 || !active[0]?.cover;
  }
}
async function pickFiles() {
  try {
    const added = await invoke("pick_tag_files");
    if (!added.length) return;
    for (const file of added) { const old = files.findIndex((item) => item.path === file.path); if (old >= 0) files[old] = file; else files.push(file); selected.add(file.path); }
    renderList();
  } catch (error) { toast(featureError(error), "error"); }
}
async function save() {
  const active = activeFiles();
  if (!active.length || (!dirty.size && !coverPath && !removeCover)) { toast(t("tags.noChanges")); return; }
  const fields = Object.fromEntries(dirty);
  const summary = [...dirty].map(([name, value]) => `${t(`tags.fields.${name}`)}: ${value || t("tags.clearValue")}`);
  if (coverPath) summary.push(t("tags.replaceCover"));
  if (removeCover) summary.push(t("tags.removeCover"));
  if (!await confirmDialog(t("tags.confirmTitle"), t("tags.confirmSave", { count: active.length }), summary)) return;
  setBusy(true);
  try {
    const results = await invoke("save_tags", { edits: active.map((file) => ({ path: file.path, expected: file.stamp, fields, cover_path: coverPath, remove_cover: removeCover })) });
    const failed = [];
    for (const result of results) {
      if (result.metadata) files[files.findIndex((file) => file.path === result.path)] = result.metadata;
      else failed.push(`${result.path.split(/[\\/]/).pop()}: ${featureError(result.error)}`);
    }
    renderList();
    $("#tag-errors").textContent = failed.join("\n");
    toast(failed.length ? t("tags.partial", { count: failed.length }) : t("tags.saved"), failed.length ? "error" : "ok");
  } catch (error) { toast(featureError(error), "error"); } finally { setBusy(false); }
}
async function restore() {
  const file = activeFiles()[0]; if (!file) return;
  if (!await confirmDialog(t("tags.restore"), t("tags.restoreConfirm"))) return;
  setBusy(true);
  try { const metadata = await invoke("restore_tags", { path: file.path }); files[files.findIndex((item) => item.path === file.path)] = metadata; renderList(); toast(t("tags.restored"), "ok"); }
  catch (error) { toast(featureError(error), "error"); } finally { setBusy(false); }
}
async function search() {
  const query = $("#tag-search").value.trim(); if (!query) return;
  const list = $("#tag-search-results"); list.replaceChildren();
  $("#tag-search-button").disabled = true;
  try {
    const response = await invoke("search_metadata", { query });
    for (const recording of response.recordings || []) {
      const button = document.createElement("button"); button.className = "btn btn-ghost metadata-match";
      const artist = (recording["artist-credit"] || []).map((credit) => (credit.name || credit.artist?.name || "") + (credit.joinphrase || "")).join("");
      const release = recording.releases?.[0];
      button.textContent = `${recording.title} · ${artist} · ${release?.title || ""}`;
      button.addEventListener("click", () => {
        const fields = { title: recording.title, artist, album: release?.title, date: release?.date };
        for (const [name, value] of Object.entries(fields)) { if (value && !$(`#tag-${name}`).disabled) { $(`#tag-${name}`).value = value; dirty.set(name, value); } }
        toast(t("tags.matchApplied"));
      });
      list.append(button);
    }
    if (!list.childElementCount) list.textContent = t("tags.noMatches");
  } catch (error) { toast(t("tags.searchError"), "error"); }
  finally { $("#tag-search-button").disabled = false; }
}
export function initTags() {
  const form = $("#tag-fields");
  for (const name of FIELDS) {
    const field = document.createElement("div"); field.className = `field ${["comment", "lyrics"].includes(name) ? "full-width" : "grow"}`;
    const label = document.createElement("label"); label.htmlFor = `tag-${name}`; label.dataset.i18n = `tags.fields.${name}`;
    const input = document.createElement(["comment", "lyrics"].includes(name) ? "textarea" : "input"); input.id = `tag-${name}`; input.className = "input";
    if (input.tagName === "TEXTAREA") input.rows = name === "lyrics" ? 5 : 2;
    input.maxLength = name === "lyrics" ? 100000 : 1000;
    input.addEventListener("input", () => dirty.set(name, input.value));
    field.append(label, input); form.append(field);
  }
  $("#tag-pick").addEventListener("click", pickFiles);
  $("#tag-clear").addEventListener("click", () => { files = []; selected.clear(); renderList(); $("#tag-search-results").replaceChildren(); $("#tag-errors").textContent = ""; });
  $("#tag-save").addEventListener("click", save);
  $("#tag-restore").addEventListener("click", restore);
  $("#tag-cover-pick").addEventListener("click", async () => { try { const path = await invoke("pick_cover"); if (path) { coverPath = path; removeCover = false; $("#tag-cover-info").textContent = path.split(/[\\/]/).pop(); } } catch (error) { toast(featureError(error), "error"); } });
  $("#tag-cover-remove").addEventListener("click", () => { removeCover = true; coverPath = null; $("#tag-cover").classList.add("hidden"); $("#tag-cover-info").textContent = t("tags.removeCover"); });
  $("#tag-export-cover").addEventListener("click", async () => { try { await invoke("export_cover", { path: activeFiles()[0].path }); } catch (error) { toast(featureError(error), "error"); } });
  $("#tag-search-button").addEventListener("click", search);
  $("#tag-search").addEventListener("keydown", (event) => { if (event.key === "Enter") search(); });
}
