"""Generate the public README checklist from the whole-game roadmap."""
import json
from pathlib import Path
root=Path(__file__).resolve().parents[1]
def main():
    rows=json.loads((root/'PROGRESS.json').read_text(encoding='utf8'))['readme_roadmap']
    lines=[]; group=None
    for row in rows:
        if row['group'] != group:
            group=row['group']; lines += ['', '### '+group, '']
        mark='x' if row['status']=='done' else ' '
        lines.append(f"- [{mark}] **{row['title']}** — {row['status']}; {row['scope']}. {row.get('detail', '')}")
    path=root/'README.md'; text=path.read_text(encoding='utf8')
    start='<!-- progress:start -->';end='<!-- progress:end -->'
    before,tail=text.split(start,1);_,after=tail.split(end,1)
    path.write_text(before+start+'\n'+'\n'.join(lines)+'\n'+end+after,encoding='utf8')
    features=[r for r in rows if r['kind']=='feature']
    counts={s:sum(r['status']==s for r in features) for s in ('done','partial','todo')}
    svg=f'''<svg xmlns="http://www.w3.org/2000/svg" width="720" height="110" viewBox="0 0 720 110" role="img" aria-label="Whole-game feature families: {counts['done']} complete, {counts['partial']} partial, {counts['todo']} not started">
<rect width="720" height="110" rx="12" fill="#17212f"/>
<g fill="#eef4ff" font-family="sans-serif"><text x="20" y="30" font-size="18">Whole-game roadmap · feature families</text>
<text x="20" y="62" font-size="16">Complete: {counts['done']}   Partial: {counts['partial']}   Not started: {counts['todo']}</text>
<text x="20" y="90" font-size="13">CPU acquisition ≠ runtime integration ≠ verified fidelity · no global percentage</text></g></svg>'''
    (root/'docs/roadmap.svg').write_text(svg,encoding='utf8')
if __name__=='__main__': main()
