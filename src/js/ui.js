import { i18n } from "./i18n.js";
import { invoke } from "./api.js";
export const $ = (selector) => document.querySelector(selector);
export const t = (key, values) => i18n.t(key, values);
let toastTimer;
export function toast(message, type = "") {
  const element = $("#toast");
  element.textContent = message;
  element.className = `toast show ${type}`;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => { element.className = "toast"; }, 6000);
}
export function formatBytes(bytes) {
  if (!bytes) return "—";
  const units = ["B", "KB", "MB", "GB"];
  let index = 0;
  while (bytes >= 1024 && index < units.length - 1) { bytes /= 1024; index++; }
  return `${bytes.toFixed(bytes < 10 ? 1 : 0)} ${units[index]}`;
}
export function featureError(error) {
  const code = String(error?.message || error || "").match(/HALITE_[A-Z_]+/)?.[0];
  const keys = {
    HALITE_TAG_ERROR: "tags.error", HALITE_TAG_CHANGED: "tags.changed",
    HALITE_STUDIO_FAILED: "studio.error", HALITE_STUDIO_PROTOCOL: "studio.error",
    HALITE_STUDIO_CHECKSUM: "studio.checksum", HALITE_STUDIO_DISK: "studio.disk",
    HALITE_STUDIO_BUSY: "studio.busy", HALITE_STUDIO_UNSUPPORTED: "studio.unsupported",
    HALITE_STUDIO_TEXT: "studio.invalidText", HALITE_STUDIO_LANGUAGE: "studio.invalidText",
    HALITE_STUDIO_CONSENT: "studio.consentRequired", HALITE_STUDIO_REFERENCE: "studio.invalidReference",
    HALITE_STUDIO_NOT_READY: "studio.notReady", HALITE_STUDIO_OUTPUT: "studio.error",
    HALITE_STUDIO_DURATION: "studio.tooLong", HALITE_STUDIO_TIMEOUT: "studio.timeout",
    HALITE_CANCELLED: "studio.cancelled",
  };
  return t(keys[code] || "common.unexpected");
}
export function confirmDialog(title, message, details = []) {
  return new Promise((resolve) => {
    const dialog = $("#confirm-dialog");
    $("#confirm-title").textContent = title;
    $("#confirm-message").textContent = message;
    const list = $("#confirm-details");
    list.replaceChildren();
    for (const detail of details) { const item = document.createElement("li"); item.textContent = detail; list.append(item); }
    const onClose = () => { dialog.removeEventListener("close", onClose); resolve(dialog.returnValue === "accept"); };
    dialog.addEventListener("close", onClose);
    dialog.returnValue = "cancel";
    dialog.showModal();
  });
}
export function escapeHtml(value) {
  return String(value || "").replace(/[&<>"']/g, (char) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[char]));
}
export function errorText(error, fallbackKey) {
  const code = String(error?.message || error || "").match(/HALITE_[A-Z_]+/)?.[0];
  const keys = {
    HALITE_AGE_RESTRICTED: "download.ageRestricted", HALITE_AUTH_REQUIRED: "download.authRequired",
    HALITE_HELPER_SETUP: "download.helperFailed", HALITE_DOWNLOAD_FAILED: "download.error",
    HALITE_OUTPUT_MISSING: "download.outputMissing", HALITE_INVALID_URL: "download.invalidUrl",
    HALITE_JS_RUNTIME_REQUIRED: "download.jsRuntimeRequired", HALITE_MODEL_MISSING: "separate.modelMissing",
    HALITE_INVALID_TRIM: "separate.invalidTrim", HALITE_INVALID_AUDIO: "separate.invalidAudio",
    HALITE_PREVIEW_DENIED: "common.previewError", HALITE_OUTPUT: "common.outputError",
    HALITE_OPEN_FAILED: "common.openError", HALITE_FFMPEG_SETUP: "common.audioConverterError",
    HALITE_FFMPEG_FAILED: "common.audioConverterError",
  };
  return t(keys[code] || fallbackKey || "common.unexpected");
}
export function operationError(prefixKey, error) {
  const prefix = t(prefixKey), detail = errorText(error, "common.unexpected");
  return detail === prefix ? prefix : `${prefix}: ${detail}`;
}
export function runPathAction(command, path) {
  if (path) invoke(command, { path }).catch((error) => toast(errorText(error, "common.openError"), "error"));
}
