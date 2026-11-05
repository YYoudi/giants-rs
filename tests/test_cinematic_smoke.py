from pathlib import Path
import tempfile
import unittest

from tools.cinematic_smoke import validate


class CinematicSmokeTests(unittest.TestCase):
    def test_persistent_cancel_requires_live_owners_then_matching_cleanup(self):
        log = 'silent_capture=true audio_plugin=false audio_clock=false\n'
        for n, ticket in enumerate((1, 3, 5), 1):
            log += f'mission_text_mounted ticket={ticket} fresh_atlas=true\n'
            log += f'mission_opening_audio_policy ticket={ticket} playback_enabled=false queued_players=0\n'
            log += f'showcase_persistent_cancel ticket={ticket} objects=2 input_injected=false observation_gate=two_owned_updates\n'
            log += f'mission_persistent_released ticket={ticket} director=1 retired_objects=2 remaining_objects=0 remaining_contexts=0 lifetime_children=0\n'
            log += f'mission_text_released ticket={ticket} entities=4 atlas_images=2 remaining_assets=0 font_and_language_dropped=true\n'
            log += f'showcase_opening_cycle_released cycles={n} remaining_entities=0 movie_released=true input_injected=false\n'
        log += 'showcase_replay_cancelled remaining_entities=0 movie_released=true normal_ready_simulated=false\ncapture_ready frame=30\n'
        with tempfile.TemporaryDirectory() as directory:
            capture = Path(directory)/'menu.png'
            capture.write_bytes(b'\x89PNG\r\n\x1a\nsynthetic')
            self.assertTrue(all(validate(log, 0, capture, persistent_cancel=True).values()))
            for bad in (log.replace('retired_objects=2', 'retired_objects=1'),
                        log.replace('remaining_contexts=0', 'remaining_contexts=1'),
                        log.replace('objects=2 input_injected', 'objects=0 input_injected'),
                        log.replace('released ticket=3 director', 'released ticket=1 director'),
                        log+' ERROR backend failed\n'):
                self.assertFalse(all(validate(bad, 0, capture, persistent_cancel=True).values()))
            lines = log.splitlines()
            a = next(i for i, line in enumerate(lines) if line.startswith('showcase_persistent_cancel'))
            lines[a], lines[a+1] = lines[a+1], lines[a]
            self.assertFalse(all(validate('\n'.join(lines), 0, capture, persistent_cancel=True).values()))

    def test_full_run_rejects_truncated_chains_missing_audio_and_invented_clock(self):
        log = 'silent_capture=true audio_plugin=true audio_clock=true global_volume=0 muted_transport=true\n'
        for n, ticket in enumerate((1, 3), 1):
            log += f'mission_opening_source ticket={ticket} directors=3 voice_commands=1 sync_commands=1 source=selected_mission\n'
            log += f'mission_text_mounted ticket={ticket} fresh_atlas=true\n'
            log += 'cinematic_voice_play=line tick=1\ncinematic_voice_sink=line active=true volume_bits=0\ncinematic_voice_finished=line position_seconds=1\n'
            log += 'cinematic_master_play=track tick=1\ncinematic_master_sink=track active=true entity_bits=1 volume_bits=0\n'
            log += f'showcase_film_terminal ticket={ticket} directors=3 sequence=last tick=20 source_terminal=true voices_drained=true input_injected=false fidelity_verified=false\n'
            log += f'mission_text_released ticket={ticket} entities=4 atlas_images=2 remaining_assets=0 font_and_language_dropped=true\n'
            log += f'showcase_film_endpoint ticket={ticket} phase=Gameplay mecc_preview=true free_inspection=false campaign_ready_claimed=false\n'
            log += f'showcase_opening_cycle_released cycles={n} remaining_entities=0 movie_released=true input_injected=false\n'
        log += 'showcase_replay_cancelled remaining_entities=0 movie_released=true normal_ready_simulated=false\ncapture_ready frame=30\n'
        with tempfile.TemporaryDirectory() as directory:
            capture = Path(directory)/'menu.png'
            capture.write_bytes(b'\x89PNG\r\n\x1a\nsynthetic')
            self.assertTrue(all(validate(log, 0, capture, full=True, require_text=True).values()))
            # A sink can finish between the audio system and retirement in the
            # same frame; an observed empty owned sink is required in that case.
            retirement = log.replace('cinematic_voice_finished=line position_seconds=1\n', '', 1)
            retirement = retirement.replace('mission_text_released ticket=1',
                'cinematic_voice_released name=line ticket=1 reported_end=false sink_present=true sink_empty=true\nmission_text_released ticket=1')
            self.assertTrue(all(validate(retirement, 0, capture, full=True, require_text=True).values()))
            self.assertFalse(all(validate(retirement.replace('sink_empty=true', 'sink_empty=false'),
                                                0, capture, full=True, require_text=True).values()))
            for bad in (
                log.replace('terminal ticket=3 directors=3', 'terminal ticket=3 directors=2'),
                log.replace('voice_commands=1', 'voice_commands=2'),
                log.replace('voice_finished=line', 'voice_missing=line'),
                log.replace('volume_bits=0', 'volume_bits=1065353216'),
                log.replace('atlas_images=2', 'atlas_images=0'),
                log.replace('entities=4', 'entities=1'),
                log.replace('cinematic_voice_finished=line position_seconds=1\n', '', 1)
                    + 'cinematic_voice_finished=line position_seconds=1\n',
                log+'cinematic_preview_audio_tail=true\n',
                log+' ERROR giants_app: playback failed\n',
            ):
                self.assertFalse(all(validate(bad, 0, capture, full=True, require_text=True).values()))

    def test_endpoint_cannot_hide_wrong_ticket_cleanup_or_missing_cycle(self):
        log = 'silent_capture=true audio_plugin=false audio_clock=false\n'
        for n, ticket in enumerate((1, 3, 5), 1):
            log += f'mission_text_mounted ticket={ticket} fresh_atlas=true\n'
            log += f'mission_opening_audio_policy ticket={ticket} playback_enabled=false queued_players=0\n'
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
                log.replace('playback_enabled=false', 'playback_enabled=true'),
                log.replace('queued_players=0', 'queued_players=1'),
                log.replace('audio_policy ticket=3', 'audio_policy ticket=1'),
                log+'cinematic_skip_probe input=escape simulated=true\n',
                log+'panicked at test',
            ):
                self.assertFalse(all(validate(bad, 0, capture).values()))
            self.assertFalse(all(validate(log, 'timeout', capture).values()))
            capture.unlink()
            self.assertFalse(all(validate(log, 0, capture).values()))
