# Windows development preview - preview-2026.11.12.3

Runtime source: `d5320b18cb352fbed05d882880974a0887152e5f`.
Executable SHA256: `d7c41dc916cceca3ec83f03278b91b8d75eb9805a1926cf5b4685ccc0ee80f7b`.

Supply your own compatible PC installation. Original assets, executables and
private research are excluded from the download.

## Test ordinary building damage

Open **Preview tools -> Browse all missions -> Mecc Story 2 -> Starving!**.
Let the opening finish or press Escape to skip it. WASD/mouse move and aim,
left mouse fires the partial pistol, hold right mouse for the jet, Space adds
nitro, R changes view and Escape returns to the menu. No replay script or
building flag enables collisions or damage.

Shoot the ordinary Smartie houses: their source health animation changes the
individual building through successive damage states, with stone/debris/smoke,
positional sounds and final disappearance. Matching admitted player/environment
and projectile shapes change with the visible mesh; neighboring houses retain
their own health. Small towers and admitted wood/stone scenery use their own
source limits and captured effects. Projectile-only fences can be shot without
gaining an invented player-blocking footprint. Some visible buildings remain
unsupported; a health field alone never makes a building destructible.

The Story2 source entry prepares 65 independent projectile candidates and 56
bounded damage owners across 15 classes. These are admission counts, not a
whole-game coverage percentage. Physical radial/Raster3 shapes, complex guard
tower replacement/effects, restricted-weapon/special scenery, other base-building
families and building-objective death credit remain open. See BUILDINGS.md in
the source repository for the shared contracts and remaining consumers.

## Mission progression retained

Starving hunts and collects twenty settled meat, then requires returning to the
father on the starting plateau. The authored collection/return predicates launch
the next source film and continue into The Wife with cleared meat. In The Wife,
reach the authored rescue destination: the 1/1 condition launches its actual
six-director completion film. Its natural end enters The Road Home, whose nearby
declared wife attaches through ordinary proximity pickup, carrying animation,
load, response audio and notice delivery. F8 toggles objective hints. The next
scene is reloaded; complete native inventory/world continuity is not claimed.

## Exact-executable validation and limits

Rendered supplied-command checks on this executable observe narrow player
contact, 28 ordinary pistol shots, house health stages and retirement, 124 source
effect births and all 124 expiries on the actual mission clock. A separate
seven-shot capture shows the first damage stage. Small-tower contact/five-shot
destruction expires all 29 bodies; projectile-only fence destruction takes three
shots and expires all 17 bodies. Two house-damage/live-effect cancellation and
fresh-entry cycles require old-ticket GPU/owned-entity release and intact fresh
health. These checks neither inject health/pose nor force readiness/cleanup.

Both complete Starving and The Wife rendered continuation chains pass again,
including real muted sinks, natural source-film terminals, ticket release and
the following ordinary player. Three common Mecc movement/fire/jet/nitro/release
cycles and actual landing dust/audio also pass.

Separate actual Windows checks cover the normal menu, two Starving entries and
opening skips, R view change, brief remapped-key fire, restored default-left-mouse
fire, clean returns and close. They do not establish OS-aimed destruction,
sustained held flight or full OS-played missions. Absolute pointer automation
can produce unsuitable camera movement; supplied navigation remains separately
labeled.

Building effects retain an explicit partial host-frame, terrain-only,
birth-reference and asynchronous positional-audio preview profile. Native
complete-list/force/audio scheduling, pixel/audio parity, broad actor AI,
animation blending, other mission predicates, campaign failure/save restoration,
Delphi/Kabuto gameplay and complete fidelity remain incomplete.

1,005 portable Rust passes / 187 ignored, 45 Python passes, formatting and strict
workspace/all-target Clippy pass. Private original-PC decision comparisons cover
3,080 house and 7,000 separate death-FX contexts with no differences; external
services there are observed/supplied boundaries, not live world execution.
Prototype PS2 lookup precedes acquisition. Search routes include `house_process`,
`house_shot`, `house_handle_destruction`, `deathfx_shot` and `deathfx_logic`.
No new PC-to-PS2 identity is confirmed; no prototype ABI/layout is imported.

Technical collection: 338 dependency and eleven toolchain notice entries,
29 system DLL imports. The ZIP is receipt-bound and must pass package validation.
These checks do not establish third-party rights or comprehensive human review.
The owner's approximate 99% AI-generated estimate describes authorship and is
never a completion metric.
