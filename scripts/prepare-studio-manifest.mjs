// Maintainer-only: refresh reviewed upstream pins; never run during app startup.
import { writeFileSync } from "node:fs";
import { createHash } from "node:crypto";
const json = async (url) => {
  const response = await fetch(url, { headers: { "User-Agent": "Halite-build" } });
  if (!response.ok) throw new Error(`${url}: ${response.status}`);
  return response.json();
};
const digest = async (url) => {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`${url}: ${response.status}`);
  const bytes = Buffer.from(await response.arrayBuffer());
  return { sha256: createHash("sha256").update(bytes).digest("hex"), size: bytes.length };
};
const code = "5de7a54aa4e5e2baadb0182dde554908b48b85c2";
const sourceUrl = `https://codeload.github.com/resemble-ai/chatterbox/zip/${code}`;
const uv = await json("https://api.github.com/repos/astral-sh/uv/releases/latest");
const hf = await json("https://huggingface.co/api/models/ResembleAI/chatterbox?blobs=true");
const wanted = ["ve.pt", "t3_mtl23ls_v3.safetensors", "s3gen.pt", "grapheme_mtl_merged_expanded_v1.json", "conds.pt", "Cangjie5_TC.json"];
const files = [];
for (const name of wanted) {
  const file = hf.siblings.find((item) => item.rfilename === name);
  if (!file) throw new Error(`Missing model file ${name}`);
  const url = `https://huggingface.co/ResembleAI/chatterbox/resolve/${hf.sha}/${name}`;
  files.push({ name, url, ...(file.lfs ? { sha256: file.lfs.sha256, size: file.size } : await digest(url)) });
}
const platforms = {
  "macos-aarch64": "aarch64-apple-darwin.tar.gz",
  "macos-x86_64": "x86_64-apple-darwin.tar.gz",
  "linux-x86_64": "x86_64-unknown-linux-gnu.tar.gz",
  "linux-aarch64": "aarch64-unknown-linux-gnu.tar.gz",
  "windows-x86_64": "x86_64-pc-windows-msvc.zip",
};
const runtimes = {};
for (const [platform, suffix] of Object.entries(platforms)) {
  const asset = uv.assets.find((item) => item.name === `uv-${suffix}`);
  if (!asset) throw new Error(`Missing uv ${platform}`);
  runtimes[platform] = { url: asset.browser_download_url, sha256: asset.digest?.replace("sha256:", ""), size: asset.size };
  if (!runtimes[platform].sha256) Object.assign(runtimes[platform], await digest(asset.browser_download_url));
}
const manifest = {
  schema: 1, id: "chatterbox-multilingual-v3", license: "MIT",
  code_revision: code, model_revision: hf.sha, uv_version: uv.tag_name,
  python: "3.11.14", required_free_bytes: 12000000000,
  source: { url: sourceUrl, ...(await digest(sourceUrl)) }, runtimes, files,
};
writeFileSync(new URL("../src-tauri/resources/studio-manifest.json", import.meta.url), JSON.stringify(manifest, null, 2) + "\n");
console.log(`Pinned ${files.length} model files (${files.reduce((sum, file) => sum + file.size, 0)} bytes), code ${code}, uv ${uv.tag_name}`);
