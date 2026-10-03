// Internationalization layer. Loads JSON locale files and applies them to
// elements carrying `data-i18n` (textContent) and `data-i18n-ph` (placeholder)
// attributes. Falls back to English for any missing key.

const SUPPORTED = ["en", "tr", "de", "es", "fr", "ru", "ja"];
const cache = {};

function getByPath(obj, path) {
  return path.split(".").reduce((acc, key) => (acc == null ? undefined : acc[key]), obj);
}

async function load(lang) {
  if (cache[lang]) return cache[lang];
  try {
    const res = await fetch(`./locales/${lang}.json`);
    const data = await res.json();
    cache[lang] = data;
    return data;
  } catch {
    cache[lang] = {};
    return {};
  }
}

export const i18n = {
  lang: "en",
  data: {},
  fallback: {},

  async init(lang) {
    this.fallback = await load("en");
    await this.setLanguage(lang || "en");
  },

  async setLanguage(lang) {
    if (!SUPPORTED.includes(lang)) lang = "en";
    this.lang = lang;
    this.data = lang === "en" ? this.fallback : await load(lang);
    document.documentElement.lang = lang;
    this.apply();
  },

  t(key, vars) {
    let str = getByPath(this.data, key) ?? getByPath(this.fallback, key) ?? key;
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
  },
};
