// Internationalization layer. Translations are embedded in `locales.js`
// (window.HALITE_LOCALES) so they load without runtime fetch(). Applies text to
// elements carrying `data-i18n` (textContent) and `data-i18n-ph` (placeholder).
// Falls back to English for any missing key.

const SUPPORTED = ["en", "tr", "de", "es", "fr", "ru", "ja"];
const LOCALES = (typeof window !== "undefined" && window.HALITE_LOCALES) || {};

function getByPath(obj, path) {
  if (obj == null) return undefined;
  const parts = path.split(".");
  let current = obj;
  for (let index = 0; index < parts.length; index += 1) {
    if (current == null || typeof current !== "object") return undefined;
    // Some locale sections intentionally use leaf keys such as "mode.all".
    const remaining = parts.slice(index).join(".");
    if (Object.prototype.hasOwnProperty.call(current, remaining)) return current[remaining];
    if (!Object.prototype.hasOwnProperty.call(current, parts[index])) return undefined;
    current = current[parts[index]];
  }
  return current;
}

export const i18n = {
  lang: "en",
  data: {},
  fallback: {},

  init(lang) {
    this.fallback = LOCALES["en"] || {};
    this.setLanguage(lang || "en");
  },

  setLanguage(lang) {
    if (!SUPPORTED.includes(lang)) lang = "en";
    this.lang = lang;
    this.data = LOCALES[lang] || this.fallback;
    document.documentElement.lang = lang;
    this.apply();
  },

  t(key, vars) {
    let str = getByPath(this.data, key) ?? getByPath(this.fallback, key) ?? getByPath(this.fallback, "common.unexpected") ?? "Unavailable";
    if (vars && typeof vars === "object") {
      for (const [k, v] of Object.entries(vars)) {
        str = str.replaceAll(`{${k}}`, String(v));
      }
    }
    return str;
  },

  apply(root) {
    const scope = root || document;
    scope.querySelectorAll("[data-i18n]").forEach((el) => {
      el.textContent = this.t(el.getAttribute("data-i18n"));
    });
    scope.querySelectorAll("[data-i18n-ph]").forEach((el) => {
      el.setAttribute("placeholder", this.t(el.getAttribute("data-i18n-ph")));
    });
    scope.querySelectorAll("[data-i18n-alt]").forEach((el) => {
      el.setAttribute("alt", this.t(el.getAttribute("data-i18n-alt")));
    });
  },
};
