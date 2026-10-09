import importlib.util
import io
from pathlib import Path
import unittest

path = Path(__file__).resolve().parents[1] / "src-tauri/resources/studio-worker.py"
spec = importlib.util.spec_from_file_location("studio_worker", path)
worker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(worker)

class WorkerTests(unittest.TestCase):
    def test_sentence_chunking_preserves_text_and_limits(self):
        text = "Merhaba İstanbul. " + "Uzun bir cümle içinde doğru sınırlar korunmalı. " * 30
        parts = worker.chunks(text)
        self.assertTrue(all(len(part) <= 180 for part in parts))
        self.assertEqual(" ".join(parts), " ".join(text.split()))

    def test_languages_without_spaces_are_not_lost(self):
        text = "音声を生成します。" * 80
        parts = worker.chunks(text)
        self.assertEqual("".join(parts), text)
        self.assertTrue(all(len(part) <= 180 for part in parts))

    def test_rejects_unknown_commands_and_unconsented_cloning(self):
        with self.assertRaises(ValueError):
            worker.validate({"protocol": 1, "command": "shell"})
        with self.assertRaises(ValueError):
            worker.validate({"protocol": 1, "command": "generate", "text": "Hello", "language": "en", "reference": "voice.wav", "consent": False})

    def test_rejects_invalid_limits(self):
        base = {"protocol": 1, "command": "generate", "text": "Merhaba", "language": "tr"}
        worker.validate(base)
        for values in [{"text": "x" * 3001}, {"language": "invalid"}, {"exaggeration": float("nan")}, {"cfg_weight": 2}, {"seed": -1}, {"device": "shell"}]:
            with self.assertRaises(ValueError):
                worker.validate({**base, **values})

    def test_errors_are_json_protocol_not_stack_traces(self):
        original_in, original_out = worker.sys.stdin, worker.PROTOCOL_OUT
        output = io.StringIO()
        try:
            worker.sys.stdin = io.StringIO('{"protocol":1,"command":"invalid"}\n')
            worker.PROTOCOL_OUT = output
            self.assertEqual(worker.main(), 1)
            self.assertIn('"code": "HALITE_STUDIO_PROTOCOL"', output.getvalue())
        finally:
            worker.sys.stdin, worker.PROTOCOL_OUT = original_in, original_out

if __name__ == "__main__":
    unittest.main()
