# Windows development preview - preview-2026.11.13.2

Runtime source: `b633f810de881c45cea8b7fb24250ec0314d47b8`.
Executable SHA256: `bebbc7adbbe4147079970b212a946a4c8750bb436eb8a39aa9d15f32c8c3e516`.

Supply your own compatible PC installation. Original assets, executables and
private research are excluded from the download.

## Test ordinary building collision and damage

Open **Preview tools -> Browse all missions -> Mecc Story 2 -> Starving!**.
Let the opening finish or press Escape to skip it. WASD/mouse move and aim,
left mouse fires the partial pistol, hold right mouse for the jet, Space adds
nitro, R changes view and Escape returns to the menu. No replay script or
building flag enables collisions or damage.

The entrance, columns and supported fences now block the player's actual mesh
contacts. The entrance also receives ordinary pistol damage: its individual
source health mesh changes through successive stages, with stone/debris/smoke,
positional sounds and final disappearance. Its detached columns remain separate
objects. Houses and small towers retain their own health, effects and retirement.
Player, generic environment and projectile memberships remain separate; the
player path does not invent generic footprints or walkable roofs.

The guard tower now receives normal pistol damage, spawns its actual mixed
debris/particle graph and positional sound, and is replaced by its own source
base. The base retains independent player collision after the effects expire;
it does not inherit the old tower's projectile membership. To try this nearby,
select **Mecc Story 2 -> The Road Home**, then use ordinary movement/jet/aim to
reach the guard tower. Seven hits reach its first health stage; thirty destroy
it. No script enables this feature.

The Story2 entry prepares 65 physical-player meshes, 31 generic environment
meshes, 65 projectile candidates and 59 bounded damage owners across 18 classes.
These are admission counts, not whole-game coverage. The source state0 table
retains contact and unchanged geometry after three actual hits: a health word
does not establish destructibility. The entrance's registered static-shadow
identity is retained; shadow drawing remains separate.

Replacement preparation is bounded to source classes 1127/1129; rendered guard
acceptance covers class 1050 -> 1127, not every variant. Restricted-weapon/special
scenery, other base-building callbacks, generic radial/Raster3 platform owners,
mission-death credit and complete native effects/physics remain open.

## Mission progression retained

Starving hunts and collects twenty settled meat, then requires returning to the
father on the starting plateau. Its authored collection/return predicates launch
the next source film and continue into The Wife with cleared meat. In The Wife,
reach the authored rescue destination: the 1/1 condition launches its actual
six-director completion film. Its natural end enters The Road Home, whose nearby
declared wife attaches through ordinary proximity pickup, carrying animation,
load, response audio and notice delivery. F8 toggles objective hints. The next
scene is reloaded; complete native inventory/world continuity is not claimed.

## Exact-executable validation and limits

Rendered supplied-command checks observe actual narrow player contact on the
entrance, house, fence and small tower. Seven ordinary pistol hits reach the
entrance's first stage; 28 retire it with all 124 source effect bodies expired.
House retirement also takes 28 hits/124 expiries; the small tower takes five
hits/29 expiries, the fence three hits/17 expiries. All observe actual draw and
muted positional sinks. The guard's 30 hits create five rigid debris pieces
and its separately rendered base. Actual source descendants bring the graph to
85 births/85 retirements, with no active effect left; 24 fresh narrow contacts
observe the base's own player mesh. Two guard-destruction/live-effect cancellation
and fresh-entry cycles release all old effect/GPU owners. The shared rigid renderer
uses the active ordinary mission camera/LOD after the opening owner is released.
The tower fixture travels through the source entrance. These checks supply normal
commands without pose, health, collider, fuel, readiness or expiry injection.

Both complete Starving and The Wife rendered continuation chains pass again,
including natural source-film ends, real muted sinks, ticket release and the
following ordinary player. Three common Mecc movement/fire/jet/nitro/release
cycles and actual landing dust/audio also pass. Menu lifecycle and the full
selected cinematic/text/cleanup controls pass on this same executable.

Separate actual Windows checks cover the normal menu, two Starving entries and
opening skips, R view change, brief remapped-key fire, restored default-left-mouse
fire, clean returns and close. They do not establish OS-aimed destruction,
sustained held flight or full OS-played missions. Supplied navigation remains
separately labeled.

Building effects retain an explicit partial host-frame, terrain-only,
birth-reference and asynchronous positional-audio preview profile. Native
complete-list/force/audio scheduling, pixel/audio parity, broad actor AI,
animation blending, other mission predicates, campaign failure/save restoration,
Delphi/Kabuto gameplay and complete fidelity remain incomplete.

1,013 portable Rust passes / 196 ignored, 45 Python passes, formatting and strict
workspace/all-target Clippy pass. Independent original-PC controls cover the
bounded player selection walk, registered shadow lookup/header/copies and the
previous house/death-FX decisions; supplied/observed service boundaries do not
establish full native world execution. Prototype PS2 lookup precedes acquisition.
Search routes include `MeccPhysicsExApplyLocation`, `object_copy_personality`,
`texture_add_file`, `house_process`, `house_shot`, `house_handle_destruction`,
`deathfx_shot`, `deathfx_logic`, `object_allocate_type`,
`object_copy_activedata`, `object_copy_posdirinfo` and
`object_bbox_to_world_bbox`. No new PC-to-PS2 identity is confirmed; no
prototype ABI/layout is imported.

Technical collection: 338 dependency and eleven toolchain notice entries,
29 system DLL imports. The ZIP is receipt-bound and must pass package validation.
These checks do not establish third-party rights or comprehensive human review.
The owner's approximate 99% AI-generated estimate describes authorship and is
never a completion metric.
