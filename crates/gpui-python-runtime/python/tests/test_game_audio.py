"""Optional game-cue queue tests; these do not imply live audio playback."""
from threading import Event, Lock
import time
import unittest

from gpui_toolkit.game_audio import GameCueAdapter


class FakeCueBackend:
    def __init__(self, *, block_play: bool = False):
        self.calls = []
        self.lock = Lock()
        self.play_started = Event()
        self.release_play = Event()
        if not block_play:
            self.release_play.set()

    def preload(self, cue_ids):
        with self.lock:
            self.calls.append(("preload", tuple(cue_ids)))

    def play(self, cue_id, command_id):
        self.play_started.set()
        self.release_play.wait(timeout=1.0)
        with self.lock:
            self.calls.append(("play", cue_id, command_id))

    def pause(self):
        with self.lock:
            self.calls.append(("pause",))

    def resume(self):
        with self.lock:
            self.calls.append(("resume",))


class GameAudioAdapterTests(unittest.TestCase):
    def test_no_backend_is_silent_and_does_not_start_a_worker(self):
        adapter = GameCueAdapter()
        self.assertFalse(adapter.available)
        self.assertFalse(adapter.preload(("place", "win")))
        self.assertFalse(adapter.play("place", "event-1"))
        self.assertIsNone(adapter._worker)

    def test_backend_commands_are_queued_and_play_ids_are_deduplicated(self):
        backend = FakeCueBackend()
        adapter = GameCueAdapter(backend)
        self.assertTrue(adapter.preload(("place", "win")))
        self.assertTrue(adapter.play("place", "event-1"))
        self.assertFalse(adapter.play("win", "event-1"))
        self.assertTrue(adapter.pause())
        self.assertTrue(adapter.resume())
        adapter._queue.join()
        self.assertIn(("preload", ("place", "win")), backend.calls)
        self.assertEqual([call for call in backend.calls if call[0] == "play"],
                         [("play", "place", "event-1")])
        self.assertIn(("pause",), backend.calls)
        self.assertIn(("resume",), backend.calls)
        adapter.close()

    def test_full_queue_drops_cues_without_blocking_the_reducer(self):
        backend = FakeCueBackend(block_play=True)
        adapter = GameCueAdapter(backend, max_pending=1)
        self.assertTrue(adapter.play("place", "first"))
        self.assertTrue(backend.play_started.wait(timeout=0.5))
        self.assertTrue(adapter.play("rotate", "second"))
        started = time.monotonic()
        self.assertFalse(adapter.play("clear", "third"))
        self.assertLess(time.monotonic() - started, 0.05)
        self.assertEqual(adapter.dropped_cues, 1)
        backend.release_play.set()
        adapter._queue.join()
        adapter.close()

    def test_invalid_cue_ids_are_rejected(self):
        adapter = GameCueAdapter(FakeCueBackend())
        with self.assertRaises(ValueError):
            adapter.preload(("",))
        with self.assertRaises(ValueError):
            adapter.play("place", "")
        adapter.close()


if __name__ == "__main__":
    unittest.main()
