"""Run silent offscreen development-session cycles with user-supplied game data.

Logs, user profiles and captures stay in a fresh private output directory. This
checks runtime lifecycle, not campaign completion, manual controls or fidelity.
"""
import argparse
import json
import math
import os
from pathlib import Path
import re
import subprocess

from tools.package_preview import sha256


MODES = {'session-cycles': (3, 'frontend'),
         'session-cycles-play': (2, 'gameplay')}


def validate(log, mode, exit_code, capture):
    returns, endpoint = MODES[mode]
    checks = {
        'process_exited_successfully': exit_code == 0,
        'silent_audio_device_disabled': 'silent_capture=true audio_plugin=false audio_clock=false' in log,
        'three_mounts': log.count('mecc_session_mounted ') == 3,
        'three_fresh_launches': log.count('mecc_session_launch fresh_health=100 projectiles=0 ') == 3,
        'expected_returns': log.count('mecc_session_return_menu ') == returns,
        'cycle_endpoint': (
            f'mecc_session_cycles_finished passed=true mounts=3 returns={returns} '
            f'endpoint={endpoint} owner_validation=false') in log,
        'capture_ready': 'capture_ready frame=' in log,
        'capture_written': capture.is_file() and capture.read_bytes().startswith(b'\x89PNG\r\n\x1a\n'),
        'no_panic': 'panicked at' not in log,
    }
    releases = re.findall(r'^mecc_session_return_menu .*$', log, re.MULTILINE)
    checks['returned_entities_released'] = len(releases) == returns and all(
        'remaining_owned=0 campaign_ack=false' in line for line in releases)
    return checks


def run(executable, receipt_path, data_root, output, timeout=120):
    executable, data_root, output = (Path(p).resolve() for p in (executable, data_root, output))
    if not math.isfinite(timeout) or timeout <= 0:
        raise ValueError('Timeout must be finite and positive')
    if output.is_relative_to(data_root) or data_root.is_relative_to(output):
        raise ValueError('Output must be separate from the original installation')
    if output.exists():
        raise ValueError('Refusing to overwrite smoke output')
    receipt = json.loads(Path(receipt_path).read_text(encoding='utf8'))
    if (not re.fullmatch(r'[a-f0-9]{40}', receipt.get('source_commit', '')) or
            receipt.get('target') != 'x86_64-pc-windows-gnu' or
            receipt.get('executable_sha256') != sha256(executable)):
        raise ValueError('Executable does not match a GNU Windows build receipt')
    if not (data_root/'Bin').is_dir() or not any((data_root/'Bin').glob('*.gzp')):
        raise ValueError('Data root must contain Bin/*.gzp')
    # Inherited replay/debug overrides must not turn this into a different test.
    env = {k: v for k, v in os.environ.items() if not k.startswith('GIANTS_')}
    if 'GIANTS_INSPECTION_FONT' in os.environ:
        env['GIANTS_INSPECTION_FONT'] = os.environ['GIANTS_INSPECTION_FONT']
    env['GIANTS_AUDIO_MUTE'] = '1'
    output.mkdir(parents=True)
    report = dict(schema_version=1, source_commit=receipt['source_commit'],
                  executable_sha256=receipt['executable_sha256'],
                  scope='Offscreen development lifecycle only',
                  manual_controls_verified=False, fidelity_verified=False, runs=[])
    for mode in MODES:
        env['GIANTS_MECC_DEV_REPLAY'] = mode
        capture = output/(mode+'.png')
        log_path = output/(mode+'.log')
        command = [str(executable), '--data-root', str(data_root), '--mecc-menu',
                   '--user-data-root', str(output/('user-'+mode)), '--offscreen',
                   '--silent-capture', '--capture', str(capture)]
        with log_path.open('xb') as log:
            try:
                result = subprocess.run(command, env=env, stdout=log, stderr=subprocess.STDOUT,
                                        timeout=timeout,
                                        creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
                exit_code = result.returncode
            except subprocess.TimeoutExpired:
                exit_code = 'timeout'
            except OSError:
                exit_code = 'launch_failed'
        checks = validate(log_path.read_text(encoding='utf8', errors='replace'), mode, exit_code, capture)
        report['runs'].append(dict(mode=mode, exit_code=exit_code, checks=checks,
                                   passed=all(checks.values()), log_sha256=sha256(log_path),
                                   capture_sha256=sha256(capture) if capture.is_file() else None))
        report['passed'] = len(report['runs']) == len(MODES) and all(r['passed'] for r in report['runs'])
        (output/'report.json').write_text(json.dumps(report, indent=2)+'\n', encoding='utf8')
        if not all(checks.values()):
            break
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--executable', type=Path, required=True)
    parser.add_argument('--receipt', type=Path, required=True)
    parser.add_argument('--data-root', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--timeout', type=float, default=120, help='Seconds per replay process')
    args = parser.parse_args()
    report = run(args.executable, args.receipt, args.data_root, args.output, args.timeout)
    print(json.dumps(report))
    if not report['passed']:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
