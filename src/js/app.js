import { invoke, listen, convertFileSrc, platform } from "./api.js";
import { i18n } from "./i18n.js";
import { theme } from "./theme.js";
import { initTags } from "./tags.js";
import { initStudio, refreshStudio } from "./studio.js";
import { waitForTask, settleTask, dropTaskWaiter } from "./tasks.js";

import { $, t, toast, formatBytes, errorText, operationError, runPathAction } from "./ui.js";
import { lyricsNowPlaying, lyricsSearch, stopLyricsSync } from "./lyrics.js";

const SUPPORTED_LANGS = ["en", "tr", "de", "es", "fr", "ru", "ja"];

// Resolve the UI language: an explicit choice wins, otherwise follow the OS.
function resolveLanguage(lang) {
  if (SUPPORTED_LANGS.includes(lang)) return lang;
  const candidates =
    navigator.languages && navigator.languages.length
      ? navigator.languages
      : [navigator.language || "en"];
  for (const l of candidates) {
    const code = String(l).toLowerCase().split("-")[0];
    if (SUPPORTED_LANGS.includes(code)) return code;
  }
  return "en";
}

const state = {
  settings: { language: "en", theme: "system", output_dir: null },
  files: [],
  models: [],
  jobs: [],
  taskId: null,
  dlTaskId: null,
  player: null,
};

// ---------- Init ----------
async function init() {
  document.documentElement.dataset.platform = platform();
  try {
    state.settings = (await invoke("get_settings")) || state.settings;
  } catch (e) {
    /* browser fallback */
  }

  const lang = resolveLanguage(state.settings.language);
  const wasAuto = state.settings.language !== lang;
  state.settings.language = lang;

  await i18n.init(lang);
  theme.setMode(state.settings.theme);
  theme.listen();

  await wireBackendEvents();
  await refreshModels();
  await refreshHistory();
  bindUI();
  initTags();
  await initStudio();
  $("#sel-language").value = lang;
  $("#sel-theme").value = ["light", "dark", "system"].includes(state.settings.theme)
    ? state.settings.theme
    : "system";
  await renderSystemInfo();
  i18n.apply();
  renderOutputDirs();

  if (wasAuto) {
    try {
      await invoke("set_settings", { settings: state.settings });
    } catch {}
  }
}

// ---------- Models ----------
async function refreshModels() {
  state.models = await invoke("get_models");
  const sel = $("#sel-model");
  const current = sel.value;
  sel.innerHTML = "";
  for (const m of state.models) {
    const opt = document.createElement("option");
    opt.value = m.id;
    opt.textContent = i18n.t(`models.${m.id}.name`);
    sel.appendChild(opt);
  }
  if (current && state.models.some((m) => m.id === current)) sel.value = current;

  renderModelsList();
}

function renderModelsList() {
  const list = $("#models-list");
  list.innerHTML = "";
  for (const m of state.models) {
    const item = document.createElement("div");
    item.className = "model-item";

    const info = document.createElement("div");
    info.className = "info";
    const name = document.createElement("div");
    name.className = "name";
    name.textContent = i18n.t(`models.${m.id}.name`);
    const desc = document.createElement("div");
    desc.className = "desc";
    desc.textContent = i18n.t(`models.${m.id}.desc`);
    info.appendChild(name);
    info.appendChild(desc);

    const size = document.createElement("span");
    size.className = "size";
    size.textContent = formatBytes(m.size_bytes);

    const badge = document.createElement("span");
    badge.className = "chip";
    badge.textContent = t("common.bundled");

    item.appendChild(info);
    item.appendChild(size);
    item.appendChild(badge);
    list.appendChild(item);
  }
}

// ---------- History ----------
async function refreshHistory() {
  state.jobs = await invoke("list_jobs");
  const wrap = $("#history");
  wrap.innerHTML = "";
  if (!state.jobs.length) return;
  for (const job of state.jobs) {
    wrap.appendChild(renderJob(job));
  }
}

function renderJob(job) {
  const item = document.createElement("div");
  item.className = "history-item";

  const info = document.createElement("div");
  info.className = "info";
  const name = document.createElement("div");
  name.className = "name";
  name.textContent = job.source_name;
  const meta = document.createElement("div");
  meta.className = "meta";
  meta.textContent = `${i18n.t(`models.${job.model_id}.name`) || job.model_id} · ${new Date(
    job.created_at
  ).toLocaleString()} · ${fmtDuration(job.duration_secs)}`;
  info.appendChild(name);
  info.appendChild(meta);

  const stems = document.createElement("div");
  stems.className = "history-stems";
  for (const stem of job.stems) {
    const chip = document.createElement("button");
    chip.className = "stem-chip";
    chip.textContent = i18n.t(`stems.${stem.name}`) || stem.name;
    chip.addEventListener("click", () => togglePreview(stem.path, chip));
    stems.appendChild(chip);
  }
  info.appendChild(stems);

  const actions = document.createElement("div");
  actions.className = "actions";
  actions.appendChild(iconBtn("reveal", t("common.reveal"), () => runPathAction("reveal_path", job.stems[0]?.path || job.output_dir)));
  actions.appendChild(iconBtn("delete", t("common.delete"), () => deleteJob(job.id), true));

  item.appendChild(info);
  item.appendChild(actions);
  return item;
}

function iconBtn(kind, title, onClick, danger = false) {
  const b = document.createElement("button");
  b.className = "icon-btn" + (danger ? " danger" : "");
  b.title = title;
  b.innerHTML = ICONS[kind] || "";
  b.addEventListener("click", onClick);
  return b;
}

const ICONS = {
  reveal:
    '<svg viewBox="0 0 24 24"><path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7-10-7-10-7Z"/><circle cx="12" cy="12" r="3"/></svg>',
  open:
    '<svg viewBox="0 0 24 24"><path d="M14 3h7v7M21 3l-9 9M10 5H5a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-5"/></svg>',
  delete:
    '<svg viewBox="0 0 24 24"><path d="M3 6h18M8 6V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2m3 0v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/></svg>',
};

function deleteJob(id) {
  if (!window.confirm(t("common.confirmDelete"))) return;
  invoke("delete_job", { id })
    .then(() => refreshHistory())
    .catch((e) => toast(errorText(e, "common.unexpected"), "error"));
}

// ---------- Audio preview ----------
async function togglePreview(path, chip) {
  if (!state.player) {
    state.player = new Audio();
    state.player.addEventListener("ended", clearActivePreview);
  }
  if (state.player.src === convertFileSrc(path) && !state.player.paused) {
    state.player.pause();
    chip.classList.remove("active");
    return;
  }
  document.querySelectorAll(".stem-chip.active").forEach((c) => c.classList.remove("active"));
  try {
    await invoke("allow_audio_preview", { path });
  } catch (error) {
    toast(errorText(error, "common.previewError"), "error");
    return;
  }
  state.player.src = convertFileSrc(path);
  state.player.play().catch(() => {
    clearActivePreview();
    toast(t("common.previewError"), "error");
  });
  chip.classList.add("active");
}

function clearActivePreview() {
  document.querySelectorAll(".stem-chip.active").forEach((chip) => chip.classList.remove("active"));
}

// ---------- Output dirs ----------
function renderOutputDirs() {
  const od = state.settings.output_dir;
  if (od) {
    $("#output-dir").textContent = od;
     $("#dl-output-dir").textContent = od;
  }
}

// ---------- System info ----------
async function renderSystemInfo() {
  try {
    const info = await invoke("get_system_info");
    $("#about-version").textContent = ` v${info.version} · ${info.os} (${info.arch})`;
  } catch {
    $("#about-version").textContent = "";
  }
}

// ---------- UI binding ----------
function bindUI() {
  // Tab switching
  document.querySelectorAll(".nav-item[data-tab]").forEach((btn) => {
    btn.addEventListener("click", () => switchTab(btn.dataset.tab));
  });

  // Separate tab
  $("#btn-pick").addEventListener("click", pickFiles);
  bindDropzone();

  $("#btn-output").addEventListener("click", () => pickFolder("#output-dir"));
  $("#btn-separate").addEventListener("click", startSeparation);
  $("#btn-cancel").addEventListener("click", cancelSeparation);
  $("#btn-analyze").addEventListener("click", analyzeFirstFile);

  // Download tab
  $("#btn-dl-output").addEventListener("click", () => pickFolder("#dl-output-dir"));
  $("#btn-download").addEventListener("click", startDownload);
  $("#btn-dl-cancel").addEventListener("click", cancelDownload);

  // Lyrics tab
  $("#btn-now-playing").addEventListener("click", lyricsNowPlaying);
  $("#btn-lyrics-search").addEventListener("click", lyricsSearch);

  // Settings
  $("#sel-language").addEventListener("change", (e) => saveSettings({ language: e.target.value }));
  $("#sel-theme").addEventListener("change", (e) => saveSettings({ theme: e.target.value }));

}

function switchTab(tab) {
  document.querySelectorAll(".nav-item[data-tab]").forEach((b) => b.classList.toggle("active", b.dataset.tab === tab));
  document.querySelectorAll(".tab").forEach((s) => s.classList.toggle("active", s.id === `tab-${tab}`));
  if (tab !== "lyrics") stopLyricsSync();
}

async function pickFiles() {
  try {
    const paths = await invoke("pick_audio_files");
    if (paths && paths.length) addFiles(paths);
  } catch (error) {
    toast(errorText(error, "common.unexpected"), "error");
  }
}

function bindDropzone() {
  const dz = $("#dropzone");
  dz.addEventListener("dragover", (e) => {
    e.preventDefault();
    dz.classList.add("dragover");
  });
  dz.addEventListener("dragleave", () => dz.classList.remove("dragover"));
  dz.addEventListener("drop", (e) => {
    e.preventDefault();
    dz.classList.remove("dragover");
  });
}

function addFiles(paths) {
  for (const p of paths) {
    const name = p.split(/[\\/]/).pop();
    if (!state.files.find((f) => f.path === p)) state.files.push({ name, path: p });
  }
  renderFiles();
}

function renderFiles() {
  const list = $("#file-list");
  list.innerHTML = "";
  for (const f of state.files) {
    const chip = document.createElement("div");
    chip.className = "file-chip";
    const span = document.createElement("span");
    span.textContent = f.name;
    const rm = document.createElement("span");
    rm.className = "remove";
    rm.textContent = "✕";
    rm.addEventListener("click", () => {
      state.files = state.files.filter((x) => x !== f);
      renderFiles();
    });
    chip.appendChild(span);
    chip.appendChild(rm);
    list.appendChild(chip);
  }
}

async function pickFolder(selector) {
  try {
    const dir = await invoke("pick_folder");
    if (dir) {
      $(selector).textContent = dir;
      state.settings.output_dir = dir;
      $("#output-dir").textContent = dir;
      $("#dl-output-dir").textContent = dir;
      await invoke("set_settings", { settings: state.settings });
    }
  } catch (error) {
    toast(errorText(error, "common.unexpected"), "error");
  }
}

function selectedOutputDir() {
  const label = $("#output-dir").textContent;
  const fallback = t("separate.defaultfolder");
  if (label && label !== fallback && !label.startsWith("(")) return label;
  return state.settings.output_dir || null;
}

// ---------- Separation ----------
async function startSeparation() {
  if (!state.files.length) return toast(t("separate.noFile"), "error");

  const modelId = $("#sel-model").value;
  if (!state.models.some((model) => model.id === modelId && model.installed)) {
    return toast(t("separate.modelMissing"), "error");
  }
  const mode = $("#sel-mode").value;
  const format = $("#sel-format").value;
  const startSec = parseFloat($("#trim-start").value) || 0;
  const endSecRaw = parseFloat($("#trim-end").value);
  if (startSec < 0 || (!isNaN(endSecRaw) && endSecRaw <= startSec)) {
    return toast(t("separate.invalidTrim"), "error");
  }

  $("#btn-separate").disabled = true;
  $("#btn-cancel").classList.remove("hidden");
  $("#progress-wrap").classList.remove("hidden");
  setProgress(0);

  // Build the job(s). For simplicity process files sequentially.
  try {
    for (const f of state.files) {
      setProgress(0);
      const taskId = crypto.randomUUID();
      state.taskId = taskId;
      const completion = waitForTask("separation", taskId);
      try {
        await invoke("separate", {
          taskId,
          options: {
            sourcePath: f.path,
            modelId,
            outputDir: selectedOutputDir(),
            format,
            mode,
            startSec: startSec || null,
            endSec: isNaN(endSecRaw) ? null : endSecRaw,
            useCoreml: true,
          },
        });
      } catch (error) {
        dropTaskWaiter("separation", taskId);
        throw error;
      }

      const outcome = await completion;
      if (outcome.status === "cancelled") {
        toast(t("separate.cancelled"));
        return;
      }
      if (outcome.status === "error") {
        throw new Error(outcome.payload.message);
      }
    }
    toast(t("separate.done"), "ok");
    await refreshHistory();
  } catch (error) {
    toast(operationError("separate.error", error), "error");
  } finally {
    resetSeparationUI();
  }
}

function cancelSeparation() {
  if (state.taskId) {
    invoke("cancel_separation", { taskId: state.taskId })
      .catch((error) => toast(errorText(error, "common.unexpected"), "error"));
  }
}

function resetSeparationUI() {
  $("#btn-separate").disabled = false;
  $("#btn-cancel").classList.add("hidden");
  $("#progress-wrap").classList.add("hidden");
  setProgress(0);
  state.taskId = null;
}

function setProgress(pct, label) {
  const p = Math.round(pct * 100);
  $("#progress-fill").style.width = p + "%";
  $("#progress-label").textContent = p + "%";
}

// ---------- Analysis ----------
async function analyzeFirstFile() {
  if (!state.files.length) return toast(t("separate.noFile"), "error");
  const result = $("#analysis-result");
  result.textContent = t("separate.analyzing");
  try {
    const a = await invoke("analyze", { path: state.files[0].path });
    const key = a.key_tonic && a.key_mode
      ? `${a.key_tonic} ${t(`music.${a.key_mode}`)}`
      : a.key;
    result.textContent = a.bpm > 0 ? `${a.bpm} BPM · ${key}${a.key_camelot ? " · " + a.key_camelot : ""}` : key;
  } catch (e) {
    result.textContent = "";
    toast(errorText(e, "common.unexpected"), "error");
  }
}

// ---------- Download ----------
async function startDownload() {
  const url = $("#dl-url").value.trim();
  if (!url) return toast(t("download.needUrl"), "error");
  const format = $("#dl-format").value;
  const label = $("#dl-output-dir").textContent;
  const fallback = t("separate.defaultfolder");
  const outputDir = label && label !== fallback && !label.startsWith("(")
    ? label
    : state.settings.output_dir || null;

  $("#btn-download").disabled = true;
  $("#btn-dl-cancel").classList.remove("hidden");
  $("#dl-progress-wrap").classList.remove("hidden");
  setDlProgress(0);

  try {
    const taskId = crypto.randomUUID();
    state.dlTaskId = taskId;
    const completion = waitForTask("download", taskId);
    try {
      await invoke("download", { taskId, url, outputDir, format });
    } catch (error) {
      dropTaskWaiter("download", taskId);
      throw error;
    }
    const outcome = await completion;
    if (outcome.status === "cancelled") {
      toast(t("download.cancelled"));
    } else if (outcome.status === "error") {
      toast(operationError("download.error", outcome.payload.message), "error");
    } else {
      toast(t("download.done"), "ok");
      appendDlResult(outcome.payload);
    }
  } catch (e) {
    toast(operationError("download.error", e), "error");
  } finally {
    resetDownloadUI();
  }
}

function appendDlResult(p) {
  const wrap = $("#dl-results");
  const item = document.createElement("div");
  item.className = "history-item";
  const info = document.createElement("div");
  info.className = "info";
  const name = document.createElement("div");
  name.className = "name";
  name.textContent = p.title;
  const meta = document.createElement("div");
  meta.className = "meta";
  meta.textContent = p.ext.toUpperCase();
  info.appendChild(name);
  info.appendChild(meta);
  const actions = document.createElement("div");
  actions.className = "actions";
  actions.appendChild(iconBtn("reveal", t("common.reveal"), () => runPathAction("reveal_path", p.path)));
  actions.appendChild(iconBtn("open", t("common.open"), () => runPathAction("open_path", p.path)));
  item.appendChild(info);
  item.appendChild(actions);
  wrap.prepend(item);
}

function cancelDownload() {
  if (state.dlTaskId) {
    invoke("cancel_download", { taskId: state.dlTaskId })
      .catch((error) => toast(errorText(error, "common.unexpected"), "error"));
  }
}

function resetDownloadUI() {
  $("#btn-download").disabled = false;
  $("#btn-dl-cancel").classList.add("hidden");
  $("#dl-progress-wrap").classList.add("hidden");
  setDlProgress(0);
  state.dlTaskId = null;
}

function setDlProgress(pct) {
  const p = Math.round(pct * 100);
  $("#dl-progress-fill").style.width = p + "%";
  $("#dl-progress-label").textContent = p + "%";
}

// ---------- Settings ----------
async function saveSettings(patch) {
  // Preserve preferences changed by feature modules since the last UI update.
  try { state.settings = await invoke("get_settings"); } catch {}
  state.settings = { ...state.settings, ...patch };
  if (patch.language) {
    await i18n.setLanguage(patch.language);
    await Promise.all([refreshModels(), refreshHistory()]);
    await refreshStudio();
    renderOutputDirs();
  }
  if (patch.theme) theme.setMode(patch.theme);
  try {
    await invoke("set_settings", { settings: state.settings });
  } catch (e) {
    toast(String(e), "error");
  }
}

// ---------- Helpers ----------
function fmtDuration(s) {
  if (!s) return "";
  const m = Math.floor(s / 60);
  const sec = Math.floor(s % 60);
  return `${m}:${String(sec).padStart(2, "0")}`;
}

// ---------- Backend events (global, e.g. drag-drop) ----------
async function wireBackendEvents() {
  // Tauri v2 forwards native drag-and-drop as these events.
  await Promise.all([
    listen("tauri://drag-drop", (payload) => {
      const paths = payload && payload.paths;
      if (Array.isArray(paths)) {
        addFiles(paths.filter((p) => /\.(mp3|wav|flac|m4a|ogg|aac|aiff|mp4)$/i.test(p)));
      }
      $("#dropzone").classList.remove("dragover");
    }),
    listen("tauri://drag-enter", () => $("#dropzone").classList.add("dragover")),
    listen("tauri://drag-leave", () => $("#dropzone").classList.remove("dragover")),
    listen("separation://progress", (payload) => {
      if (payload.task_id === state.taskId) setProgress(payload.pct);
    }),
    listen("separation://done", (payload) => settleTask("separation", payload, "done")),
    listen("separation://error", (payload) => settleTask("separation", payload, "error")),
    listen("separation://cancelled", (payload) => settleTask("separation", payload, "cancelled")),
    listen("download://progress", (payload) => {
      if (payload.task_id === state.dlTaskId) setDlProgress(payload.pct);
    }),
    listen("download://done", (payload) => settleTask("download", payload, "done")),
    listen("download://error", (payload) => settleTask("download", payload, "error")),
    listen("download://cancelled", (payload) => settleTask("download", payload, "cancelled")),
  ]);
}

init().catch((error) => {
  console.error(error);
  toast(errorText(error, "common.unexpected"), "error");
});
