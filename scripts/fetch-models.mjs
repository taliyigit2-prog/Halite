// Downloads and verifies the two ONNX models bundled with Halite.
// The upstream revision and SHA-256 digests are pinned so release builds are
// reproducible and never silently accept a changed or partial model.
import {
  createReadStream,
  createWriteStream,
  existsSync,
  mkdirSync,
  renameSync,
  rmSync,
  statSync,
} from "node:fs";
import { createHash } from "node:crypto";
import { once } from "node:events";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const destDir = join(scriptDir, "..", "src-tauri", "resources", "models");
const revision = "d54ed9eb60e258ea82131c6ee14578628816456a";

const models = [
  {
    name: "htdemucs.onnx",
    bytes: 316_446_953,
    sha256: "68d0bf16428ef66e692cdff8a9ccf28f1ef3f69440d57e58605a4cc55fcc5e74",
  },
  {
    name: "htdemucs_fp16weights.onnx",
    bytes: 165_612_636,
    sha256: "d05c269d0178d2a72ad484b10b11dd370193fc923201c3b27a99f848745db70a",
  },
];

function modelUrl(name) {
  return `https://huggingface.co/StemSplitio/htdemucs-onnx/resolve/${revision}/${name}`;
}

async function sha256(path) {
  const hash = createHash("sha256");
  const stream = createReadStream(path);
  stream.on("data", (chunk) => hash.update(chunk));
  await once(stream, "end");
  return hash.digest("hex");
}

async function isValid(path, model) {
  if (!existsSync(path) || statSync(path).size !== model.bytes) return false;
  return (await sha256(path)) === model.sha256;
}

async function download(model, dest) {
  const part = `${dest}.part`;
  let lastError;

  for (let attempt = 1; attempt <= 3; attempt += 1) {
    rmSync(part, { force: true });
    try {
      const response = await fetch(modelUrl(model.name), {
        redirect: "follow",
        signal: AbortSignal.timeout(30 * 60 * 1000),
      });
      if (!response.ok || !response.body) throw new Error(`HTTP ${response.status}`);

      const writer = createWriteStream(part, { flags: "wx" });
      const reader = response.body.getReader();
      let received = 0;
      try {
        for (;;) {
          const { done, value } = await reader.read();
          if (done) break;
          received += value.length;
          if (!writer.write(Buffer.from(value))) await once(writer, "drain");
          const pct = Math.min(100, Math.round((received / model.bytes) * 100));
          process.stdout.write(`\r[fetch-models] ${model.name} ${pct}%`);
        }
        writer.end();
        await once(writer, "finish");
      } catch (error) {
        writer.destroy();
        throw error;
      }

      if (!(await isValid(part, model))) {
        throw new Error("size or SHA-256 verification failed");
      }
      rmSync(dest, { force: true });
      renameSync(part, dest);
      process.stdout.write("\n");
      return;
    } catch (error) {
      lastError = error;
      rmSync(part, { force: true });
      if (attempt < 3) console.warn(`[fetch-models] retry ${attempt}/3: ${error.message}`);
    }
  }
  throw new Error(`failed to download ${model.name}: ${lastError?.message || lastError}`);
}

mkdirSync(destDir, { recursive: true });

for (const model of models) {
  const dest = join(destDir, model.name);
  if (await isValid(dest, model)) {
    console.log(`[fetch-models] ${model.name} verified`);
    continue;
  }
  if (existsSync(dest)) {
    console.warn(`[fetch-models] replacing invalid ${model.name}`);
    rmSync(dest, { force: true });
  }
  await download(model, dest);
}

console.log("[fetch-models] all bundled models verified");
