import { invoke, listen, convertFileSrc } from "./api.js";
import { i18n } from "./i18n.js";
import { $, t, toast, featureError, formatBytes, confirmDialog } from "./ui.js";
import { waitForTask, settleTask, dropTaskWaiter } from "./tasks.js";

const LANGUAGES = ["ar", "da", "de", "el", "en", "es", "fi", "fr", "he", "hi", "it", "ja", "ko", "ms", "nl", "no", "pl", "pt", "ru", "sv", "sw", "tr", "zh"];
let status, taskId = null, reference = null, profiles = [];
const GENERATION_CONTROLS = "#studio-preview, #studio-generate, #studio-mode, #studio-language, #studio-device, #studio-text, #studio-expression, #studio-guidance, #studio-seed, #studio-pick-reference, #studio-consent, #studio-profile, #studio-profile-name, #studio-save-profile, #studio-delete-profile";

export async function refreshStudio() {
  try {
    status = await invoke("studio_status");
    $("#studio-setup").classList.toggle("hidden", status.installed);
    $("#studio-setup-info").textContent = status.supported ? t("studio.setupInfo", { size: formatBytes(status.model_bytes), disk: formatBytes(status.required_free_bytes) }) : t("studio.unsupported");
    $("#studio-install").disabled = !status.supported || Boolean(taskId);
    $("#studio-pack-info").textContent = status.installed ? `${status.version} · ${formatBytes(status.installed_bytes)} · MIT` : t("studio.notReady");
    $("#studio-pack-location").textContent = status.installed ? status.location : "";
    $("#studio-verify").disabled = !status.installed || Boolean(taskId);
    $("#studio-remove").disabled = !status.installed || Boolean(taskId);
    updateControls();
  } catch (error) { $("#studio-status").textContent = featureError(error); }
}
function updateControls() {
  document.querySelectorAll(GENERATION_CONTROLS).forEach((element) => { element.disabled = Boolean(taskId); });
  $("#studio-preview").disabled = Boolean(taskId) || !status?.installed;
  $("#studio-generate").disabled = Boolean(taskId) || !status?.installed;
  $("#studio-cancel").classList.toggle("hidden", !taskId);
  $("#studio-progress").classList.toggle("hidden", !taskId);
  $("#studio-reference-panel").classList.toggle("hidden", $("#studio-mode").value !== "clone");
  $("#studio-delete-profile").disabled = Boolean(taskId) || !$("#studio-profile").value;
}
async function task(command, args = {}) {
  taskId = crypto.randomUUID();
  const id = taskId;
  const completion = waitForTask("studio", id);
  updateControls(); $("#studio-install").disabled = true; $("#studio-progress-fill").style.width = "0%";
  try {
    await invoke(command, { taskId: id, ...args });
    const outcome = await completion;
    if (outcome.status === "error") throw new Error(outcome.payload.message);
    if (outcome.status === "cancelled") { toast(t("studio.cancelled")); return; }
    if (outcome.payload.path) renderResult(outcome.payload.path);
    else toast(t("studio.prepared"), "ok");
    $("#studio-status").textContent = t(outcome.payload.path ? "studio.generated" : "studio.prepared");
  } catch (error) { dropTaskWaiter("studio", id); toast(featureError(error), "error"); $("#studio-status").textContent = featureError(error); }
  finally { taskId = null; updateControls(); await refreshStudio(); }
}
async function prepare() {
  if (!status?.supported) return;
  if (!await confirmDialog(t("studio.setupTitle"), t("studio.installConfirm", { size: formatBytes(status.model_bytes), disk: formatBytes(status.required_free_bytes) }))) return;
  await task("install_studio");
}
function options(preview) {
  const clone = $("#studio-mode").value === "clone";
  if (clone && !reference) throw new Error("HALITE_STUDIO_REFERENCE");
  if (clone && !$("#studio-consent").checked) throw new Error("HALITE_STUDIO_CONSENT");
  let text = $("#studio-text").value.trim();
  if (!text) throw new Error("HALITE_STUDIO_TEXT");
  if (preview) {
    const first = text.match(/^.{1,180}?(?:[.!?。！？](?:\s|$)|$)/su);
    text = first?.[0]?.trim() || text.slice(0, 180).replace(/\s+\S*$/, "");
  }
  return { text, language: $("#studio-language").value, seed: Number($("#studio-seed").value), exaggeration: Number($("#studio-expression").value), cfg_weight: Number($("#studio-guidance").value), device: $("#studio-device").value, reference: clone ? reference.path : null, consent: clone && $("#studio-consent").checked };
}
async function generate(preview = false) {
  try { await task("generate_speech", { options: options(preview) }); } catch (error) { toast(featureError(error), "error"); }
}
function renderResult(path) {
  const card = document.createElement("div"); card.className = "history-item";
  const info = document.createElement("div"); info.className = "info";
  const name = document.createElement("div"); name.className = "name"; name.textContent = t("studio.generated");
  const meta = document.createElement("div"); meta.className = "meta"; meta.textContent = "WAV · 24 kHz · Chatterbox V3 · PerTh";
  const audio = document.createElement("audio"); audio.controls = true; audio.preload = "metadata"; audio.className = "speech-player"; audio.src = convertFileSrc(path);
  info.append(name, meta, audio);
  const button = document.createElement("button"); button.className = "btn btn-ghost"; button.textContent = t("studio.export");
  button.addEventListener("click", async () => { try { await invoke("export_speech", { path }); } catch (error) { toast(featureError(error), "error"); } });
  card.append(info, button); $("#studio-results").prepend(card);
}
async function refreshProfiles() {
  profiles = await invoke("list_voice_profiles");
  const select = $("#studio-profile"); select.replaceChildren();
  const empty = document.createElement("option"); empty.value = ""; empty.textContent = t("studio.chooseProfile"); select.append(empty);
  for (const profile of profiles) { const option = document.createElement("option"); option.value = profile.id; option.textContent = profile.name; select.append(option); }
  updateControls();
}
function setReference(value) {
  reference = value; $("#studio-reference-name").textContent = value?.name || "";
  $("#studio-consent").checked = false;
}
export async function initStudio() {
  const names = new Intl.DisplayNames([i18n.lang], { type: "language" });
  for (const language of LANGUAGES) { const option = document.createElement("option"); option.value = language; option.textContent = names.of(language); $("#studio-language").append(option); }
  $("#studio-language").value = LANGUAGES.includes(i18n.lang) ? i18n.lang : "en";
  const settings = await invoke("get_settings");
  const preferences = settings.voice || {};
  if (LANGUAGES.includes(preferences.language)) $("#studio-language").value = preferences.language;
  for (const [field, key] of [["seed", "seed"], ["expression", "exaggeration"], ["guidance", "cfg_weight"], ["device", "device"]]) {
    if (preferences[key] != null) $(`#studio-${field}`).value = preferences[key];
  }
  for (const id of ["language", "seed", "expression", "guidance", "device"]) $(`#studio-${id}`).addEventListener("change", async () => {
    try { await invoke("save_voice_preferences", { preferences: { language: $("#studio-language").value, seed: Number($("#studio-seed").value), exaggeration: Number($("#studio-expression").value), cfg_weight: Number($("#studio-guidance").value), device: $("#studio-device").value } }); } catch (error) { toast(featureError(error), "error"); }
  });
  $("#studio-mode").addEventListener("change", updateControls);
  $("#studio-text").addEventListener("input", () => { $("#studio-text-count").textContent = `${$("#studio-text").value.length} / 3000`; });
  $("#studio-install").addEventListener("click", prepare);
  $("#studio-generate").addEventListener("click", () => generate());
  $("#studio-preview").addEventListener("click", () => generate(true));
  $("#studio-cancel").addEventListener("click", () => { if (taskId) invoke("cancel_studio", { taskId }).catch((error) => toast(featureError(error), "error")); });
  $("#studio-pick-reference").addEventListener("click", async () => { try { const file = await invoke("pick_studio_reference"); if (file) { setReference(file); $("#studio-profile").value = ""; } } catch (error) { toast(featureError(error), "error"); } });
  $("#studio-profile").addEventListener("change", () => { const profile = profiles.find((item) => item.id === $("#studio-profile").value); setReference(profile || null); updateControls(); });
  $("#studio-save-profile").addEventListener("click", async () => {
    if (!reference) return toast(t("studio.invalidReference"), "error");
    try { await invoke("save_voice_profile", { path: reference.path, name: $("#studio-profile-name").value.trim(), consent: $("#studio-consent").checked }); await refreshProfiles(); toast(t("studio.profileSaved"), "ok"); } catch (error) { toast(featureError(error), "error"); }
  });
  $("#studio-delete-profile").addEventListener("click", async () => {
    const id = $("#studio-profile").value; if (!id || !await confirmDialog(t("common.delete"), t("studio.deleteProfileConfirm"))) return;
    try { await invoke("delete_voice_profile", { id }); setReference(null); await refreshProfiles(); toast(t("studio.profileDeleted")); } catch (error) { toast(featureError(error), "error"); }
  });
  $("#studio-verify").addEventListener("click", async () => { $("#studio-verify").disabled = true; try { await invoke("verify_studio"); toast(t("studio.verified"), "ok"); } catch (error) { toast(featureError(error), "error"); } finally { await refreshStudio(); } });
  $("#studio-remove").addEventListener("click", async () => {
    if (!await confirmDialog(t("studio.remove"), t("studio.removeConfirm"))) return;
    try { await invoke("remove_studio"); $("#studio-results").replaceChildren(); toast(t("studio.removed")); } catch (error) { toast(featureError(error), "error"); } finally { await refreshStudio(); }
  });
  await Promise.all([
    listen("studio://progress", (event) => { if (event.task_id !== taskId) return; const pct = Math.round(event.pct * 100); $("#studio-progress-fill").style.width = `${pct}%`; $("#studio-progress-label").textContent = `${pct}%`; $("#studio-status").textContent = t(`studio.stages.${event.stage}`); }),
    ...["done", "error", "cancelled"].map((kind) => listen(`studio://${kind}`, (event) => settleTask("studio", event, kind))),
  ]);
  await refreshStudio(); await refreshProfiles();
}
