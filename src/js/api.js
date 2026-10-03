// Minimal, defensive wrapper around the Tauri v2 global API injected via
// `withGlobalTauri: true`. No bundler, so everything is plain ES modules.

const T = () => (window.__TAURI__ || null);

export function invoke(cmd, args = {}) {
  const t = T();
  if (t && t.core && typeof t.core.invoke === "function") {
    return t.core.invoke(cmd, args);
  }
  if (t && typeof t.invoke === "function") {
    return t.invoke(cmd, args);
  }
  return Promise.reject(new Error("Tauri API is not available (run inside the desktop app)."));
}

export function listen(event, cb) {
  const t = T();
  const fn = t && t.event && typeof t.event.listen === "function" ? t.event.listen : null;
  if (!fn) return Promise.resolve(() => {});
  return fn(event, (e) => cb(e.payload || {}));
}

export function convertFileSrc(path) {
  const t = T();
  if (t && t.core && typeof t.core.convertFileSrc === "function") {
    return t.core.convertFileSrc(path);
  }
  return path;
}

export function platform() {
  const ua = navigator.userAgent || "";
  if (ua.includes("Mac")) return "macos";
  if (ua.includes("Windows")) return "windows";
  return "linux";
}
