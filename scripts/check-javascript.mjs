import { readdirSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const targets = [join(root, "src", "js"), join(root, "scripts")];

function javascriptFiles(path) {
  return readdirSync(path)
    .flatMap((name) => {
      const child = join(path, name);
      return statSync(child).isDirectory() ? javascriptFiles(child) : [child];
    })
    .filter((path) => path.endsWith(".js") || path.endsWith(".mjs"));
}

for (const path of targets.flatMap(javascriptFiles).sort()) {
  const result = spawnSync(process.execPath, ["--check", path], { encoding: "utf8" });
  if (result.status !== 0) {
    process.stderr.write(result.stderr || result.stdout);
    process.exit(result.status || 1);
  }
}

console.log("[javascript] syntax OK");

