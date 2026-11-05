"""Receipt-bound silent selected-mission opening/cancellation cycles.

This does not test skip input, full playback, audio, gameplay or visual fidelity.
Profiles, logs and captures belong in a fresh private directory.
"""
import argparse
import json
import math
import os
from pathlib import Path
import re
import subprocess

from tools.package_preview import sha256


def validate(log, exit_code, capture):
    mounts = re.findall(r'^mission_text_mounted ticket=(\d+) ', log, re.MULTILINE)
    releases = re.findall(r'^mission_text_released ticket=(\d+) .*remaining_assets=0 ', log, re.MULTILINE)
    return {
        'process_exited_successfully': exit_code == 0,
        'audio_device_disabled': 'silent_capture=true audio_plugin=false audio_clock=false' in log,
        'three_fresh_text_owners_released': len(mounts) == 3 and len(set(mounts)) == 3 and mounts == releases,
        'three_real_cancellations': all(
            f'showcase_opening_cycle_released cycles={n} remaining_entities=0 movie_released=true input_injected=false' in log
            for n in range(1, 4)),
        'endpoint': 'showcase_replay_cancelled remaining_entities=0 movie_released=true normal_ready_simulated=false' in log,
        'capture_ready': 'capture_ready frame=' in log,
        'capture_written': capture.is_file() and capture.read_bytes().startswith(b'\x89PNG\r\n\x1a\n'),
        'no_panic_or_injected_skip': 'panicked at' not in log and 'cinematic_skip_probe ' not in log,
    }


def run(executable, receipt_path, data_root, output, world, mission, timeout):
    executable, data_root, output = (Path(p).resolve() for p in (executable, data_root, output))
    if not math.isfinite(timeout) or timeout <= 0 or mission < 0:
        raise ValueError('Positive finite timeout and nonnegative mission index required')
    if not re.fullmatch(r'[A-Za-z0-9_]+', world):
        raise ValueError('Expected a catalog world identifier')
    if output.exists() or output.is_relative_to(data_root) or data_root.is_relative_to(output):
        raise ValueError('Output must be fresh and separate from the original installation')
    receipt = json.loads(Path(receipt_path).read_text(encoding='utf8'))
    if (not re.fullmatch(r'[a-f0-9]{40}', receipt.get('source_commit', ''))
            or receipt.get('target') != 'x86_64-pc-windows-gnu'
            or receipt.get('executable_sha256') != sha256(executable)):
        raise ValueError('Executable does not match a GNU Windows build receipt')
    if not (data_root/'Bin').is_dir() or not any((data_root/'Bin').glob('*.gzp')):
        raise ValueError('Data root must contain Bin/*.gzp')
    env = {k: v for k, v in os.environ.items() if not k.startswith('GIANTS_')}
    if 'GIANTS_INSPECTION_FONT' in os.environ:
        env['GIANTS_INSPECTION_FONT'] = os.environ['GIANTS_INSPECTION_FONT']
    env.update(GIANTS_MISSION_REPLAY='showcase-opening-cycles', GIANTS_AUDIO_MUTE='1',
               GIANTS_SHOWCASE_WORLD=world, GIANTS_SHOWCASE_MISSION=str(mission))
    output.mkdir(parents=True)
    capture, log_path = output/'menu.png', output/'run.log'
    command = [str(executable), '--data-root', str(data_root), '--mecc-menu',
               '--user-data-root', str(output/'profile'), '--offscreen',
               '--silent-capture', '--capture', str(capture)]
    with log_path.open('xb') as log:
        try:
            code = subprocess.run(command, env=env, stdout=log, stderr=subprocess.STDOUT,
                                  timeout=timeout, creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0)).returncode
        except subprocess.TimeoutExpired:
            code = 'timeout'
    checks = validate(log_path.read_text(encoding='utf8', errors='replace'), code, capture)
    report = dict(schema_version=1, source_commit=receipt['source_commit'],
                  executable_sha256=receipt['executable_sha256'], world=world, mission=mission,
                  scope='Selected-mission partial opening and cancellation lifecycle only',
                  full_playback_verified=False, audio_verified=False, fidelity_verified=False,
                  interactive_controls_verified=False, exit_code=code, checks=checks,
                  passed=all(checks.values()), log_sha256=sha256(log_path),
                  capture_sha256=sha256(capture) if capture.is_file() else None)
    (output/'report.json').write_text(json.dumps(report, indent=2)+'\n', encoding='utf8')
    return report


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('executable', 'receipt', 'data-root', 'output'):
        parser.add_argument('--'+name, type=Path, required=True)
    parser.add_argument('--world', default='Story1')
    parser.add_argument('--mission', type=int, default=0, help='Zero-based selected mission')
    parser.add_argument('--timeout', type=float, default=240)
    args = parser.parse_args()
    report = run(args.executable, args.receipt, args.data_root, args.output, args.world, args.mission, args.timeout)
    print(json.dumps({'passed': report['passed'], 'checks': report['checks']}))
    raise SystemExit(0 if report['passed'] else 1)
