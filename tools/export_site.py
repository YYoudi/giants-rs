"""Export the public roadmap and owner-authorized comparison gallery."""
import argparse
import hashlib
import html
import json
from pathlib import Path
import re
import shutil

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


def export(output, root=ROOT):
    output = Path(output)
    if output.exists():
        raise ValueError('Refusing to overwrite site output')
    data = project(json.loads((root/'PROGRESS.json').read_text(encoding='utf8')))
    gallery = gallery_data(root)
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
                    'DONE': str(counts['done']), 'PARTIAL': str(counts['partial']), 'TODO': str(counts['todo']), 'TOTAL': str(len(features))}
    document = (TEMPLATES/'index.html').read_text(encoding='utf8')
    for key, value in replacements.items():
        document = document.replace('{{'+key+'}}', value)
    output.mkdir(parents=True)
    (output/'index.html').write_text(document, encoding='utf8')
    for name in ('app.js', 'style.css'):
        shutil.copyfile(TEMPLATES/name, output/name)
    (output/'status.json').write_text(json.dumps(data, indent=2)+'\n', encoding='utf8')
    (output/'gallery.json').write_text(json.dumps({'comparisons': gallery['comparisons']}, indent=2)+'\n', encoding='utf8')
    for name in gallery['images']:
        (output/name).parent.mkdir(exist_ok=True)
        shutil.copyfile(root/'docs/gallery'/name, output/name)
    (output/'.nojekyll').touch()
    return data


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    export(parser.parse_args().output)
