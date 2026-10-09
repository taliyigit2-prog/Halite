"""Halite's offline, one-request worker. Protocol JSON goes exclusively to stdout."""
import contextlib
import json
import os
from pathlib import Path
import re
import sys
import time

PROTOCOL = 1
LANGUAGES = {"ar", "da", "de", "el", "en", "es", "fi", "fr", "he", "hi", "it", "ja", "ko", "ms", "nl", "no", "pl", "pt", "ru", "sv", "sw", "tr", "zh"}
PROTOCOL_OUT = sys.stdout

def emit(kind, **values):
    print(json.dumps({"protocol": PROTOCOL, "kind": kind, **values}, ensure_ascii=False), file=PROTOCOL_OUT, flush=True)

def chunks(text, limit=180):
    """Keep sentence boundaries; split oversized sentences without dropping text."""
    result, current = [], ""
    for sentence in re.split(r"(?<=[.!?。！？])\s+|\n+", text.strip()):
        while len(sentence) > limit:
            split = sentence.rfind(" ", 0, limit + 1)
            if split < limit // 2:
                split = limit
            if current:
                result.append(current)
                current = ""
            result.append(sentence[:split].strip())
            sentence = sentence[split:].strip()
        if len(current) + len(sentence) + 1 > limit and current:
            result.append(current)
            current = ""
        current = (current + " " + sentence).strip()
    if current:
        result.append(current)
    return result

def validate(request):
    if request.get("protocol") != PROTOCOL or request.get("command") not in {"generate", "check"}:
        raise ValueError("HALITE_STUDIO_PROTOCOL")
    if request["command"] == "check":
        return
    text = request.get("text", "")
    if not isinstance(text, str) or not text.strip() or len(text) > 3000 or "\0" in text:
        raise ValueError("HALITE_STUDIO_TEXT")
    if request.get("language") not in LANGUAGES:
        raise ValueError("HALITE_STUDIO_LANGUAGE")
    if request.get("reference") and request.get("consent") is not True:
        raise ValueError("HALITE_STUDIO_CONSENT")
    for key, high in (("exaggeration", 2), ("cfg_weight", 1)):
        value = request.get(key, 0.5)
        if isinstance(value, bool) or not isinstance(value, (int, float)) or not 0 <= value <= high:
            raise ValueError("HALITE_STUDIO_OPTIONS")
    if request.get("device", "auto") not in {"auto", "cpu"}:
        raise ValueError("HALITE_STUDIO_OPTIONS")
    if not isinstance(request.get("seed", 0), int) or not 0 <= request.get("seed", 0) <= 2**32 - 1:
        raise ValueError("HALITE_STUDIO_OPTIONS")

def run(request):
    validate(request)
    started = time.monotonic()
    # Any output from third-party libraries is diagnostic, not part of our protocol.
    with contextlib.redirect_stdout(sys.stderr):
        import numpy as np
        import torch
        import soundfile as sf
        import huggingface_hub
        model_dir = Path(request["model_dir"]).resolve(strict=True)
        # The upstream Chinese tokenizer otherwise attempts a main-branch lookup.
        # Resolve its one ancillary file from our hash-verified local model pack.
        original_download = huggingface_hub.hf_hub_download
        def local_download(repo_id, filename, *args, **kwargs):
            if repo_id == "ResembleAI/chatterbox" and filename == "Cangjie5_TC.json":
                return str(model_dir / filename)
            return original_download(repo_id, filename, *args, local_files_only=True, **{key: value for key, value in kwargs.items() if key != "local_files_only"})
        huggingface_hub.hf_hub_download = local_download
        from chatterbox.mtl_tts import ChatterboxMultilingualTTS
        if request["command"] == "check":
            emit("done", device="mps" if torch.backends.mps.is_available() else "cuda" if torch.cuda.is_available() else "cpu", torch=torch.__version__)
            return
        device = "cpu"
        if request.get("device", "auto") == "auto":
            if torch.cuda.is_available():
                device = "cuda"
            elif torch.backends.mps.is_available():
                device = "mps"
        torch.set_num_threads(min(8, os.cpu_count() or 2))
        torch.manual_seed(request.get("seed", 0))
        np.random.seed(request.get("seed", 0))
        emit("progress", pct=0.03, stage="loading", device=device)
        model = ChatterboxMultilingualTTS.from_local(model_dir, device, t3_model="v3")
        parts = chunks(request["text"])
        reference = request.get("reference")
        if reference:
            info = sf.info(reference)
            if not 3 <= info.duration <= 30:
                raise ValueError("HALITE_STUDIO_REFERENCE")
            samples, _ = sf.read(reference, dtype="float32")
            if not np.isfinite(samples).all() or float(np.sqrt(np.mean(samples**2))) < 0.001:
                raise ValueError("HALITE_STUDIO_REFERENCE")
        audio = []
        duration = 0
        for index, text in enumerate(parts):
            emit("progress", pct=0.1 + 0.85 * index / len(parts), stage="generating", part=index + 1, parts=len(parts), device=device)
            wav = model.generate(text, language_id=request["language"], audio_prompt_path=reference if index == 0 else None, exaggeration=request.get("exaggeration", 0.5), cfg_weight=request.get("cfg_weight", 0.5))
            samples = wav.squeeze().cpu().numpy()
            if not np.isfinite(samples).all() or samples.size < 2400 or float(np.max(np.abs(samples))) < 0.0001:
                raise ValueError("HALITE_STUDIO_OUTPUT")
            duration += len(samples) / model.sr
            if duration > 600:
                raise ValueError("HALITE_STUDIO_DURATION")
            # Short edge fades and a pause prevent clicks without removing watermark.
            fade = min(int(model.sr * 0.005), len(samples) // 4)
            samples[:fade] *= np.linspace(0, 1, fade)
            samples[-fade:] *= np.linspace(1, 0, fade)
            if audio:
                audio.append(np.zeros(int(model.sr * 0.18), dtype=np.float32))
            audio.append(samples)
        output = Path(request["output"]).resolve()
        sf.write(output, np.concatenate(audio), model.sr, subtype="PCM_16")
        emit("done", device=device, duration=duration, elapsed=time.monotonic() - started, sample_rate=model.sr, watermark="PerTh")
        del model, audio
        if device == "mps":
            torch.mps.empty_cache()
        elif device == "cuda":
            torch.cuda.empty_cache()

def main():
    try:
        line = sys.stdin.readline(65537)
        if len(line) > 65536:
            raise ValueError("HALITE_STUDIO_PROTOCOL")
        run(json.loads(line))
    except Exception as error:
        message = str(error)
        code = re.search(r"HALITE_[A-Z_]+", message)
        emit("error", code=code.group() if code else "HALITE_STUDIO_FAILED")
        print(f"{type(error).__name__}: {message}", file=sys.stderr)
        return 1
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
