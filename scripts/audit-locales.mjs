// Prune only reviewed legacy keys; dynamic model/stem keys are intentionally kept.
import { readFileSync, writeFileSync, readdirSync } from "node:fs";
const base = new URL("../src/locales/", import.meta.url);
const keys = ["common.install", "common.installed", "common.deleteModel", "common.installing", "common.downloading", "common.close", "common.notinstalled", "separate.processing", "separate.noModel", "separate.stemsReady", "download.installingYtDlp", "download.pickFolder", "lyrics.noSource", "settings.version", "settings.outputFolder"];
const sources = readFileSync(new URL("../src/index.html", import.meta.url), "utf8") + readdirSync(new URL("../src/js/", import.meta.url)).filter((name) => name.endsWith(".js") && name !== "locales.js").map((name) => readFileSync(new URL(`../src/js/${name}`, import.meta.url), "utf8")).join("\n");
const unused = keys.filter((key) => !sources.includes(key));
if (process.argv.includes("--prune")) {
  for (const name of readdirSync(base).filter((name) => /^[a-z]{2}\.json$/.test(name))) {
    const path = new URL(name, base), data = JSON.parse(readFileSync(path, "utf8"));
    for (const key of unused) { const [section, leaf] = key.split("."); delete data[section][leaf]; }
    writeFileSync(path, JSON.stringify(data, null, 2) + "\n");
  }
}
console.log(`Reviewed unused keys (${unused.length}): ${unused.join(", ")}`);
