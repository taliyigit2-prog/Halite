// Synthetic, original fixture for native UI tests and README screenshots.
import { mkdirSync } from "node:fs";
import { spawnSync } from "node:child_process";
const directory = "/private/tmp/halite-demo";
mkdirSync(directory, { recursive: true });
const path = `${directory}/Aurora.flac`;
const result = spawnSync("ffmpeg", ["-hide_banner", "-loglevel", "error", "-y", "-f", "lavfi", "-i", "sine=frequency=440:sample_rate=44100:duration=8", "-ac", "2", "-metadata", "title=Aurora", "-metadata", "artist=Halite", "-metadata", "album=Local Sessions", "-metadata", "genre=Electronic", "-metadata", "date=2026", "-metadata", "track=1", "-metadata", "comment=Original synthetic demo audio — CC0", path], { encoding: "utf8" });
if (result.status !== 0) throw new Error(result.stderr);
console.log(path);
