// Maintainer-only action pin refresh. Commit the resulting immutable SHAs.
import { readFileSync, writeFileSync } from "node:fs";
const cache = new Map();
for (const name of ["ci.yml", "release.yml"]) {
  const path = new URL(`../.github/workflows/${name}`, import.meta.url);
  let source = readFileSync(path, "utf8");
  for (const match of source.matchAll(/uses: ([\w-]+\/[\w-]+)@([^\s#]+)/g)) {
    const [, repo, ref] = match;
    if (/^[a-f0-9]{40}$/.test(ref)) continue;
    const key = `${repo}@${ref}`;
    if (!cache.has(key)) {
      const response = await fetch(`https://api.github.com/repos/${repo}/commits/${ref}`, { headers: { "User-Agent": "Halite-build" } });
      if (!response.ok) throw new Error(`${key}: ${response.status}`);
      cache.set(key, (await response.json()).sha);
    }
    source = source.replaceAll(`uses: ${key}`, `uses: ${repo}@${cache.get(key)} # ${ref}`);
  }
  writeFileSync(path, source);
}
console.log(`Pinned ${cache.size} actions`);
