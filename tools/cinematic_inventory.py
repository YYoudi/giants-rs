"""Batch the real CPU scene preparers; keep per-source diagnostics private.

This detects reported capability gaps, not all visual errors. No App, window,
GPU, input or audio is created. The public projection contains only fixed labels
and aggregate counts; original identifiers, messages and research stay private.
"""
import argparse
import csv
import hashlib
import html
import json
from pathlib import Path
import re
import subprocess


# Priority: preparation barriers, missing consumers, partial services, review.
# Within each level, fix the cause affecting the most distinct sources first.
CAUSES = {
    'preparation_failure': (0, 'Preparation failure', 'reported_refusal'),
    'stream_missing': (0, 'Voice or sync file unavailable', 'missing_resource'),
    'camera_refused': (1, 'Camera binding refused', 'reported_refusal'),
    'body_part_refused': (1, 'Body material or part refused', 'reported_refusal'),
    'rigid_refused': (1, 'Attachment or rigid part refused', 'reported_refusal'),
    'effect_refused': (3, 'Effect preview refusal needs request-level review', 'review_needed'),
    'main_fx_binding': (1, 'Mission FX binding refused (preview may still render)', 'reported_refusal'),
    'fx_actor_prefix': (1, 'Actor mutations before FX attachment are not admitted', 'reported_refusal'),
    'fx_queue_profile': (1, 'General FX queue exceeds the terminal-only profile', 'reported_refusal'),
    'numeric_audio': (1, 'Numeric Flick sound commands lack a live consumer', 'missing_consumer'),
    'unclassified_flk_review': (3, 'FLK command 28/30 needs consumer review', 'review_needed'),
    'credits': (1, 'Cinematic credits commands lack a live consumer', 'missing_consumer'),
    'effect_partial': (2, 'Effect preview retains incomplete services', 'partial_implementation'),
    'effect_resume': (2, 'External effect resume source unresolved', 'partial_implementation'),
    'strict_profile_review': (3, 'Strict FLK profile requires preview handling', 'review_needed'),
    'mixed_objects': (2, 'Mixed effect objects deferred by preparation', 'reported_refusal'),
    'fx_audio_review': (3, 'Prepared FX audio needs live-consumer verification', 'review_needed'),
    'tracer_review': (3, 'Prepared tracer needs live-consumer verification', 'review_needed'),
    'other_diagnostic': (3, 'Other consumer diagnostic requires review', 'review_needed'),
}
UNVERIFIED = ['complete_visual_fidelity', 'native_timing', 'complete_functional_playback',
              'lighting_shadows_water', 'audio_mix_and_subtitle_parity', 'MDV_music_commands_not_audited']
CASE = re.compile(r'^cinematic_consumer_audit_case world=(\w+) sequence=(\w+)$')
FIELDS = re.compile(r'(\w+)=([^ ]+)')


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def manifest(path, root, expected):
    cases = []
    for line in Path(path).read_text(encoding='utf8').splitlines():
        values = line.split('\t')
        if len(values) != 2 or not all(re.fullmatch(r'[A-Za-z0-9_]+', s) for s in values):
            raise ValueError('Expected private world/sequence TSV without a header')
        cases.append(tuple(values))
    keys = [(w.lower(), s.lower()) for w, s in cases]
    if len(keys) != len(set(keys)) or len(keys) != expected:
        raise ValueError('Duplicate cases or unexpected source count')
    installed = {p.stem[4:].lower() for p in (Path(root)/'Bin').iterdir()
                 if p.is_file() and p.name.lower().startswith('flk_') and p.suffix.lower() == '.bin'}
    if {s.lower() for _, s in cases} != installed:
        raise ValueError('Case inventory does not cover the installed FLK directory exactly')
    return cases


def parse(log, result_tsv, expected_cases):
    """Count final-consumer diagnostics, never legacy route omissions as losses."""
    records = {}
    current = None
    if 'schema_version=2 cpu_only=true app_created=false gpu_initialized=false audio_initialized=false' not in log:
        raise ValueError('Missing CPU-only versioned audit guard')
    for line in log.splitlines():
        match = CASE.fullmatch(line)
        if match:
            key = tuple(s.lower() for s in match.groups())
            if key in records:
                raise ValueError('Repeated source marker')
            current = dict(world=match[1], sequence=match[2], declarations=None,
                           commands={}, findings=[], routed_elsewhere=0,
                           stream_resources=None, mission_fx_binding=None,
                           persistent_fx_prepared_requests=0,
                           terminal_fx_preview_rejections=0, unverified=UNVERIFIED.copy(),
                           _seen=set())
            records[key] = current
            continue
        if current is None:
            continue
        if line in current['_seen']:
            continue
        current['_seen'].add(line)
        event = line.split(' ', 1)[0]
        fields = dict(FIELDS.findall(line))

        def add(cause, count=1):
            if count > 0:
                current['findings'].append(dict(cause=cause, count=count, diagnostic=line))

        if event == 'cinematic_audit_declarations':
            if current['declarations'] is not None:
                raise ValueError('Repeated declaration summary')
            current['declarations'] = {k: int(v) if v.isdigit() else v for k, v in fields.items()}
            for field, cause in [('numeric_sound_requests', 'numeric_audio'), ('credits_commands', 'credits')]:
                add(cause, int(fields[field]))
            # Schema-2 producer's legacy music_commands label counts FLK
            # 28/30, not the distinct MDV music opcodes. Never infer missing
            # music from their coincident numeric values.
            add('unclassified_flk_review', int(fields.get('unclassified_commands_28_30', fields.get('music_commands', 0))))
            # The legacy metadata field named opaque_blocks is the raw effect
            # slot capacity. It is not evidence of unknown executable content.
            # Strict-parser rejection also does not prove a missing live draw:
            # this audit deliberately invokes the actual preview consumers.
            if fields['runtime_profile_supported'] != 'true':
                add('strict_profile_review')
        elif event == 'cinematic_audit_command':
            current['commands'][fields['opcode']] = int(fields['count'])
        elif event == 'cinematic_audit_stream_resources':
            current['stream_resources'] = {k: int(fields[k]) for k in ['checked', 'missing']}
        elif event == 'cinematic_audit_stream_missing':
            add('stream_missing')
        elif event == 'cinematic_audit_mission_effect_binding':
            current['mission_fx_binding'] = fields['result']
            if fields['result'] == 'refused':
                add('fx_actor_prefix' if 'effect attachment prefix requires unimplemented actor mutations' in line
                    else 'fx_queue_profile' if 'live cinematic effect outside measured mode1/FX02 profile' in line
                    else 'main_fx_binding')
        elif event == 'cinematic_audit_persistent_effect_binding':
            if (fields.get('result') != 'prepared' or fields.get('readiness_acknowledged') != 'false'
                    or fields.get('live_execution_verified') != 'false'):
                raise ValueError('Persistent preparation cannot acknowledge live execution')
            current['persistent_fx_prepared_requests'] = int(fields['requests'])
        elif event == 'cinematic_rigid_attachment_deferred' and fields.get('reason') == 'existing_specialized_consumer':
            current['routed_elsewhere'] += 1
        elif event in {'cinematic_preview_omission', 'cinematic_common_attachment_request'}:
            # These are emitted before the final specialized/common ownership
            # decision. Neither establishes an absent draw in the main build.
            pass
        elif event == 'cinematic_part_request' and 'result=Err(' in line:
            add('body_part_refused')
        elif event == 'cinematic_camera_request' and 'result=Err(' in line:
            add('camera_refused')
        elif event in {'cinematic_rigid_attachment_deferred', 'cinematic_rigid_part_deferred',
                       'cinematic_specialized_consumer_deferred', 'cinematic_attachment_loop_deferred'}:
            add('rigid_refused')
        elif event == 'cinematic_effect_inspection_deferred':
            add('effect_partial' if 'visual objects prepared' in line or 'visual preview prepared' in line
                else 'effect_refused')
        elif event == 'cinematic_mixed_effect_prepared':
            add('mixed_objects', int(fields.get('deferred_objects', 0)))
        elif event == 'cinematic_context_resume_pending':
            add('effect_resume')
        elif event == 'cinematic_mixed_context_sounds_prepared':
            add('fx_audio_review', int(fields['count']))
        elif event == 'cinematic_emitted_tracer_pending':
            # A preparation-time pending label alone is not proof that the live
            # tracer updater is missing; keep it in the review queue.
            add('tracer_review')
        elif any(word in event for word in ['deferred', 'pending', 'failed', 'refused']):
            add('other_diagnostic')
    if 'cinematic_consumer_audit finished=true gpu_initialized=false audio_initialized=false scene_complete=false' not in log:
        raise ValueError('Truncated audit: missing terminal marker')
    expected = {(w.lower(), s.lower()) for w, s in expected_cases}
    if set(records) != expected:
        raise ValueError('Observed cases do not match the inventory')
    results = {}
    for line in result_tsv.splitlines():
        status, world, sequence, stage, detail = line.split('\t', 4)
        key = (world.lower(), sequence.lower())
        if key not in records or key in results or status not in {'prepared', 'error'}:
            raise ValueError('Invalid or repeated preparation result')
        results[key] = status
        row = records[key]
        row['preparation'] = status
        row['failed_stage'] = stage if status == 'error' else None
        if status == 'error':
            row['findings'].append(dict(cause='preparation_failure', count=1, diagnostic=detail))
        elif row['declarations'] is None or row['stream_resources'] is None:
            raise ValueError('Prepared source lacks its declaration/resource audit')
    if set(results) != expected:
        raise ValueError('Missing preparation results')
    for row in records.values():
        del row['_seen']
        if row['mission_fx_binding'] == 'bound':
            # Bindings::load accepts only inactive or terminator-only programs.
            # An empty visual-preview phase is expected for such a request;
            # the real mission queue owns it without needing particle geometry.
            row['terminal_fx_preview_rejections'] = sum(f['count'] for f in row['findings'] if f['cause'] == 'effect_refused')
            row['findings'] = [f for f in row['findings'] if f['cause'] != 'effect_refused']
        row['state'] = 'reported_gap' if any(CAUSES[f['cause']][0] < 3 for f in row['findings']) else 'no_reported_gap_unverified'
    return list(records.values())


def aggregate(records):
    grouped = {}
    for cause, (priority, label, kind) in CAUSES.items():
        affected = [r for r in records if any(f['cause'] == cause for f in r['findings'])]
        if affected:
            grouped[cause] = dict(cause=cause, priority=priority, label=label, evidence_kind=kind,
                affected_sources=len(affected), affected_worlds=len({r['world'] for r in affected}),
                diagnostic_units=sum(f['count'] for r in affected for f in r['findings'] if f['cause'] == cause))
    return sorted(grouped.values(), key=lambda r: (r['priority'], -r['affected_sources'], -r['diagnostic_units'], r['cause']))


def public_summary(records, identity):
    # Explicit projection: no source names, reasons, paths, scripts or raw fields.
    return dict(schema_version=1, scope='CPU preparation and declared-consumer gaps only',
        source_commit=identity['source_commit'], executable_sha256=identity['executable_sha256'],
        sources=len(records), worlds=len({r['world'] for r in records}),
        preparation_errors=sum(r['preparation'] == 'error' for r in records),
        sources_with_reported_gaps=sum(r['state'] == 'reported_gap' for r in records),
        sources_requiring_review=sum(any(CAUSES[f['cause']][0] == 3 for f in r['findings']) for r in records),
        complete_scene_fidelity_verified=0, visual_fidelity_checked=False,
        priority_rule='Preparation barriers first; then distinct affected sources within each priority level.',
        priorities=aggregate(records), unverified=UNVERIFIED.copy())


def write_reports(output, records, identity):
    summary = public_summary(records, identity)
    report = dict(identity, schema_version=1, batch_completed=True,
                  full_scene_fidelity_verified=False, sources=records, priorities=summary['priorities'])
    (output/'inventory.json').write_text(json.dumps(report, indent=2)+'\n', encoding='utf8')
    (output/'public-summary.json').write_text(json.dumps(summary, indent=2)+'\n', encoding='utf8')
    with (output/'sources.tsv').open('w', encoding='utf8', newline='') as file:
        writer = csv.writer(file, delimiter='\t')
        writer.writerow(['world', 'sequence', 'preparation', 'state', 'causes', 'visual_fidelity'])
        for row in records:
            writer.writerow([row['world'], row['sequence'], row['preparation'], row['state'],
                ', '.join(sorted({f['cause'] for f in row['findings']})), 'unverified'])
    esc = html.escape
    priorities = ''.join(f'<tr><td>{esc(r["label"])}</td><td>{r["affected_sources"]}</td><td>{r["affected_worlds"]}</td><td>{esc(r["evidence_kind"])}</td></tr>' for r in summary['priorities'])
    rows = []
    for row in sorted(records, key=lambda r: (-len({f['cause'] for f in r['findings']}), r['world'], r['sequence'])):
        findings = ''.join(f'<li>{esc(CAUSES[f["cause"]][1])}: {f["count"]}<details><summary>Diagnostic</summary><pre>{esc(f["diagnostic"])}</pre></details></li>' for f in row['findings'])
        rows.append(f'<tr class="{row["state"]}"><td>{esc(row["world"])}</td><th>{esc(row["sequence"])}</th><td>{esc(row["preparation"])}</td><td><ul>{findings}</ul></td><td>Unverified</td></tr>')
    page = f'''<!doctype html><html lang="en"><meta charset="utf-8"><title>Private Flick inventory</title>
<style>body{{font:15px system-ui;background:#111827;color:#e5e7eb;margin:2em}}table{{border-collapse:collapse;width:100%;margin:1em 0}}td,th{{padding:.6em;border:1px solid #475569;text-align:left;vertical-align:top}}.reported_gap{{background:#3d261f}}.no_reported_gap_unverified{{background:#1e293b}}pre{{white-space:pre-wrap;overflow-wrap:anywhere}}li{{margin-bottom:.5em}}</style>
<h1>Private Flick inventory</h1><p>{len(records)} sources. CPU preparation and declared-consumer diagnostics. Zero scenes certified faithful. Orange marks reported gaps; slate means no reported gap, with fidelity still unverified. No completeness percentage.</p>
<p>Priorities group common causes by preparation barrier and number of distinct affected sources. Diagnostic units overlap and are not counts of missing rendered objects. Full raw evidence remains in inventory.json and consumers.log.</p>
<table><caption>Common causes to prioritize</caption><tr><th>Cause</th><th>Sources</th><th>Worlds</th><th>Evidence</th></tr>{priorities}</table>
<table><caption>Every source</caption><tr><th>World</th><th>Flick</th><th>CPU preparation</th><th>Known diagnostics</th><th>Complete fidelity</th></tr>{''.join(rows)}</table></html>'''
    (output/'index.html').write_text(page, encoding='utf8')
    return summary


def run(executable, receipt, root, cases_path, output, expected=545, timeout=3600):
    executable, root, output = Path(executable).resolve(), Path(root).resolve(), Path(output).resolve()
    if output.is_relative_to(root):
        raise ValueError('Output must stay outside the read-only installation')
    identity = json.loads(Path(receipt).read_text(encoding='utf8'))
    if digest(executable) != identity['executable_sha256'] or not re.fullmatch(r'[a-f0-9]{40}', identity['source_commit']):
        raise ValueError('Executable does not match its build receipt')
    cases = manifest(cases_path, root, expected)
    output.mkdir(parents=True, exist_ok=False)
    identity = {k: identity[k] for k in ['source_commit', 'executable_sha256', 'target', 'toolchain', 'linker']}
    identity.update(cases_sha256=digest(cases_path), runner_sha256=digest(__file__))
    result_path = output/'preparation.tsv'
    command = [str(executable), '--data-root', str(root), '--audit-cinematic-consumers',
               str(Path(cases_path).resolve()), '--audit-output', str(result_path)]
    with (output/'consumers.log').open('xb') as log:
        result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, timeout=timeout,
                                creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
    if result.returncode:
        raise RuntimeError(f'CPU audit exited with {result.returncode}; retain the private log')
    log_path = output/'consumers.log'
    records = parse(log_path.read_text(encoding='utf8', errors='replace'),
                    result_path.read_text(encoding='utf8'), cases)
    identity.update(log_sha256=digest(log_path), preparation_sha256=digest(result_path))
    return write_reports(output, records, identity)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['executable', 'receipt', 'data-root', 'cases', 'output']:
        parser.add_argument('--'+name, type=Path, required=True)
    parser.add_argument('--expected-sources', type=int, default=545)
    parser.add_argument('--timeout', type=float, default=3600)
    args = parser.parse_args()
    result = run(args.executable, args.receipt, args.data_root, args.cases, args.output,
                 args.expected_sources, args.timeout)
    print(json.dumps(result, indent=2))
