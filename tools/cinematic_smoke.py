"""Receipt-bound silent selected-mission opening/cancellation cycles.

The optional full mode follows natural endings with real sinks at zero volume.
Neither mode certifies skip input, audio mix, campaign rules or visual fidelity.
Profiles, logs and captures belong in a fresh private directory.
"""
import argparse
from collections import Counter
import json
import math
import os
from pathlib import Path
import re
import subprocess

from tools.package_preview import sha256


def validate(log, exit_code, capture, full=False, require_text=False):
    cycles = 2 if full else 3
    mounts = re.findall(r'^mission_text_mounted ticket=(\d+) ', log, re.MULTILINE)
    releases = re.findall(r'^mission_text_released ticket=(\d+) .*remaining_assets=0 ', log, re.MULTILINE)
    checks = {
        'process_exited_successfully': exit_code == 0,
        'audio_policy': ('silent_capture=true audio_plugin=true audio_clock=true global_volume=0 muted_transport=true'
                         if full else 'silent_capture=true audio_plugin=false audio_clock=false') in log,
        'fresh_text_owners_released': len(mounts) == cycles and len(set(mounts)) == cycles and mounts == releases,
        'real_cancellations': all(
            f'showcase_opening_cycle_released cycles={n} remaining_entities=0 movie_released=true input_injected=false' in log
            for n in range(1, cycles + 1)),
        'endpoint': 'showcase_replay_cancelled remaining_entities=0 movie_released=true normal_ready_simulated=false' in log,
        'capture_ready': 'capture_ready frame=' in log,
        'capture_written': capture.is_file() and capture.read_bytes().startswith(b'\x89PNG\r\n\x1a\n'),
        'no_panic_or_injected_skip': 'panicked at' not in log and 'cinematic_skip_probe ' not in log,
    }
    if full:
        source_matches = list(re.finditer(r'^mission_opening_source ticket=(\d+) directors=(\d+) voice_commands=(\d+) sync_commands=(\d+) ', log, re.MULTILINE))
        sources = [s.groups() for s in source_matches]
        terminals = re.findall(r'^showcase_film_terminal ticket=(\d+) directors=(\d+) .*source_terminal=true voices_drained=true input_injected=false ', log, re.MULTILINE)
        endpoints = re.findall(r'^showcase_film_endpoint ticket=(\d+) phase=(\w+) mecc_preview=(true|false) free_inspection=(true|false) campaign_ready_claimed=false', log, re.MULTILINE)
        checks['source_terminals_match_prepared_chains'] = (
            len(sources) == cycles and [s[0] for s in sources] == mounts
            and [(s[0], s[1]) for s in sources] == terminals)
        checks['real_post_film_endpoints'] = (
            [e[0] for e in endpoints] == mounts and len(endpoints) == cycles
            and all(e[2] == 'true' or e[3] == 'true' for e in endpoints))
        for kind, opcode_index in [('voice', 2), ('master', 3)]:
            played = Counter(re.findall(r'cinematic_'+kind+r'_play=(\w+)', log))
            sinks = re.findall(r'cinematic_'+kind+r'_sink=(\w+) active=true .*?volume_bits=(\d+)', log)
            checks[kind+'_source_commands_and_silent_sinks'] = (
                len(sources) == cycles and sum(played.values()) == sum(int(s[opcode_index]) for s in sources)
                and played == Counter(s[0] for s in sinks) and all(s[1] == '0' for s in sinks))
            if kind == 'voice':
                finished = Counter(re.findall(r'cinematic_voice_finished=(\w+)', log))
                retired_empty = Counter(re.findall(
                    r'cinematic_voice_released name=(\w+) ticket=\d+ reported_end=false sink_present=true sink_empty=true', log))
                checks['voices_finished'] = played == finished + retired_empty
        # Do not let duplicated receipts from one entry conceal a missing voice
        # or sink in the other entry of the same source chain.
        cycle_audio = []
        for i, source in enumerate(source_matches):
            block = log[source.start():source_matches[i + 1].start() if i + 1 < len(source_matches) else len(log)]
            ticket, _, voice_count, sync_count = source.groups()
            for kind, count in [('voice', int(voice_count)), ('master', int(sync_count))]:
                played = Counter(re.findall(r'cinematic_'+kind+r'_play=(\w+)', block))
                sinks = Counter(re.findall(r'cinematic_'+kind+r'_sink=(\w+) active=true ', block))
                complete = sum(played.values()) == count and played == sinks
                if kind == 'voice':
                    finished = Counter(re.findall(r'cinematic_voice_finished=(\w+)', block))
                    retired = Counter(re.findall(
                        r'cinematic_voice_released name=(\w+) ticket='+ticket+r' reported_end=false sink_present=true sink_empty=true', block))
                    complete = complete and played == finished + retired
                cycle_audio.append(complete)
        checks['audio_receipts_match_each_entry'] = len(cycle_audio) == cycles * 2 and all(cycle_audio)
        checks['no_unverified_audio_tail'] = 'cinematic_preview_audio_tail=true' not in log
        checks['no_playback_failure'] = 'playback_failed' not in log and ' ERROR ' not in log
    else:
        policies = re.findall(r'^mission_opening_audio_policy ticket=(\d+) playback_enabled=false queued_players=0$', log, re.MULTILINE)
        checks['disabled_audio_queues_no_players'] = policies == mounts and len(policies) == cycles
    if require_text:
        text = re.findall(r'^mission_text_released ticket=(\d+) entities=(\d+) atlas_images=(\d+) remaining_assets=0 ', log, re.MULTILINE)
        checks['rendered_text_atlases_released'] = (
            [t[0] for t in text] == mounts and len(text) == cycles
            and all(int(t[1]) > 1 and int(t[2]) > 0 for t in text))
    return checks


def run(executable, receipt_path, data_root, output, world, mission, timeout, full=False, require_text=False):
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
    env.update(GIANTS_MISSION_REPLAY='showcase-film-cycles' if full else 'showcase-opening-cycles', GIANTS_AUDIO_MUTE='1',
               GIANTS_SHOWCASE_WORLD=world, GIANTS_SHOWCASE_MISSION=str(mission))
    if full:
        env['RUST_LOG'] = 'giants_app=info'
    output.mkdir(parents=True)
    capture, log_path = output/'menu.png', output/'run.log'
    command = [str(executable), '--data-root', str(data_root), '--mecc-menu',
               '--user-data-root', str(output/'profile'), '--offscreen',
               '--silent-capture', '--capture', str(capture)]
    if full:
        command.append('--muted-transport')
    with log_path.open('xb') as log:
        try:
            code = subprocess.run(command, env=env, stdout=log, stderr=subprocess.STDOUT,
                                  timeout=timeout, creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0)).returncode
        except subprocess.TimeoutExpired:
            code = 'timeout'
    checks = validate(log_path.read_text(encoding='utf8', errors='replace'), code, capture, full, require_text)
    report = dict(schema_version=1, source_commit=receipt['source_commit'],
                  executable_sha256=receipt['executable_sha256'], world=world, mission=mission,
                  scope=('Selected-mission partial-renderer source-terminal transport and re-entry' if full else
                         'Selected-mission partial opening and cancellation lifecycle only'),
                  source_terminal_transport_verified=full and all(checks.values()),
                  full_playback_verified=False, full_scene_verified=False,
                  native_timing_verified=False, audio_verified=False, fidelity_verified=False,
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
    parser.add_argument('--timeout', type=float, help='Seconds; default 240, or 1800 with --full')
    parser.add_argument('--full', action='store_true', help='Two source-terminal transport cycles with real sinks at zero volume')
    parser.add_argument('--require-text', action='store_true', help='Require nonempty rendered glyph pools and atlas cleanup')
    args = parser.parse_args()
    report = run(args.executable, args.receipt, args.data_root, args.output, args.world, args.mission,
                 args.timeout if args.timeout is not None else (1800 if args.full else 240), args.full, args.require_text)
    print(json.dumps({'passed': report['passed'], 'checks': report['checks']}))
    raise SystemExit(0 if report['passed'] else 1)
