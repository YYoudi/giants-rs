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
PREVIEW_CHECKS = {
    'palm_stump_gpu': 'Ordinary palm contact and pistol destruction hide twelve foliage parts, submit source wood effects/audio and leave a separately colliding stump; seventeen births/seventeen retirements and two fresh stump contacts after expiry. Supplied normal commands; other trees and native fidelity remain unverified',
    'road_home_return_gpu': 'Actual carried-wife return satisfies the authored counter, plays the source completion film to its natural end and enters the next ordinary Mecc scene. The strict combined report retains the separate High Village AI platform refusal; this is not next-mission completion',
    'barracks_damage_gpu': 'Acquired miniature and Ripper barracks receive ordinary pistol damage through source stages, effects/audio and retirement; rendered supplied-command acceptance and actual-clock expiry, without complete spawning AI or mission death-credit',
    'ripper_movement_attack_gpu': 'The Wife ordinary classic Ripper moves about147 source units, emerges and launches an actual owned projectile in the rendered common runtime; supplied Mecc commands only, no forced NPC state or reciprocal-damage claim',
    'normal_menu_visible': 'The same executable renders its normal Windows menu without replay or NPC activation flags; computer use stops before mission-input and re-entry acceptance',
    'building_guard_replacement_gpu': 'Ordinary guard damage, five rigid debris pieces, nested source particles and positional sound; 85 births/85 retirements and 24 fresh contacts on the independent source base after expiry. Supplied normal controls, not OS-aimed destruction or native fidelity',
    'building_indestructible_gpu': 'The source state0 table receives actual narrow player contact and three distinct normal pistol hits while keeping unchanged geometry, with no burst or retirement',
    'building_player_meshes_gpu': 'Actual narrow player contacts on the entrance, fence, house and small tower; normal pistol damage updates each admitted player/projectile shape with its rendered mesh and retirement, with source effects and actual-clock expiry. Supplied commands; generic actor footprints and native fidelity remain separate',
    'building_house_damage_gpu': 'Ordinary pistol hits change individual source house health meshes and all admitted physical/projectile inputs, cross source thresholds, play real muted positional sinks and retire the house; supplied navigation/fire, no forced health/pose or native-fidelity claim',
    'building_scenery_damage_gpu': 'A small source tower receives real narrow player contact and five ordinary pistol hits, renders its own captured wood/stone burst and sound, then retires; all 29 effect bodies expire on the actual mission clock',
    'building_projectile_only_gpu': 'A source fence receives three ordinary pistol hits and its wood/smoke effect/sound, then retires with all 17 bodies expired; independent projectile eligibility does not establish player lateral/platform response',
    'building_cancel_reentry_gpu': 'Two ordinary guard-destruction/live-effect cancellation and fresh-entry cycles release old ticket/draw/GPU owners and restart with intact source health; supplied commands, not forced cleanup',
    'building_menu_fire_os': 'Actual Windows menu, normal mission entry/skip, view and brief ordinary pistol fire plus return/re-entry on this executable; accurate OS-aimed destruction and sustained held input remain separate',
    'wife_campaign_chain_gpu': 'Actual The Wife rescue satisfies its authored 1/1 condition, plays the six-director completion film to its natural source end, releases the old ticket and enters The Road Home with the declared wife actually picked up, rendered and supplying carrying load, real muted audio and notice delivery; supplied commands, no forced pose/resource/readiness/film end',
    'wife_short_input_os': 'Actual Windows menu entry into The Wife, opening skip, short keyboard input, view change and pistol/mouse pulses, followed by two ordinary RoadHome entries with visible wife pickup and clean menu return; the complete rescue and sustained held flight are not certified as OS-played interactions',
    'starving_short_input_os': 'Actual Windows menu entry/re-entry into Starving, short keyboard movement, view changes, pistol/jet mouse pulses and clean menu return; the complete hunt and sustained held flight are separately unverified as OS-played interactions',
    'starving_campaign_chain_gpu': 'Actual Starving hunt collects twenty meat, satisfies the authored return predicate and both objective producers, releases the old ticket, renders the following source film to its real end and resumes an ordinary player with cleared meat; supplied commands and real muted sinks',
    'story5_authored_herds_gpu': 'Story5 source Mecc entry admits its two authored herds, nineteen world markers and one mission marker, generates living Vimps and draws them through the shared mission component; supplied commands, not full Story5 mission acceptance',
    'story2_meat_collection_gpu': 'The ordinary Mecc controller collects actual settled Vimp meat, updates its carried count, retires the picked pieces and starts real muted pickup sinks; supplied navigation, no injected births or OS-held-input claim',
    'offscreen_menu_cycles': 'Offscreen development launches, menu return and re-entry',
    'selected_cinematic_transport': 'Selected-mission source-terminal/re-entry with muted real sinks',
    'manual_menu_skip_return': 'Manual menu, opening skip, mission entry and return on one local profile',
    'ordinary_mecc_jet_projectile_gpu': 'Ordinary-controller jet/nitro/release GPU cycles and visible jet/projectile samples with supplied commands',
    'ordinary_mecc_descent_gpu': 'Ordinary selector reaches visible descent after supplied thrust/release; jet draws hidden and fuel gauge updated; OS-held flight remains unverified',
    'ordinary_mecc_os_pulses': 'Normal menu entry/re-entry, OS jet/fire mouse pulses, view toggle and exit; sustained held flight remains unverified',
    'ordinary_mecc_short_fire_os': 'Brief remapped keyboard and default mouse fire verified through the normal menu; native damage remains incomplete',
    'ordinary_mecc_landing_gpu': 'Actual terrain contact creates visible canonical landing dust and a real muted positional sink after supplied flight commands',
    'ordinary_mecc_cycles_gpu': 'Three ordinary admission/motion/fire/jet/nitro/release cycles and clean menu returns with supplied commands and real muted transport',
    'story2_vimp_reaction_gpu': 'Story2 supplied-command approach/fire reaches the actual herd, renders canonical Vimp startle/return and starts real muted positional sinks; no OS-held input claim',
    'story2_vimp_combat_gpu': 'Actual partial-pistol injuries and deaths render source blood/FX, meat and ghosts with real muted injury/death sinks; supplied navigation/fire, not OS-held input or native-fidelity proof',
    'story2_vimp_contact_gpu': 'The graphical ordinary controller walks into actual living Vimp collision meshes and records player contact responses; supplied navigation, no shots or forced poses, not OS-held/native motion proof',
    'shared_body_focus_os': 'F7 focuses initial Delphi and Kabuto bodies from ordinary mission inspection; gameplay controls remain unavailable',
}
PREVIEW_LIMITS = {
    'palm_remaining_profiles': 'Palm collision uses the acquired initial idle pose; live wind/collision alignment, other vegetation classes and heavy-actor felling remain unverified or unacquired',
    'high_village_ai_platform': 'High Village ordinary player entry follows the completed return film, but Ripper AI refuses an unacquired source-platform cold command46; its full mission is not certified',
    'ripper_combat_incomplete': 'Reciprocal Ripper injury/death, special close attacks and full encounter/menu re-entry remain incomplete. Unsupported actors hold only their last completed visual pose and supply no fresh contacts; failed projectiles acknowledge no damage',
    'building_remaining_families': 'Generic actor radial/Raster3 platform owners, replacements beyond the acquired profiles, restricted-weapon/special scenery, other base-building callbacks and building-objective credit still require their real consumers; a configured health word alone does not imply destructibility',
    'building_fx_policy': 'Building FX use source burst and acquired mixed-scene resources with an explicit partial host-frame, terrain-only, birth-reference and asynchronous-audio profile; native complete-list/force/audio scheduling and pixel/audio parity remain unverified',
    'cinematic_native_fidelity': 'The Wife completion film reaches its source terminal; native timing/RNG interleaving, complete collision coverage, animation blending and full visual/audio fidelity remain unverified',
    'mission_objective_subset': 'Only admitted collection and stationary proximity producers are connected; moving, death, equipment and other mission conditions still require their actual owners',
    'native_campaign_continuity': 'Next source mission loads a fresh scene/player and clears meat; complete native equipment restoration, persistent world and save-game continuity remain incomplete',
    'meat_economy_outside_objectives': 'General feeding, deposits, economy and replenishment remain unavailable outside the admitted mission objective subset',
    'other_actor_ai': 'Other actor AI, live Smartie animation and complete native-world admission remain incomplete; authored herd declarations alone do not enable Delphi, Kabuto or multiplayer gameplay',
    'meat_feeding_economy': 'Meat feeding, deposits, economy and campaign objective credit remain unavailable; the preview uses the explicit no-objective-credit branch',
    'ordinary_mecc_native_motion': 'Full native Mecc primary, damage and world scheduling remain incomplete',
    'delphi_gameplay': 'Delphi gameplay is not enabled',
    'kabuto_gameplay': 'Kabuto gameplay is not enabled',
    'npc_remaining_ai': 'The partial Story2 Vimp interaction is connected; other staged AI, feeding, objectives and complete native-world admission remain unavailable',
    'vimp_fx_preview_policy': 'Death FX use explicit host-frame scheduling, terrain-only contacts and owner-lifetime splat retention; complete native effects/world scheduling remains unverified',
    'meat_collection': 'Meat drops are visible; collection, feeding and the associated economy are not enabled',
    'ordinary_controls_os_unverified': 'Real keyboard/mouse gameplay on this executable remains unverified; supplied-command GPU checks and prior-build manual evidence are separately scoped',
    'npc_live_ai': 'NPC AI is not admitted to ordinary missions',
    'full_game_fidelity': 'Complete campaigns and visual/audio fidelity remain unverified',
}


def public_preview(root):
    """Read only a reviewed, published download snapshot, never infer it from main."""
    path = root/'docs/public-preview.json'
    if not path.exists():
        return None, ''
    data = json.loads(path.read_text(encoding='utf8'))
    fields = {'schema_version', 'tag', 'source_commit', 'executable_sha256',
              'archive_sha256', 'checks', 'limitations'}
    if (not isinstance(data, dict) or set(data) != fields
            or type(data['schema_version']) is not int or data['schema_version'] != 1):
        raise ValueError('Unexpected public preview fields')
    patterns = {'tag': r'preview-\d{4}\.\d{2}\.\d{2}\.[1-9]\d*',
                'source_commit': r'[a-f0-9]{40}', 'executable_sha256': r'[a-f0-9]{64}',
                'archive_sha256': r'[a-f0-9]{64}'}
    for name, pattern in patterns.items():
        if not isinstance(data[name], str) or not re.fullmatch(pattern, data[name]):
            raise ValueError('Invalid public preview identity')
    for name, catalog in [('checks', PREVIEW_CHECKS), ('limitations', PREVIEW_LIMITS)]:
        values = data[name]
        if (not isinstance(values, list)
                or any(not isinstance(v, str) or v not in catalog for v in values)
                or len(set(values)) != len(values) or name == 'checks' and not values):
            raise ValueError('Unsupported public preview scope')
    base = 'https://github.com/YYoudi/giants-rs'
    release = f'{base}/releases/tag/{data["tag"]}'
    download = f'{base}/releases/download/{data["tag"]}/giants-preview-windows-x64.zip'
    checks = '; '.join(html.escape(PREVIEW_CHECKS[v]) for v in data['checks'])
    limits = '; '.join(html.escape(PREVIEW_LIMITS[v]) for v in data['limitations']) or 'See the release notes for the assessed scope'
    section = ('<section id="current-preview" aria-labelledby="current-preview-title">'
               '<div class="section-heading"><div><p class="eyebrow">PUBLISHED WINDOWS BUILD</p>'
               '<h2 id="current-preview-title">Current Windows download</h2></div>'
               f'<a href="{download}">Download ZIP ↗</a></div>'
               '<p>Supply your own game installation. This download is a fixed build; later source milestones and archived captures do not update its capabilities.</p>'
               '<div class="table-scroll" tabindex="0"><table class="summary-table"><caption>Published build snapshot — separate from the source roadmap</caption><tbody>'
               f'<tr><th scope="row">Release</th><td><a href="{release}">{data["tag"]} / release notes ↗</a></td></tr>'
               f'<tr><th scope="row">Built from</th><td><a href="{base}/commit/{data["source_commit"]}">{data["source_commit"][:12]} ↗</a></td></tr>'
               f'<tr><th scope="row">Checks on this executable</th><td>{checks}</td></tr>'
               f'<tr><th scope="row">Still incomplete</th><td>{limits}</td></tr>'
               '<tr><th scope="row">Checksums</th><td><details><summary>ZIP and executable SHA256</summary>'
               f'<p>ZIP <code>{data["archive_sha256"]}</code></p><p>EXE <code>{data["executable_sha256"]}</code></p>'
               '</details></td></tr></tbody></table></div></section>')
    return data, section


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
    for row in data.get('screenshots', []):
        required = {'title', 'image', 'caption', 'commit'}
        receipt = {'preview_tag', 'executable_sha256'}
        if not required <= set(row) or set(row) - required not in (set(), receipt):
            raise ValueError('Unexpected screenshot field')
        if receipt <= set(row) and (not re.fullmatch(r'preview-\d{4}\.\d{2}\.\d{2}(?:\.\d+)?', row['preview_tag'])
                                  or not re.fullmatch(r'[a-f0-9]{64}', row['executable_sha256'])):
            raise ValueError('Invalid screenshot preview receipt')
        if any(not isinstance(v, str) or PRIVATE.search(v) for v in row.values()):
            raise ValueError('Private path in screenshot')
        if (not re.fullmatch(r'media/[a-f0-9]{64}\.webp', row['image'])
                or row['image'] not in data['images']
                or not re.fullmatch(r'[a-f0-9]{40}', row['commit'])):
            raise ValueError('Unapproved screenshot image or revision')
    for name, digest in data['images'].items():
        if not re.fullmatch(r'media/[a-f0-9]{64}\.webp', name):
            raise ValueError('Invalid image path')
        if hashlib.sha256((path.parent/name).read_bytes()).hexdigest() != digest or Path(name).stem != digest:
            raise ValueError('Gallery image hash mismatch')
    return data


def screenshots_html(gallery, preview=None):
    rows = gallery.get('screenshots', [])
    if not rows:
        return '', '<figure id="hero-image" hidden></figure>'
    esc = html.escape
    cards = []
    def current(row):
        return bool(preview and row['commit'] == preview['source_commit']
                    and row.get('preview_tag') == preview['tag']
                    and row.get('executable_sha256') == preview['executable_sha256'])
    for row in rows:
        availability = ('Capture from the current Windows download. The caption states its test conditions; visual fidelity remains unverified.'
                        if current(row) else 'Archived diagnostic capture. Availability in the current download or ordinary gameplay is not established by this image.')
        cards.append(f'<figure class="build-capture"><a href="{row["image"]}" target="_blank" rel="noopener" aria-label="Open full-size image: {esc(row["title"])}">'
                     f'<img src="{row["image"]}" alt="{esc(row["title"])}" width="1920" height="1080" decoding="async"></a>'
                     f'<figcaption><h3>{esc(row["title"])}</h3>'
                     f'<p class="capture-availability">{availability}</p>'
                     f'<p>{esc(row["caption"])}</p>'
                     f'<a class="capture-revision" href="https://github.com/YYoudi/giants-rs/commit/{row["commit"]}">Recorded build {row["commit"][:7]} ↗</a></figcaption></figure>')
    section = ('<section id="latest-captures" aria-labelledby="latest-captures-title"><div class="section-heading"><div>'
               '<p class="eyebrow">ACTUAL BEVY RENDERS / ARCHIVE</p><h2 id="latest-captures-title">Recorded development builds</h2></div>'
               '<a href="#captures">Browse before / after comparisons ↓</a></div>'
               '<p>These captures document specific development revisions, including scripted inspections. '
               'They do not certify ordinary gameplay in the latest Windows preview. Open an image for its full-size view and recorded revision. Visual fidelity remains unverified.</p>'
               f'<div class="capture-grid">{"".join(cards)}</div></section>')
    identity = ('Current Windows preview capture · fidelity unverified' if current(rows[0])
                else 'Archived diagnostic render · ordinary gameplay availability unverified')
    hero = (f'<figure id="hero-image"><img src="{rows[0]["image"]}" alt="" decoding="async">'
            f'<figcaption>{identity} · build {rows[0]["commit"][:7]}</figcaption></figure>')
    return section, hero


def featured_captures(gallery):
    """Render reviewed display copies even when gallery JavaScript is unavailable."""
    titles = gallery.get('featured_comparisons', [])
    if (not isinstance(titles, list) or any(not isinstance(title, str) for title in titles)
            or len(titles) > 3 or len(set(titles)) != len(titles)):
        raise ValueError('Invalid featured comparison selection')
    rows = []
    for title in titles:
        matches = [row for row in gallery['comparisons'] if row['title'] == title]
        if len(matches) != 1:
            raise ValueError('Featured comparison must identify one reviewed gallery row')
        rows.append(matches[0])
    if not rows:
        return '', ''
    esc = html.escape
    cards = []
    for row in rows:
        cards.append(f'<figure><a href="{esc(row["right"])}" target="_blank" rel="noopener">'
                     f'<img src="{esc(row["right"])}" alt="{esc(row["right_label"])}" '
                     f'loading="eager" decoding="async" width="960" height="600"></a>'
                     f'<figcaption><strong>{esc(row["title"])}</strong>'
                     f'<span>{esc(row["right_label"])}</span>'
                     '<span class="capture-availability">Historical diagnostic; not a current gameplay availability claim.</span></figcaption></figure>')
    section = ('<section id="highlights" aria-labelledby="highlights-title">'
               '<div class="section-heading"><div><p class="eyebrow">RUST + BEVY / DEVELOPMENT CAPTURES</p>'
               '<h2 id="highlights-title">Historical diagnostic highlights</h2></div>'
               '<a href="#captures">Open all comparisons ↘</a></div>'
               '<p>Historical engine captures: jetpack, first-person view and cinematic rendering. '
               'These include scripted checks and do not establish current ordinary-gameplay availability. '
               'Each image retains its archive context in the full gallery; current-build fidelity remains unverified.</p>'
               f'<div class="capture-highlights">{"".join(cards)}</div></section>')
    hero = (f'<img src="{esc(rows[0]["right"])}" alt="{esc(rows[0]["right_label"])}" '
            'fetchpriority="high" decoding="async">'
            '<figcaption>Historical Bevy development render · fidelity unverified</figcaption>')
    return hero, section



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
<div class="table-scroll" tabindex="0"><table class="summary-table"><caption>Common causes — recorded executable audit, not a visual fidelity score</caption><thead><tr><th>Priority</th><th>Cause</th><th>Sources</th><th>Worlds</th><th>Evidence</th></tr></thead><tbody>{''.join(rows)}</tbody></table></div>
<p class="fineprint">Inventory source {data['source_commit'][:12]}, executable SHA256 {data['executable_sha256'][:12]}. This separate audit may predate the current download; its counts are not transferred to newer builds. Complete visual quality, native cadence, functional playback, lighting/shadows/water and audio/subtitle parity remain unverified. Detailed source identifiers, diagnostics and research evidence stay private.</p></section>'''
    return data, section


def export(output, root=ROOT):
    output = Path(output)
    if output.exists():
        raise ValueError('Refusing to overwrite site output')
    data = project(json.loads((root/'PROGRESS.json').read_text(encoding='utf8')))
    preview, preview_html = public_preview(root)
    gallery = gallery_data(root)
    screenshots, hero = screenshots_html(gallery, preview)
    historical_hero, featured = featured_captures(gallery)
    if not screenshots and historical_hero:
        hero = f'<figure id="hero-image">{historical_hero}</figure>'
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
                    'CINEMATIC_INVENTORY': inventory_html, 'SCREENSHOTS': screenshots, 'HERO': hero, 'FEATURED_CAPTURES': featured,
                    'PUBLIC_PREVIEW': preview_html}
    document = (TEMPLATES/'index.html').read_text(encoding='utf8')
    for key, value in replacements.items():
        document = document.replace('{{'+key+'}}', value)
    output.mkdir(parents=True)
    (output/'index.html').write_text(document, encoding='utf8')
    for name in ('app.js', 'style.css'):
        shutil.copyfile(TEMPLATES/name, output/name)
    (output/'status.json').write_text(json.dumps(data, indent=2)+'\n', encoding='utf8')
    (output/'gallery.json').write_text(json.dumps({'comparisons': gallery['comparisons']}, indent=2)+'\n', encoding='utf8')
    if preview is not None:
        (output/'public-preview.json').write_text(json.dumps(preview, indent=2)+'\n', encoding='utf8')
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
