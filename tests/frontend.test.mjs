import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import { waitForTask, settleTask, dropTaskWaiter } from "../src/js/tasks.js";
const context = { window: {} }; vm.runInNewContext(readFileSync(new URL("../src/js/locales.js", import.meta.url), "utf8"), context);
const locales = context.window.HALITE_LOCALES;
function lookup(object, key) {
  const parts = key.split(".");
  for (let i = 0; i < parts.length; i++) {
    if (!object) return undefined;
    if (Object.hasOwn(object, parts.slice(i).join("."))) return object[parts.slice(i).join(".")];
    object = object[parts[i]];
  }
  return object;
}
test("all visible UI keys exist in every language", () => {
  const html = readFileSync(new URL("../src/index.html", import.meta.url), "utf8");
  for (const match of html.matchAll(/data-i18n(?:-ph|-alt)?="([^"]+)"/g)) {
    for (const [lang, data] of Object.entries(locales)) assert.equal(typeof lookup(data, match[1]), "string", `${lang}: ${match[1]}`);
  }
  for (const data of Object.values(locales)) {
    for (const key of ["settings.themeSystem", "settings.themeDark", "settings.themeLight", "studio.stages.generating", "tags.fields.album_artist"]) assert.ok(lookup(data, key));
  }
});
test("task completion is correlated by kind and id", async () => {
  const first = waitForTask("studio", "first");
  const second = waitForTask("download", "first");
  settleTask("studio", { task_id: "other" }, "done");
  settleTask("studio", { task_id: "first", path: "voice.wav" }, "done");
  settleTask("download", { task_id: "first" }, "cancelled");
  assert.equal((await first).payload.path, "voice.wav");
  assert.equal((await second).status, "cancelled");
  dropTaskWaiter("studio", "missing");
});
test("native safe area and all navigation targets remain present", () => {
  const html = readFileSync(new URL("../src/index.html", import.meta.url), "utf8");
  for (const tab of ["separate", "download", "lyrics", "tags", "studio", "settings"]) {
    assert.ok(html.includes(`data-tab="${tab}"`)); assert.ok(html.includes(`id="tab-${tab}"`));
  }
  const css = readFileSync(new URL("../src/styles/main.css", import.meta.url), "utf8");
  assert.match(css, /html\[data-platform="macos"\] \.sidebar \{ padding-top: 48px; \}/);
});
