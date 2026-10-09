import { invoke } from "./api.js";
import { $, t, toast, errorText, escapeHtml } from "./ui.js";
let lyricsSync = null;

// ---------- Lyrics ----------
export async function lyricsNowPlaying() {
  stopLyricsSync();
  const results = $("#lyrics-results");
  results.innerHTML = `<span class="muted">${t("lyrics.loading")}</span>`;
  let np;
  try {
    np = await invoke("get_now_playing");
  } catch (error) {
    results.innerHTML = "";
    toast(errorText(error, "lyrics.error"), "error");
    return;
  }
  const box = $("#lyrics-nowplaying");
  if (!np) {
    box.classList.remove("hidden");
    box.innerHTML = `<span class="muted">${t("lyrics.nothingPlaying")}</span>`;
    results.innerHTML = "";
    return;
  }
  box.classList.remove("hidden");
  box.innerHTML = `
    <div class="artwork">${escapeHtml((np.track || "♪").charAt(0).toUpperCase())}</div>
    <div>
      <div class="np-title">${escapeHtml(np.track)}</div>
      <div class="np-artist">${escapeHtml(np.artist)}</div>
    </div>`;
  try {
    const lyrics = await invoke("get_lyrics", {
      track: np.track,
      artist: np.artist,
      album: np.album,
      duration: np.duration,
    });
    const rendered = renderLyricsResult(results, lyrics, np.track, np.artist);
    if (lyrics?.synced_lyrics && rendered) startLyricsSync(np, rendered);
  } catch (error) {
    results.innerHTML = `<span class="muted">${t("lyrics.error")}</span>`;
    toast(errorText(error, "lyrics.error"), "error");
  }
}

export async function lyricsSearch() {
  stopLyricsSync();
  const q = $("#lyrics-search").value.trim();
  if (!q) return;
  const wrap = $("#lyrics-results");
  wrap.innerHTML = `<span class="muted">${t("lyrics.loading")}</span>`;
  try {
    const results = await invoke("search_lyrics", { query: q });
    wrap.innerHTML = "";
    if (!results.length) {
      wrap.innerHTML = `<span class="muted">${t("lyrics.notfound")}</span>`;
      return;
    }
    for (const r of results.slice(0, 6)) {
      renderLyricsResult(wrap, r, r.track_name, r.artist_name, true);
    }
  } catch (error) {
    wrap.innerHTML = `<span class="muted">${t("lyrics.error")}</span>`;
    toast(errorText(error, "lyrics.error"), "error");
  }
}

function renderLyricsResult(container, lyrics, track, artist, append = false) {
  if (!append) container.innerHTML = "";
  if (!lyrics) {
    if (!append) container.innerHTML = `<span class="muted">${t("lyrics.notfound")}</span>`;
    return null;
  }

  const card = document.createElement("div");
  card.className = "lyrics-card";
  const head = document.createElement("div");
  head.className = "lc-head";
  const title = document.createElement("span");
  title.className = "lc-title";
  title.textContent = lyrics.track_name || track || "";
  const art = document.createElement("span");
  art.className = "lc-artist";
  art.textContent = lyrics.artist_name || artist || "";
  head.appendChild(title);
  head.appendChild(art);
  card.appendChild(head);

  const body = document.createElement("div");
  body.className = "lyrics-body";
  if (lyrics.instrumental) {
    body.textContent = t("lyrics.instrumental");
  } else if (lyrics.synced_lyrics) {
    for (const line of parseLrc(lyrics.synced_lyrics)) {
      const el = document.createElement("div");
      el.className = "lyric-line";
      el.textContent = line.text;
      if (line.time != null) el.dataset.time = String(line.time);
      body.appendChild(el);
    }
  } else if (lyrics.plain_lyrics) {
    body.textContent = lyrics.plain_lyrics;
  } else {
    body.textContent = t("lyrics.notfound");
  }
  card.appendChild(body);
  container.appendChild(card);
  return body;
}

function parseLrc(synced) {
  const lines = [];
  const offsetMatch = synced.match(/^\[offset:([+-]?\d+)\]$/im);
  const offset = offsetMatch ? Number(offsetMatch[1]) / 1000 : 0;
  const timestamp = /\[(\d{1,3}):(\d{2}(?:[.,]\d{1,3})?)\]/g;
  for (const raw of synced.split("\n")) {
    const matches = Array.from(raw.matchAll(timestamp));
    if (matches.length) {
      const text = raw.replace(timestamp, "").trim();
      if (!text) continue;
      for (const match of matches) {
        const time = parseInt(match[1], 10) * 60 + parseFloat(match[2].replace(",", ".")) + offset;
        lines.push({ time: Math.max(0, time), text });
      }
    } else if (raw.trim() && !/^\[[a-z]+:/i.test(raw.trim())) {
      lines.push({ time: null, text: raw.trim() });
    }
  }
  return lines.filter((line) => line.text).sort((a, b) => (a.time ?? Infinity) - (b.time ?? Infinity));
}

function startLyricsSync(nowPlaying, body) {
  const trackKey = `${nowPlaying.track}\n${nowPlaying.artist}`;
  const sync = {
    body,
    trackKey,
    anchorPosition: Math.max(0, Number(nowPlaying.position) || 0),
    anchorTime: performance.now(),
    playing: Boolean(nowPlaying.playing),
    activeIndex: -1,
    refreshing: false,
    lastRefresh: performance.now(),
    timer: null,
  };

  const tick = async () => {
    if (lyricsSync !== sync || !document.body.contains(body)) return;
    const now = performance.now();
    const position = sync.anchorPosition + (sync.playing ? (now - sync.anchorTime) / 1000 : 0);
    const lines = Array.from(body.querySelectorAll(".lyric-line[data-time]"));
    let activeIndex = -1;
    for (let index = 0; index < lines.length; index++) {
      if (Number(lines[index].dataset.time) <= position + 0.08) activeIndex = index;
      else break;
    }
    if (activeIndex !== sync.activeIndex) {
      lines[sync.activeIndex]?.classList.remove("active");
      const active = lines[activeIndex];
      active?.classList.add("active");
      active?.scrollIntoView({ block: "center", behavior: "smooth" });
      sync.activeIndex = activeIndex;
    }

    if (!sync.refreshing && now - sync.lastRefresh >= 5000) {
      sync.refreshing = true;
      sync.lastRefresh = now;
      try {
        const current = await invoke("get_now_playing");
        if (!current || `${current.track}\n${current.artist}` !== sync.trackKey) {
          stopLyricsSync();
          return;
        }
        sync.anchorPosition = Math.max(0, Number(current.position) || 0);
        sync.anchorTime = performance.now();
        sync.playing = Boolean(current.playing);
      } catch {
        // Keep the local clock running; the next refresh may recover.
      } finally {
        sync.refreshing = false;
      }
    }
  };

  sync.timer = window.setInterval(tick, 250);
  lyricsSync = sync;
  tick();
}

export function stopLyricsSync() {
  if (!lyricsSync) return;
  window.clearInterval(lyricsSync.timer);
  lyricsSync = null;
}
