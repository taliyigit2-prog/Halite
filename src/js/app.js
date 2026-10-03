import { invoke, listen, convertFileSrc, platform } from "./api.js";
import { i18n } from "./i18n.js";
import { theme } from "./theme.js";

const t = (k, v) => i18n.t(k, v);
const $ = (sel) => document.querySelector(sel);

const STEM_ORDER = ["drums", "bass", "other", "vocals"];

const state = {
  settings: { language: "en", theme: "system", output_dir: null },
  files: [],
  models: [],
  jobs: [],
  taskId: null,
  dlTaskId: null,
  player: null,
};

// ---------- Toast ----------
let toastTimer = null;
function toast(msg, type = "") {
  const el = $("#toast");
  el.textContent = msg;
  el.className = "toast show" + (type ? " " + type : "");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (el.className = "toast"), 3000);
}

// ---------- Init ----------
async function init() {
  try {
    state.settings = (await invoke("get_settings")) || state.settings;
  } catch (e) {
    /* browser fallback */
  }

  await i18n.init(state.settings.language);
  theme.setMode(state.settings.theme);
  theme.listen();

  await refreshModels();
  await refreshHistory();
  bindUI();
  await renderSystemInfo();
  i18n.apply();
  renderOutputDirs();
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
    opt.textContent = i18n.t(`models.${m.id}.name`) + (m.installed ? "" : ` — ${t("common.notinstalled")}`);
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

    const btn = document.createElement("button");
    btn.className = "btn " + (m.installed ? "btn-ghost" : "btn-primary");
    btn.textContent = m.installed ? t("common.deleteModel") : t("common.install");
    btn.addEventListener("click", () => (m.installed ? removeModel(m) : installModel(m)));

    item.appendChild(info);
    item.appendChild(size);
    item.appendChild(btn);
    list.appendChild(item);
  }
}

function installModel(model) {
  const list = $("#models-list");
  const item = list.querySelectorAll(".model-item")[state.models.indexOf(model)];
  const btn = item && item.querySelector(".btn");
  if (btn) btn.textContent = t("common.installing");
  invoke("install_model", { modelId: model.id })
    .then(() => {})
    .catch(() => {});
}

function removeModel(model) {
  invoke("delete_model", { modelId: model.id })
    .then(() => {
      toast(t("common.delete") + " ✓", "ok");
      refreshModels();
    })
    .catch((e) => toast(String(e), "error"));
}

async function ensureModel(modelId) {
  const models = await invoke("get_models");
  const m = models.find((x) => x.id === modelId);
  if (m && m.installed) return;
  await invoke("install_model", { modelId });
  await new Promise((resolve, reject) => {
    let settled = false;
    let un1 = null;
    let un2 = null;
    const cleanup = () => {
      if (un1) un1();
      if (un2) un2();
    };
    listen("model://done", (p) => {
      if (p.model_id === modelId && !settled) {
        settled = true;
        cleanup();
        resolve();
      }
    }).then((u) => (un1 = u));
    listen("model://error", (p) => {
      if (p.task_id === modelId && !settled) {
        settled = true;
        cleanup();
        reject(new Error(p.message));
      }
    }).then((u) => (un2 = u));
  });
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
  actions.appendChild(iconBtn("reveal", t("common.reveal"), () => invoke("reveal_path", { path: job.stems[0]?.path || job.output_dir })));
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
  invoke("delete_job", { id })
    .then(() => refreshHistory())
    .catch((e) => toast(String(e), "error"));
}

// ---------- Audio preview ----------
function togglePreview(path, chip) {
  if (!state.player) state.player = new Audio();
  if (state.player.src === convertFileSrc(path) && !state.player.paused) {
    state.player.pause();
    chip.classList.remove("active");
    return;
  }
  document.querySelectorAll(".stem-chip.active").forEach((c) => c.classList.remove("active"));
  state.player.src = convertFileSrc(path);
  state.player.play().catch(() => {});
  chip.classList.add("active");
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

  // Backend events
  wireBackendEvents();
}

function switchTab(tab) {
  document.querySelectorAll(".nav-item[data-tab]").forEach((b) => b.classList.toggle("active", b.dataset.tab === tab));
  document.querySelectorAll(".tab").forEach((s) => s.classList.toggle("active", s.id === `tab-${tab}`));
}

async function pickFiles() {
  const paths = await invoke("pick_audio_files");
  if (paths && paths.length) addFiles(paths);
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
    const name = p.split("/").pop();
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
  const dir = await invoke("pick_folder");
  if (dir) {
    $(selector).textContent = dir;
    state.settings.output_dir = dir;
    $("#output-dir").textContent = dir;
    $("#dl-output-dir").textContent = dir;
    try {
      await invoke("set_settings", { settings: state.settings });
    } catch {}
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
  const mode = $("#sel-mode").value;
  const format = $("#sel-format").value;
  const startSec = parseFloat($("#trim-start").value) || 0;
  const endSecRaw = parseFloat($("#trim-end").value);

  $("#btn-separate").disabled = true;
  $("#btn-cancel").classList.remove("hidden");
  $("#progress-wrap").classList.remove("hidden");
  setProgress(0);

  // Auto-install model if needed.
  try {
    await ensureModel(modelId);
    await refreshModels();
  } catch (e) {
    toast(String(e), "error");
    resetSeparationUI();
    return;
  }

  // Build the job(s). For simplicity process files sequentially.
  try {
    for (const f of state.files) {
      state.taskId = await invoke("separate", {
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
      }).then((r) => r.task_id);

      await waitForSeparation(state.taskId, f.name, mode, modelId);
    }
  } catch (e) {
    /* handled in waitForSeparation */
  }
}

function waitForSeparation(taskId, name, mode, modelId) {
  return new Promise(async (resolve) => {
    const unlisteners = [];
    const cleanup = () => unlisteners.forEach((u) => u && u());
    const on = (event, handler) => listen(event, handler).then((u) => unlisteners.push(u));

    await on("separation://progress", (p) => {
      if (p.task_id === taskId) setProgress(p.pct, t("separate.processing") + " " + name);
    });
    await on("separation://done", (p) => {
      if (p.task_id === taskId) {
        cleanup();
        toast(t("separate.done"), "ok");
        resetSeparationUI();
        refreshHistory();
        resolve();
      }
    });
    await on("separation://error", (p) => {
      if (p.task_id === taskId) {
        cleanup();
        toast(t("separate.error") + ": " + p.message, "error");
        resetSeparationUI();
        resolve();
      }
    });
    await on("separation://cancelled", (p) => {
      if (p.task_id === taskId) {
        cleanup();
        toast(t("separate.cancelled"), "");
        resetSeparationUI();
        resolve();
      }
    });
  });
}

function cancelSeparation() {
  if (state.taskId) invoke("cancel_separation", { taskId: state.taskId });
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
    result.textContent = a.bpm > 0 ? `${a.bpm} BPM · ${a.key}${a.key_camelot ? " · " + a.key_camelot : ""}` : a.key;
  } catch (e) {
    result.textContent = "";
    toast(String(e), "error");
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
    const r = await invoke("download", { url, outputDir, format });
    state.dlTaskId = r.task_id;
    waitForDownload(r.task_id);
  } catch (e) {
    toast(String(e), "error");
    resetDownloadUI();
  }
}

function waitForDownload(taskId) {
  const unlisteners = [];
  const cleanup = () => unlisteners.forEach((u) => u && u());
  const on = (event, handler) => listen(event, handler).then((u) => unlisteners.push(u));

  on("download://progress", (p) => {
    if (p.task_id === taskId) setDlProgress(p.pct);
  });
  on("download://done", (p) => {
    cleanup();
    toast(t("download.done"), "ok");
    appendDlResult(p);
    resetDownloadUI();
  });
  on("download://error", (p) => {
    cleanup();
    toast(t("download.error") + ": " + p.message, "error");
    resetDownloadUI();
  });
  on("download://cancelled", (p) => {
    cleanup();
    toast(t("download.cancelled"), "");
    resetDownloadUI();
  });
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
  actions.appendChild(iconBtn("reveal", t("common.reveal"), () => invoke("reveal_path", { path: p.path })));
  actions.appendChild(iconBtn("open", t("common.open"), () => invoke("open_path", { path: p.path })));
  item.appendChild(info);
  item.appendChild(actions);
  wrap.prepend(item);
}

function cancelDownload() {
  if (state.dlTaskId) invoke("cancel_download", { taskId: state.dlTaskId });
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

// ---------- Lyrics ----------
async function lyricsNowPlaying() {
  let np;
  try {
    np = await invoke("get_now_playing");
  } catch {
    np = null;
  }
  const box = $("#lyrics-nowplaying");
  if (!np) {
    box.classList.remove("hidden");
    box.innerHTML = `<span class="muted">${t("lyrics.nothingPlaying")}</span>`;
    return;
  }
  box.classList.remove("hidden");
  box.innerHTML = `
    <div class="artwork">${(np.track || "♪").charAt(0).toUpperCase()}</div>
    <div>
      <div class="np-title">${escapeHtml(np.track)}</div>
      <div class="np-artist">${escapeHtml(np.artist)}</div>
    </div>`;
  const lyrics = await invoke("get_lyrics", {
    track: np.track,
    artist: np.artist,
    album: np.album,
    duration: np.duration,
  });
  renderLyricsResult($("#lyrics-results"), lyrics, np.track, np.artist);
}

async function lyricsSearch() {
  const q = $("#lyrics-search").value.trim();
  if (!q) return;
  const results = await invoke("search_lyrics", { query: q });
  const wrap = $("#lyrics-results");
  wrap.innerHTML = "";
  if (!results.length) {
    wrap.innerHTML = `<span class="muted">${t("lyrics.notfound")}</span>`;
    return;
  }
  for (const r of results.slice(0, 6)) {
    renderLyricsResult(wrap, r, r.track_name, r.artist_name, true);
  }
}

function renderLyricsResult(container, lyrics, track, artist, append = false) {
  if (!append) container.innerHTML = "";
  if (!lyrics) {
    if (!append) container.innerHTML = `<span class="muted">${t("lyrics.notfound")}</span>`;
    return;
  }

  const card = document.createElement("div");
  card.className = "lyrics-card";
  const head = document.createElement("div");
  head.className = "lc-head";
  const title = document.createElement("span");
  title.className = "lc-title";
  title.textContent = lyrics.track_name || track || "";
  const art = document.createElement("span");
  art.className = "lc-artist";
  art.textContent = lyrics.artist_name || artist || "";
  head.appendChild(title);
  head.appendChild(art);
  card.appendChild(head);

  const body = document.createElement("div");
  body.className = "lyrics-body";
  if (lyrics.instrumental) {
    body.textContent = t("lyrics.instrumental");
  } else if (lyrics.synced_lyrics) {
    for (const line of parseLrc(lyrics.synced_lyrics)) {
      const el = document.createElement("div");
      el.className = "lyric-line";
      el.textContent = line.text;
      body.appendChild(el);
    }
  } else if (lyrics.plain_lyrics) {
    body.textContent = lyrics.plain_lyrics;
  } else {
    body.textContent = t("lyrics.notfound");
  }
  card.appendChild(body);
  container.appendChild(card);
}

function parseLrc(synced) {
  const lines = [];
  for (const raw of synced.split("\n")) {
    const m = raw.match(/\[(\d+):(\d+(?:[.,]\d+)?)\](.*)/);
    if (m) {
      lines.push({ time: parseInt(m[1], 10) * 60 + parseFloat(m[2].replace(",", ".")), text: m[3].trim() });
    } else if (raw.trim()) {
      lines.push({ time: null, text: raw.trim() });
    }
  }
  return lines.filter((l) => l.text);
}

// ---------- Settings ----------
async function saveSettings(patch) {
  state.settings = { ...state.settings, ...patch };
  if (patch.language) {
    await i18n.setLanguage(patch.language);
    refreshModels();
    refreshHistory();
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
function formatBytes(n) {
  if (!n) return "—";
  const units = ["B", "KB", "MB", "GB"];
  let i = 0;
  let v = n;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return v.toFixed(v < 10 ? 1 : 0) + " " + units[i];
}

function fmtDuration(s) {
  if (!s) return "";
  const m = Math.floor(s / 60);
  const sec = Math.floor(s % 60);
  return `${m}:${String(sec).padStart(2, "0")}`;
}

function escapeHtml(s) {
  return String(s || "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
}

// ---------- Backend events (global, e.g. drag-drop & model install progress) ----------
function wireBackendEvents() {
  // Tauri v2 forwards native drag-and-drop as these events.
  listen("tauri://drag-drop", (payload) => {
    const paths = payload && payload.paths;
    if (Array.isArray(paths)) {
      addFiles(paths.filter((p) => /\.(mp3|wav|flac|m4a|ogg|aac|aiff|mp4)$/i.test(p)));
    }
    $("#dropzone").classList.remove("dragover");
  });
  listen("tauri://drag-enter", () => $("#dropzone").classList.add("dragover"));
  listen("tauri://drag-leave", () => $("#dropzone").classList.remove("dragover"));

  listen("model://progress", (p) => {
    updateModelProgress(p.model_id, p.pct);
  });
  listen("model://done", async (p) => {
    toast(t("common.install") + " ✓", "ok");
    await refreshModels();
  });
  listen("model://error", (p) => {
    toast(t("separate.error") + ": " + p.message, "error");
    refreshModels();
  });
}

function updateModelProgress(modelId, pct) {
  const idx = state.models.findIndex((m) => m.id === modelId);
  if (idx < 0) return;
  const item = $("#models-list").querySelectorAll(".model-item")[idx];
  const btn = item && item.querySelector(".btn");
  if (btn) btn.textContent = t("common.installing") + " " + Math.round(pct * 100) + "%";
}

init();
