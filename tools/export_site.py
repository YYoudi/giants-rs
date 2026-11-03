"""Generate the asset-free public roadmap dashboard from an explicit field list."""
import argparse
import html
import json
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
FIELDS = ('id', 'kind', 'group', 'title', 'status', 'scope', 'detail')


def project(source):
    rows = []
    for original in source['readme_roadmap']:
        row = {key: str(original[key]) for key in FIELDS if key in original}
        if row['status'] not in ('done', 'partial', 'todo') or row['kind'] not in ('feature', 'enabler'):
            raise ValueError('Unknown roadmap state')
        for value in row.values():
            if re.search(r'[A-Za-z]:[\\/]|(?:evidence|\.local)[\\/]|Users[\\/]', value, re.I):
                raise ValueError('Private path in public roadmap field')
        rows.append(row)
    return {'project': 'giants-rs', 'overall_fidelity_verified': source.get('overall_fidelity_verified') is True,
            'roadmap': rows}


def export(output, root=ROOT):
    output = Path(output)
    if output.exists():
        raise ValueError('Refusing to overwrite site output')
    data = project(json.loads((root/'PROGRESS.json').read_text(encoding='utf8')))
    esc = html.escape
    features = [r for r in data['roadmap'] if r['kind'] == 'feature']
    counts = {s: sum(r['status'] == s for r in features) for s in ('done', 'partial', 'todo')}
    sections = []
    for group in dict.fromkeys(r['group'] for r in data['roadmap']):
        cards = []
        for row in (r for r in data['roadmap'] if r['group'] == group):
            cards.append(f'<article data-status="{row["status"]}"><h3>{esc(row["title"])}</h3>'
                         f'<p class="tag">{esc(row["status"])} · {esc(row["scope"])}</p>'
                         f'<p>{esc(row.get("detail", ""))}</p></article>')
        sections.append(f'<section><h2>{esc(group)}</h2><div class="cards">'+''.join(cards)+'</div></section>')
    document = '''<!doctype html><html lang="en"><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>giants-rs · Whole-game roadmap</title><style>
:root{color-scheme:dark;font:16px/1.6 system-ui;background:#111a27;color:#edf3ff}
body{max-width:1120px;margin:auto;padding:32px}h1{font-size:2.6rem;line-height:1.15}
a{color:#9ccfff}header,.notice{background:#1c2b40;padding:24px;border-radius:14px}
.cards{display:grid;grid-template-columns:repeat(auto-fit,minmax(270px,1fr));gap:14px}
article{background:#1b283a;border:1px solid #3b516c;border-radius:12px;padding:18px}
h3{margin:0}.tag{color:#b7d6ff}label{display:inline-block;margin:8px 20px 8px 0}
input[type=search]{padding:12px;width:min(440px,90%);font:inherit}section{margin-top:32px}
[hidden]{display:none!important}footer{margin-top:40px;color:#b5c0d3}
</style><header><h1>giants-rs</h1><p>Independent Rust + Bevy reconstruction of Giants: Citizen Kabuto.
The goal is the complete game. The Windows menu-first preview is incomplete.</p>
<p><a href="https://github.com/YYoudi/giants-rs">Source and README</a> ·
<a href="https://github.com/YYoudi/giants-rs/actions">Build checks</a> ·
<a href="https://github.com/YYoudi/giants-rs/releases">Reviewed releases</a></p></header>
<p class="notice">The owner estimates approximately <strong>99% AI-generated content</strong>, without
comprehensive human review. This is neither a measured authorship statistic nor a completion percentage.
This project does not claim a separated-team clean room. Original game data must be supplied separately.</p>
<h2>Whole-game feature families</h2>'''
    document += f'<p>{counts["done"]} complete · {counts["partial"]} partial · {counts["todo"]} not started</p>'
    document += '''<p>CPU acquisition, runtime integration and verified fidelity are separate.
Technical foundations appear separately below. No global completion percentage is claimed.
No complete scene or campaign is certified by this dashboard.</p>
<label>Search <input id="search" type="search" placeholder="Feature or system"></label>
<label>Status <select id="status"><option value="">All</option><option>partial</option><option>todo</option><option>done</option></select></label>
<p id="results" aria-live="polite"></p>'''+''.join(sections)
    document += '''<footer>Generated from PROGRESS.json in the working repository.
Only allowlisted roadmap fields are exported. No private evidence, original assets or screenshots are included.
Build checks do not certify playable missions, fidelity or redistribution rights.</footer>
<script src="app.js"></script></html>'''
    output.mkdir(parents=True)
    (output/'index.html').write_text(document, encoding='utf8')
    (output/'status.json').write_text(json.dumps(data, indent=2)+'\n', encoding='utf8')
    (output/'.nojekyll').touch()
    (output/'app.js').write_text('''const search = document.querySelector('#search');
const status = document.querySelector('#status');
function filter() {
  let count = 0;
  for (const card of document.querySelectorAll('article')) {
    card.hidden = !card.textContent.toLowerCase().includes(search.value.toLowerCase()) ||
      (status.value !== '' && card.dataset.status !== status.value);
    if (!card.hidden) count++;
  }
  for (const section of document.querySelectorAll('section')) {
    section.hidden = !section.querySelector('article:not([hidden])');
  }
  document.querySelector('#results').textContent = `${count} roadmap entries shown`;
}
search.addEventListener('input', filter);
status.addEventListener('change', filter);
filter();
''', encoding='utf8')
    return data


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    export(parser.parse_args().output)
