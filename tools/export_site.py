"""Export the public roadmap and owner-authorized comparison gallery."""
import argparse
import hashlib
import html
import json
from pathlib import Path
import re
import shutil
from tools.cinematic_inventory import CAUSES, UNVERIFIED

ROOT = Path(__file__).resolve().parents[1]
TEMPLATES = Path(__file__).resolve().parent / 'site'
FIELDS = ('id', 'kind', 'group', 'title', 'status', 'scope', 'detail')
LABELS = {'done': 'Complete in stated scope', 'partial': 'Partial', 'todo': 'Not started'}
PRIVATE = re.compile(r'[A-Za-z]:[\\/]|(?:evidence|\.local)[\\/]|Users[\\/]', re.I)


def project(source):
    rows = []
    for original in source['readme_roadmap']:
        row = {key: str(original[key]) for key in FIELDS if key in original}
        if row['status'] not in LABELS or row['kind'] not in ('feature', 'enabler'):
            raise ValueError('Unknown roadmap state')
        if any(PRIVATE.search(value) for value in row.values()):
            raise ValueError('Private path in public roadmap field')
        rows.append(row)
    return {'project': 'giants-rs', 'overall_fidelity_verified': source.get('overall_fidelity_verified') is True, 'roadmap': rows}


def count(rows):
    return {s: sum(r['status'] == s for r in rows) for s in LABELS}


def table(rows, caption):
    esc = html.escape
    body = []
    for group in dict.fromkeys(r['group'] for r in rows):
        body.append(f'<tbody class="roadmap-group"><tr class="group-heading"><th colspan="3" scope="colgroup">{esc(group)}</th></tr>')
        for row in (r for r in rows if r['group'] == group):
            body.append(f'<tr class="entry" data-status="{row["status"]}"><th scope="row">{esc(row["title"])}'
                        f'<span class="scope">{esc(row["scope"])}</span></th><td><span class="badge {row["status"]}">'
                        f'{LABELS[row["status"]]}</span></td><td>{esc(row.get("detail", "—"))}</td></tr>')
        body.append('</tbody>')
    return f'<div class="table-scroll" role="region" aria-label="{caption}" tabindex="0"><table class="roadmap"><caption>{caption}</caption><thead><tr><th scope="col">Feature / scope</th><th scope="col">Status</th><th scope="col">Remaining scope / evidence limits</th></tr></thead>{"".join(body)}</table></div>'


def gallery_data(root):
    path = root/'docs/gallery/manifest.json'
    if not path.exists():
        return {'comparisons': [], 'images': {}}
    data = json.loads(path.read_text(encoding='utf8'))
    fields = {'title', 'level', 'event', 'commit', 'left', 'right', 'left_label', 'right_label', 'verification', 'alignment'}
    for row in data['comparisons']:
        if set(row) - fields:
            raise ValueError('Unexpected gallery field')
        if any(not isinstance(v, str) or PRIVATE.search(v) for v in row.values()):
            raise ValueError('Private path in gallery')
        for side in ('left', 'right'):
            name = row[side]
            if not re.fullmatch(r'media/[a-f0-9]{64}\.webp', name) or name not in data['images']:
                raise ValueError('Unapproved gallery image')
    for name, digest in data['images'].items():
        if not re.fullmatch(r'media/[a-f0-9]{64}\.webp', name):
            raise ValueError('Invalid image path')
        if hashlib.sha256((path.parent/name).read_bytes()).hexdigest() != digest or Path(name).stem != digest:
            raise ValueError('Gallery image hash mismatch')
    return data


def cinematic_inventory(root):
    path = root/'docs/cinematic-inventory-summary.json'
    if not path.exists():
        return None, ''
    data = json.loads(path.read_text(encoding='utf8'))
    fields = {'schema_version', 'scope', 'source_commit', 'executable_sha256', 'sources', 'worlds',
              'preparation_errors', 'sources_with_reported_gaps', 'sources_requiring_review', 'complete_scene_fidelity_verified',
              'visual_fidelity_checked', 'priority_rule', 'priorities', 'unverified'}
    if set(data) != fields or data['schema_version'] != 1:
        raise ValueError('Unexpected cinematic inventory fields')
    if not re.fullmatch(r'[a-f0-9]{40}', data['source_commit']) or not re.fullmatch(r'[a-f0-9]{64}', data['executable_sha256']):
        raise ValueError('Invalid cinematic inventory identity')
    for name in ['sources', 'worlds', 'preparation_errors', 'sources_with_reported_gaps', 'sources_requiring_review', 'complete_scene_fidelity_verified']:
        if type(data[name]) is not int or data[name] < 0:
            raise ValueError('Invalid cinematic inventory count')
    if any(data[name] > data['sources'] for name in ['worlds', 'preparation_errors', 'sources_with_reported_gaps', 'sources_requiring_review']):
        raise ValueError('Cinematic count exceeds source inventory')
    if (data['visual_fidelity_checked'] is not False or data['complete_scene_fidelity_verified'] != 0
            or data['unverified'] != UNVERIFIED
            or data['scope'] != 'CPU preparation and declared-consumer gaps only'
            or data['priority_rule'] != 'Preparation barriers first; then distinct affected sources within each priority level.'):
        raise ValueError('Unsupported cinematic inventory verification claim')
    rows, seen = [], set()
    for row in data['priorities']:
        if set(row) != {'cause', 'priority', 'label', 'evidence_kind', 'affected_sources', 'affected_worlds', 'diagnostic_units'}:
            raise ValueError('Unexpected cinematic priority fields')
        cause = row['cause']
        if cause not in CAUSES or cause in seen:
            raise ValueError('Unknown or duplicate cinematic cause')
        seen.add(cause)
        if (row['priority'], row['label'], row['evidence_kind']) != CAUSES[cause]:
            raise ValueError('Noncanonical cinematic priority label')
        for name, maximum in [('affected_sources', data['sources']), ('affected_worlds', data['worlds']), ('diagnostic_units', None)]:
            if type(row[name]) is not int or row[name] < 1 or maximum is not None and row[name] > maximum:
                raise ValueError('Invalid cinematic priority count')
        rows.append(f'<tr><td><span class="inventory-priority priority-{row["priority"]}">P{row["priority"]}</span></td><th scope="row">{html.escape(row["label"])}</th>'
                    f'<td>{row["affected_sources"]}</td><td>{row["affected_worlds"]}</td><td>{html.escape(row["evidence_kind"].replace("_", " "))}</td></tr>')
    section = f'''<section id="cinematic-inventory" aria-labelledby="cinematic-inventory-title"><div class="section-heading"><div><p class="eyebrow">BATCH INVENTORY / SHARED CAUSES</p><h2 id="cinematic-inventory-title">Cinematic gaps to prioritize</h2></div></div>
<p><strong>{data['sources']} sources / {data['worlds']} worlds</strong> inspected by the real CPU preparers. {data['sources_with_reported_gaps']} sources report a preparation or implementation gap; {data['sources_requiring_review']} have items in the separate review queue (overlap possible); {data['preparation_errors']} fail top-level preparation. <strong>Zero complete scenes certified faithful.</strong></p>
<p>Preparation barriers first, then causes affecting the most distinct sources within each priority level. Counts overlap. A refused mission FX binding can coexist with a partial visual preview. Preparation-time review items do not establish missing live rendering.</p>
<div class="table-scroll" tabindex="0"><table class="summary-table"><caption>Common causes — current audited source, not a visual fidelity score</caption><thead><tr><th>Priority</th><th>Cause</th><th>Sources</th><th>Worlds</th><th>Evidence</th></tr></thead><tbody>{''.join(rows)}</tbody></table></div>
<p class="fineprint">Audit source {data['source_commit'][:12]}. Complete visual quality, native cadence, functional playback, lighting/shadows/water and audio/subtitle parity remain unverified. Detailed source identifiers, diagnostics and research evidence stay private.</p></section>'''
    return data, section


def export(output, root=ROOT):
    output = Path(output)
    if output.exists():
        raise ValueError('Refusing to overwrite site output')
    data = project(json.loads((root/'PROGRESS.json').read_text(encoding='utf8')))
    gallery = gallery_data(root)
    inventory, inventory_html = cinematic_inventory(root)
    features = [r for r in data['roadmap'] if r['kind'] == 'feature']
    foundations = [r for r in data['roadmap'] if r['kind'] == 'enabler']
    counts = count(features)
    summary = []
    for group in dict.fromkeys(r['group'] for r in features):
        c = count([r for r in features if r['group'] == group])
        segments = ''.join(f'<span class="{s}" style="flex:{c[s]}"></span>' for s in LABELS if c[s])
        summary.append(f'<tr><th scope="row">{html.escape(group)}</th><td><div class="status-bar" role="img" aria-label="{c["done"]} complete, {c["partial"]} partial, {c["todo"]} not started">{segments}</div></td>'
                       f'<td class="number done-text">{c["done"]}</td><td class="number partial-text">{c["partial"]}</td><td class="number todo-text">{c["todo"]}</td></tr>')
    replacements = {'SUMMARY': ''.join(summary), 'FEATURES': table(features, 'Whole-game feature roadmap'),
                    'FOUNDATIONS': table(foundations, 'Technical foundations — separate from game completion'),
                    'DONE': str(counts['done']), 'PARTIAL': str(counts['partial']), 'TODO': str(counts['todo']), 'TOTAL': str(len(features)),
                    'CINEMATIC_INVENTORY': inventory_html}
    document = (TEMPLATES/'index.html').read_text(encoding='utf8')
    for key, value in replacements.items():
        document = document.replace('{{'+key+'}}', value)
    output.mkdir(parents=True)
    (output/'index.html').write_text(document, encoding='utf8')
    for name in ('app.js', 'style.css'):
        shutil.copyfile(TEMPLATES/name, output/name)
    (output/'status.json').write_text(json.dumps(data, indent=2)+'\n', encoding='utf8')
    (output/'gallery.json').write_text(json.dumps({'comparisons': gallery['comparisons']}, indent=2)+'\n', encoding='utf8')
    if inventory is not None:
        (output/'cinematic-inventory.json').write_text(json.dumps(inventory, indent=2)+'\n', encoding='utf8')
    for name in gallery['images']:
        (output/name).parent.mkdir(exist_ok=True)
        shutil.copyfile(root/'docs/gallery'/name, output/name)
    (output/'.nojekyll').touch()
    return data


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    export(parser.parse_args().output)
