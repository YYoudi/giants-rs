from pathlib import Path
import tempfile
import unittest

from tools.cinematic_smoke import validate


class CinematicSmokeTests(unittest.TestCase):
    def test_endpoint_cannot_hide_wrong_ticket_cleanup_or_missing_cycle(self):
        log = 'silent_capture=true audio_plugin=false audio_clock=false\n'
        for n, ticket in enumerate((1, 3, 5), 1):
            log += f'mission_text_mounted ticket={ticket} fresh_atlas=true\n'
            log += f'mission_text_released ticket={ticket} entities=4 atlas_images=2 remaining_assets=0 font_and_language_dropped=true\n'
            log += f'showcase_opening_cycle_released cycles={n} remaining_entities=0 movie_released=true input_injected=false\n'
        log += 'showcase_replay_cancelled remaining_entities=0 movie_released=true normal_ready_simulated=false\n'
        log += 'capture_ready frame=30\n'
        with tempfile.TemporaryDirectory() as directory:
            capture = Path(directory)/'menu.png'
            capture.write_bytes(b'\x89PNG\r\n\x1a\nsynthetic')
            self.assertTrue(all(validate(log, 0, capture).values()))
            for bad in (
                log.replace('released ticket=3', 'released ticket=1'),
                log.replace('remaining_assets=0', 'remaining_assets=1'),
                log.replace('cycles=2', 'cycles=1'),
                log.replace('audio_plugin=false', 'audio_plugin=true'),
                log+'cinematic_skip_probe input=escape simulated=true\n',
                log+'panicked at test',
            ):
                self.assertFalse(all(validate(bad, 0, capture).values()))
            self.assertFalse(all(validate(log, 'timeout', capture).values()))
            capture.unlink()
            self.assertFalse(all(validate(log, 0, capture).values()))
