import { readFileSync, writeFileSync, readdirSync, statSync, mkdirSync } from "node:fs";
import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { basename, join } from "node:path";
function walk(path) {
  return readdirSync(path).flatMap((name) => { const child = join(path, name); return statSync(child).isDirectory() ? walk(child) : [child]; });
}
const files = walk("src-tauri/target").filter((path) => /\.(dmg|msi|deb|rpm|AppImage)$/.test(path) || /[\\/]nsis[\\/].*\.exe$/.test(path));
if (!files.length) throw new Error("No release packages found");
mkdirSync("release-inventory", { recursive: true });
const checksums = files.map((path) => `${createHash("sha256").update(readFileSync(path)).digest("hex")}  ${basename(path)}`);
writeFileSync("release-inventory/SHA256SUMS.txt", checksums.join("\n") + "\n");
const result = spawnSync("cargo", ["metadata", "--locked", "--format-version", "1", "--manifest-path", "src-tauri/Cargo.toml"], { encoding: "utf8", maxBuffer: 20 * 1024 * 1024 });
if (result.status !== 0) throw new Error(result.stderr);
const packages = JSON.parse(result.stdout).packages;
const components = packages.map((pkg) => ({ type: "library", name: pkg.name, version: pkg.version, purl: `pkg:cargo/${pkg.name}@${pkg.version}`, licenses: pkg.license ? [{ expression: pkg.license }] : [] }));
writeFileSync("release-inventory/halite.cdx.json", JSON.stringify({ bomFormat: "CycloneDX", specVersion: "1.5", version: 1, components }, null, 2) + "\n");
writeFileSync("release-inventory/licenses.txt", packages.map((pkg) => `${pkg.name}\t${pkg.version}\t${pkg.license || "SEE SOURCE"}`).sort().join("\n") + "\n");
writeFileSync("release-inventory/packages.json", JSON.stringify(files.map((path) => ({ name: basename(path), bytes: statSync(path).size })), null, 2) + "\n");
for (const path of files) {
  const bytes = statSync(path).size;
  if (bytes > 900_000_000) throw new Error(`Package budget exceeded: ${basename(path)} (${bytes} bytes)`);
}
console.log(`Inventoried ${components.length} Rust components and ${files.length} packages; budget <900 MB/package`);
