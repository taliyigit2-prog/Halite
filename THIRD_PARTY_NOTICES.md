# Halite third-party notices

Halite is distributed under the MIT License. It uses open-source libraries listed
in `Cargo.lock` and `package-lock.json`, each under its own license.

## HT-Demucs ONNX models

Halite bundles `htdemucs.onnx` and `htdemucs_fp16weights.onnx` from
[StemSplitio/htdemucs-onnx](https://huggingface.co/StemSplitio/htdemucs-onnx),
derived from [facebookresearch/demucs](https://github.com/facebookresearch/demucs).
The model repository identifies these files as MIT licensed. Release builds use a
pinned upstream revision and verify the files with SHA-256 before packaging.

## yt-dlp

The Download feature obtains the official platform binary from
[yt-dlp](https://github.com/yt-dlp/yt-dlp) on first use and verifies it against the
release's published SHA-256 manifest. yt-dlp is distributed under The Unlicense.
It is not embedded in Halite's installer.

## FFmpeg

Halite uses an existing system FFmpeg when available. Features that require
transcoding may otherwise obtain a platform build through the MIT-licensed
[`ffmpeg-sidecar`](https://github.com/nathanbabcock/ffmpeg-sidecar) library and
store it in Halite's per-user application-data directory. FFmpeg builds are
licensed by their respective distributors and may include LGPL- or GPL-licensed
components. See [ffmpeg.org/legal.html](https://ffmpeg.org/legal.html) and the
notices supplied by the selected build distributor.

## LRCLIB

Lyrics are requested at runtime from [LRCLIB](https://lrclib.net). Lyrics are not
bundled with Halite and remain subject to their respective rights.

## Metadata editing

Halite uses [Lofty](https://github.com/Serial-ATA/lofty-rs) to read and write
local audio metadata. Lofty is dual-licensed under Apache-2.0 and MIT. Halite
does not send the selected audio file to a metadata service. The optional
MusicBrainz lookup sends only the text query entered by the user and is subject
to [MusicBrainz](https://musicbrainz.org/) terms and rate limits.

## Chatterbox Multilingual V3 voice runtime

The optional Voice Studio downloads the reviewed source revision and model
revision of [Resemble AI Chatterbox](https://github.com/resemble-ai/chatterbox)
on first use. Chatterbox source code and the V3 model are licensed under MIT;
the exact revision, file sizes and SHA-256 values are shipped in
`src-tauri/resources/studio-manifest.json` and verified before use.

The managed Python environment includes PyTorch, Torchaudio, Transformers,
Diffusers, PerTh, S3Tokenizer and their transitive dependencies under their own
licences. It is not bundled in the Halite installer. Generated Voice Studio
audio is marked with the PerTh watermarking library. See the installed runtime's
package metadata for the complete dependency notices.

## uv

The Voice Studio obtains the pinned `uv` runtime from
[Astral](https://github.com/astral-sh/uv) to create its isolated local Python
environment. uv is distributed under Apache-2.0 or MIT, at the user's option.

