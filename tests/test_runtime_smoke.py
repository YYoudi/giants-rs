import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from tools.package_preview import sha256
from tools.runtime_smoke import MODES, run, validate


def successful_log(mode):
    returns, endpoint = MODES[mode]
    return ('silent_capture=true audio_plugin=false audio_clock=false\n' +
            'mecc_session_mounted entities=10 assets=20 campaign_ready=false\n'*3 +
            'mecc_session_launch fresh_health=100 projectiles=0 launch=development\n'*3 +
            'mecc_session_return_menu entities=10 assets=20 remaining_owned=0 campaign_ack=false\n'*returns +
            f'mecc_session_cycles_finished passed=true mounts=3 returns={returns} endpoint={endpoint} owner_validation=false\n' +
            'capture_ready frame=30 elapsed_seconds=6\n')


class RuntimeSmokeTests(unittest.TestCase):
    def test_success_runs_both_modes_silently_with_isolated_profiles(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            exe = root/'giants-app.exe'
            exe.write_bytes(b'MZ synthetic')
            data = root/'game'
            (data/'Bin').mkdir(parents=True)
            (data/'Bin/test.gzp').write_bytes(b'synthetic')
            receipt = root/'receipt.json'
            receipt.write_text(json.dumps(dict(source_commit='a'*40,
                               target='x86_64-pc-windows-gnu', executable_sha256=sha256(exe))))
            profiles = []

            def replay(command, **kwargs):
                env = kwargs['env']
                mode = env['GIANTS_MECC_DEV_REPLAY']
                self.assertEqual(env['GIANTS_AUDIO_MUTE'], '1')
                self.assertNotIn('GIANTS_MISSION_REPLAY', env)
                self.assertIn('--silent-capture', command)
                self.assertIn('--offscreen', command)
                profiles.append(command[command.index('--user-data-root')+1])
                kwargs['stdout'].write(successful_log(mode).encode())
                Path(command[command.index('--capture')+1]).write_bytes(b'\x89PNG\r\n\x1a\nsynthetic')
                return subprocess.CompletedProcess(command, 0)

            with patch.dict(os.environ, {'GIANTS_MISSION_REPLAY': 'unrelated'}), \
                    patch('tools.runtime_smoke.subprocess.run', side_effect=replay):
                report = run(exe, receipt, data, root/'output')
            self.assertTrue(report['passed'])
            self.assertEqual(len(set(profiles)), 2)
            self.assertFalse(report['manual_controls_verified'])
            self.assertFalse(report['fidelity_verified'])
            self.assertNotIn(str(root), json.dumps(report))
            for timeout in (-1, float('nan'), float('inf')):
                with self.assertRaisesRegex(ValueError, 'finite and positive'):
                    run(exe, receipt, data, root/'unused', timeout)

    def test_endpoint_marker_alone_cannot_hide_failed_or_incomplete_run(self):
        with tempfile.TemporaryDirectory() as directory:
            capture = Path(directory)/'capture.png'
            capture.write_bytes(b'\x89PNG\r\n\x1a\nsynthetic')
            for mode in MODES:
                log = successful_log(mode)
                self.assertTrue(all(validate(log, mode, 0, capture).values()))
                for bad in (log.replace('mecc_session_mounted ', 'missing '),
                            log.replace('remaining_owned=0', 'remaining_owned=1'),
                            log.replace('fresh_health=100', 'fresh_health=64'),
                            log.replace('capture_ready frame=', 'not_ready='),
                            log+'panicked at test'):
                    self.assertFalse(all(validate(bad, mode, 0, capture).values()))
                self.assertFalse(all(validate(log, mode, 1, capture).values()))
                capture.write_bytes(b'not a capture')
                self.assertFalse(all(validate(log, mode, 0, capture).values()))
                capture.write_bytes(b'\x89PNG\r\n\x1a\nsynthetic')

    def test_timeout_is_recorded_and_remaining_replays_are_not_launched(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            exe = root/'giants-app.exe'
            exe.write_bytes(b'MZ synthetic')
            data = root/'game'
            (data/'Bin').mkdir(parents=True)
            (data/'Bin/test.gzp').write_bytes(b'synthetic')
            receipt = root/'receipt.json'
            receipt.write_text(json.dumps(dict(source_commit='a'*40,
                               target='x86_64-pc-windows-gnu', executable_sha256=sha256(exe))))
            with patch('tools.runtime_smoke.subprocess.run',
                       side_effect=subprocess.TimeoutExpired('synthetic', 1)) as launch:
                report = run(exe, receipt, data, root/'output', timeout=1)
                self.assertEqual(launch.call_count, 1)
                self.assertFalse(report['passed'])
                self.assertEqual(report['runs'][0]['exit_code'], 'timeout')
                self.assertEqual(json.loads((root/'output/report.json').read_text()), report)
                self.assertNotIn(str(root), json.dumps(report))
                for output in (data, data/'smoke', root, root/'output'):
                    with self.assertRaises(ValueError):
                        run(exe, receipt, data, output)
                exe.write_bytes(b'MZ changed')
                with self.assertRaisesRegex(ValueError, 'build receipt'):
                    run(exe, receipt, data, root/'new-output')
                self.assertFalse((root/'new-output').exists())


if __name__ == '__main__':
    unittest.main()
