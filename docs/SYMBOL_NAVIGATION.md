# Original symbol navigation

This manually reviewed index lets an engine contributor search literal prototype
PS2 names and find the related Rust code. It is a small navigation aid, not an
export of the private symbol corpus or a complete PC-to-PS2 attribution table.
The central [reconstruction method](RECONSTRUCTION_METHOD.md) defines binary
identities, attribution evidence and the required name lookup.

## Reviewed identities

These routine identities are already confirmed by the central method. Their
Rust implementations retain narrower, separately documented PC behavior.

| Prototype PS2 name | Rust entry | Scope |
| --- | --- | --- |
| `SetupCameraDataNormal` (base of the raw mangled symbol recorded centrally) | [`mecc_camera_update::State::advance`](../crates/giants-game/src/mecc_camera_update.rs) | Ordinary acquired camera profiles; also discoverable by Rustdoc alias. |
| `chase_value_time` (literal ELF symbol) | [`mecc_camera_update::approach` / `approach_with_time`](../crates/giants-game/src/mecc_camera_update.rs) | Acquired camera call profile of the temporal-approach service. |

## Mission search route awaiting attribution

The mission work also follows the literal prototype symbol
`mission_check_obj_proximity__FP6OBJECT` (prototype address `0x0016d9e8`). Its
relationship to PC debug-1.1 `FUN_00497590` and
[`mission_objectives::proximity`](../crates/giants-game/src/mission_objectives.rs)
is **probable**, not confirmed. Independent evidence includes the mission-mark
filter, carried-resource comparison and one-shot counter/notice behavior. The
PC point/AABB fallback differs from the prototype's summed-radius test. The
central method pins both binary digests; old PC aliases remain valid. See
[MISSION_PROGRESSION.md](MISSION_PROGRESSION.md) for the bounded runtime scope.

## Fresh startup and tree search routes awaiting attribution

The literal prototype names `read_world_list` and
`scenerio_start_mission__Fv` also lead to
[`mission_cinematics::select_fresh_world`](../crates/giants-game/src/mission_cinematics.rs)
and the ordinary startup guard. Their PC identities remain **unknown**. Original
PC controls establish fresh empty slot initialization and the empty-opening
startup branch, with explicit external service boundaries. They do not establish
native continuity or public RoadHome completion; see MISSION_PROGRESSION.md.

Tree integration begins with the literal prototype search routes `tree_process`,
`tree_init` and `collision_wood__FP6OBJECT`. Their PC attributions remain
**unknown**. See [BUILDINGS.md](BUILDINGS.md) for the pending shared damage,
physical/render, wood-effect and replacement integration; current animated
foliage is not a delivered destructible-tree service.

## Vimp search routes awaiting attribution

The following seven names are literal function names present in the pinned
prototype ELF. Their existence is established; **their exact correspondence to
a PC function is still unknown**. The Rust locations identify the related work
for investigation, without asserting equivalent signatures, whole-function
coverage or behavior. Each source module repeats its search key so `rg` and
editor search work without opening this index first.

| Literal prototype PS2 symbol | Related Rust location | Attribution |
| --- | --- | --- |
| `vimp_logic` | [`vimp_living`](../crates/giants-game/src/vimp_living.rs), `Session::advance_vimp_living_after_history` | Unknown; partial common-primary composition. |
| `vimp_shot` | [`vimp_damage`](../crates/giants-game/src/vimp_damage.rs), `Session::damage_vimp_from_partial_pistol` | Unknown; source-bearing PC damage adapters. |
| `vimp_animselect` | [`vimp_animation`](../crates/giants-game/src/vimp_animation.rs) | Unknown; locomotion and hurt selection subsets. |
| `vimp_cleanup` | [`resident_states::vimp_retirement`](../crates/giants-game/src/resident_states/vimp_retirement.rs) | Unknown; bounded PC owner retirement. |
| `create_one_vimp_chunk` | [`vimp_chunks::Factory::birth`](../crates/giants-game/src/vimp_chunks.rs) | Unknown; owned meat birth. |
| `vimpchunk_move` | [`vimp_chunks`](../crates/giants-game/src/vimp_chunks.rs), `Session::advance_vimp_chunk` | Unknown; owned meat update. |
| `vimpsoul_move` | [`resident_states::vimp_soul`](../crates/giants-game/src/resident_states/vimp_soul.rs) | Unknown; state84 branch without mana-eligible players. |

Older Rust names and private PC address aliases remain valid search routes.
Finding a symbol here changes neither gameplay availability nor fidelity status.
The runtime's detailed prerequisites remain in [BACKLOG.md](../BACKLOG.md).

## Meat collection routes

| Literal prototype PS2 symbol | Related Rust location | Attribution |
| --- | --- | --- |
| `mecc_collect_food` | [`Session::collect_vimp_chunk`](../crates/giants-game/src/resident_states.rs), [`mecc_collection`](../crates/giants-game/src/mecc_collection.rs) | **Probable** PC routine identity; offline unparented subset implemented. |
| `mecc_check_food_chunk_collect` | [`mecc_periodic_scan::plan`](../crates/giants-game/src/mecc_periodic_scan.rs), [`Session::collect_nearby_vimp_chunks`](../crates/giants-game/src/resident_states/mecc_collection.rs) | **Probable** correspondence to an acquired PC primary block, not a separate established function. |
| `vimpchunk_destroy__FP6OBJECT` | [`Session::cleanup_collected_vimp_chunk`](../crates/giants-game/src/resident_states/vimp_retirement.rs) | **Unknown** relationship to generic PC retirement; no guessed state destructor. |

See [MEAT_COLLECTION.md](MEAT_COLLECTION.md) for independent evidence, version
differences, normal integration and the separately admitted objective boundary.

## Smartie proximity route

The literal prototype PS2 symbol `smartie_check_follow__FP6OBJECTf` leads to
[`smartie_follow::smartie_check_follow`](../crates/giants-game/src/smartie_follow.rs).
Its PC relationship is **probable**, not confirmed. The port admits the ordinary
selector3/Mecc selection branch with explicit clock, actor and inventory inputs;
it does not perform pickup or establish runtime integration. Keep the mangled
spelling as a search key; no equivalent Rust signature or platform layout is
asserted. See [MISSION_PROGRESSION.md](MISSION_PROGRESSION.md).

The literal routes `smartie_move` and `smartie_board_player__FP6OBJECTT0i`
also lead to [`smartie_declared`](../crates/giants-game/src/smartie_declared.rs)
and `Session::pickup_declared_mecc`. These function relationships remain
**unknown**. The new ordinary composition retains authored actor identity,
selector and flags and shares the acquired Mecc attachment services with
command12. It does not establish complete Smartie AI or equivalent ABI/layout.

## Physical finalization route

The literal prototype symbol `object_apply_location` leads to
[`actor_finalization::finalize_with_progress`](../crates/giants-game/src/actor_finalization.rs).
Its relationship to the debug-PC finalizer is **probable**. The prototype's
physical progress threshold starts at zero, whereas debug PC reuses its caller
argument bits. The portable `PrototypePs2Zero` policy selects only that threshold;
the default PC entry stays strict and other arithmetic remains independently
scoped. This is not an imported signature/layout or whole-function PS2 parity
claim. See [MISSION_PROGRESSION.md](MISSION_PROGRESSION.md).

## Detached cinematic splash route

The literal prototype name `fx_setupVerticleFade__FP10FXSTACKTAGfffff`
leads to `vertical_fade`, `effect_actor::Template::random_vertical_positioned`
and `cinematic_timed`. This remains an **unknown** PC attribution. The bounded
PC single-segment initializer/callback has independent controls including the
exact-duration boundary; the installed detached source event has canonical
creation, fade, lifetime and scoped cleanup controls. These do not establish
native scheduling, pixels, full film completion or any imported PS2 layout.

## Effect ground-contact routes

The literal prototype search route `FLK_FxSpawn__FP9FLK_FlickiiiiPc`
leads to `cinematic_support` and `cinematic_timed`. The PC cinematic effect
caller uses a loaded scalar class-zero support and source actor/reference pose;
the prototype uses a different support class and factorization. This is an
unknown function attribution, not permission to import its class or layout.
The source3 constructor snapshots the genuine support while preserving the
attached creation context. A recovered spawn name does not certify film fidelity.

The literal prototype symbols `fx_logic` and
`fx_processOpCodes__FP6FxDataiP6OBJECT` lead to the related work in
[`effect_ground_emission`](../crates/giants-game/src/effect_ground_emission.rs)
and the source constructor in `effect_actor::Template::ground_positioned`.
Both relationships remain **unknown**. The acquired PC contact subset decides
whether to request a child and retire its parent; source names do not validate
the physical query, child factory, whole callback or film integration. The
source constructor retains its mutable budget in the canonical actor payload.

The same unknown `fx_processOpCodes__FP6FxDataiP6OBJECT` route also reaches
[`effect_scene`](../crates/giants-game/src/effect_scene.rs): its construction
ledger retains the guard effect's sampled spin/curved size and pending
source15/source47 requests. Parameter and RNG controls do not execute their
physical, retirement, replacement or rendering consumers. No new identity or
prototype signature/layout is asserted.

The literal `object_render__FP6OBJECTiG7f32quat` is an **unknown** route for
[`static_geometry::Scene::constructed_rigid_draw`](../crates/giants-game/src/static_geometry/constructed.rs).
This new Rust composition reuses acquired source resources, LODs and material
profiles for already constructed rigid owners. It is not one attributed PC
function or proof of construction, collision registration or GPU execution.
The separately pinned renderer module's shader/material/cube name lookup has no
prototype match; its bounded material controls confirm no new PS2 identity.

The more specific literal `fx_setupGroundHitCreate__FP10FXSTACKTAGii` has a
**probable block-level relationship** to the acquired PC opcode5a writer in
`effect_ground_emission::Source::configure`. Independent body evidence includes
lookup/result gating, emission-budget/child stores and the matching control
flag. This is not a separate PC function identity or an equivalent signature;
lookup-argument representation and platform layout remain distinct. The shared
flag-writer route `fx_setupGroundHit__FP10FXSTACKTAGi` remains unknown.

`fx_orientToGround__FP10FXSTACKTAGf` is a literal prototype stub; the related PC
surface helper performs a real query, alignment and location update. Its
relationship remains **unknown**. Find the PC reconstruction in
`effect_actor::Template::surface_positioned` and `effect_surface`; do not replace
it with the prototype's no-op behavior.

## Building damage search routes

The literal prototype PS2 names `house_process`, `house_shot` and
`house_handle_destruction` lead to
[`building_damage`](../crates/giants-game/src/building_damage.rs). All three
function relationships remain **unknown**. The acquired debug-PC decision
kernels preserve cumulative damage, health-animation thresholds and ordered
effect/sound/mission/retirement requests. Their source/collider/render/audio
consumers and initial registration remain separate contracts; these kernels
alone establish neither ordinary interaction nor full callback execution.
The literal names `deathfx_shot` and `deathfx_logic` also lead to the separate
[`building_death_fx`](../crates/giants-game/src/building_death_fx.rs) decisions.
Both PC relationships remain **unknown**. Their weapon gates, direct captured
effect entries and replacement requests must not be replaced by universal house
behavior. The ordinary adapters are in `mission_buildings`, with source health
geometry in `building_geometry`; naming does not validate those consumers.
See [BUILDINGS.md](BUILDINGS.md) for current integration evidence and acceptance.

The literal prototype PS2 names `barracks_logic`, `barracks_shot`,
`barracks_alert_rippers__FP6OBJECT`, `rpman_alert_team__FP6OBJECTf` and
`distancesq2d__FP3P3DT0` also lead to
[`building_barracks`](../crates/giants-game/src/building_barracks.rs). These
relationships remain **unknown**. The PC building-origin team branch and the
inlined state23 alert loop retain different horizontal-radius and target rules.
The Rust service boundary preserves ordered alerts, damage, effects/sounds,
conditional mission requests and retirement. It neither constructs the missing
mission actors nor establishes a complete world list or ordinary building hit.
The canonical initial owner is in
[`resident_states::barracks`](../crates/giants-game/src/resident_states/barracks.rs),
with typed list alerts in
[`resident_states::building_alert`](../crates/giants-game/src/resident_states/building_alert.rs).
Initial selector/stock/channel setup and the health-frame helper use actual source
globals; their numeric contracts do not acknowledge later spawning/AI, a world
tick, rendered damage or an audio backend start. These routes remain unknown
attributions, independently of their PC controls.
The literal prototype PS2 names `sound_play_obj` and `sound_play_3dpos` also
provide **unknown** navigation routes to the object/position request adapter in
[`mission_buildings::barracks`](../crates/giants-app/src/mission_buildings/barracks.rs).
Independent PC controls retain the absent-descriptor no-voice branch; neither
an accepted request nor a missing source establishes backend playback. The shared
[`effect_scene`](../crates/giants-game/src/effect_scene.rs) continuation preserves
the original absent immediate-child branch without inventing a child or a random
draw. These bounded contracts do not confirm cross-platform identities or ABI.

## Publication boundary

`MeccPhysicsExApplyLocation` is an **unknown** prototype PS2 navigation route to
the related player-location/object-walk work. The bounded independent initial
mesh cohort is exposed by
[`declared_mesh_scene::prepare_players`](../crates/giants-game/src/declared_mesh_scene.rs)
and consumed through `player_world_collision`. Original-PC object-walk controls
separate player mesh eligibility from generic actor footprints. They do not
confirm this prototype identity, full native-world admission or platform
behavior. Player, environment and projectile role ownership remains distinct.

The literal prototype names `object_copy_personality` and `texture_add_file`
provide **unknown** navigation routes for
[`actor_static_shadow`](../crates/giants-game/src/actor_static_shadow.rs) and the
registered resource retained by `actor_fields`. Initial source registration,
class-header and copy controls do not confirm these function identities or
prototype signatures/layouts. Static shared references, missing-name allocation
and generated/dynamic copies stay distinct; shadow drawing and a new ordinary
building delivery do not follow from the field preparation. Prior aliases remain
unchanged, and detailed native records stay private.

The literal prototype PS2 name `hurt_object` is an **unknown** search route to
the separately acquired PC shot-dispatch boundary and
[`mission_buildings/no_shot`](../crates/giants-app/src/mission_buildings/no_shot.rs).
The original state0 descriptor has no callback function to attribute. Its
unchanged-actor controls remain distinct from the original positive house and
scenery callbacks, the ordinary hit adapter and pending rendered acceptance.
This imports no prototype signature or layout and confirms no new identity.
The exact PC dispatcher alias and both binary identities remain private.

The literal prototype PS2 names `FLK_FlickProcess__FP9FLK_Flick`,
`FLK_FlickUpdateTime__FP9FLK_Flickf` and `FlickAnimate__FP9FLK_Flick` are search
routes to [`flick::Timeline`](../crates/giants-game/src/flick.rs),
`cinematic_camera` and `mission_effects`. These relationships remain **unknown**,
not confirmed function identities. The acquired PC command/clock/process call
chain distinguishes reached requests from future declarations at animation
retirement. Rust retains that distinction without importing prototype layouts,
ABI or a guessed signature. See MISSION_PROGRESSION.md for the common runtime
settlement contract; naming does not certify rendering, timing or mission completion.

The literal prototype PS2 names `object_allocate_type`, `object_copy_personality`,
`object_copy_activedata`, `object_copy_posdirinfo` and `object_bbox_to_world_bbox`
lead to the bounded ordinary replacement owner in
[`mission_buildings::replacement`](../crates/giants-app/src/mission_buildings/replacement.rs)
and its shared class/header/bounds projections. These relationships remain
**unknown** PC identities. Constructor fields, acquired inheritance, placement
history, bounds inputs, independent physical/render identities and later world
admission remain separate evidence. Logical old-object/source aliases do not
assign semantics to the original opaque word. Naming is not native fidelity or
ordinary build acceptance; see [BUILDINGS.md](BUILDINGS.md).

The same constructor/copy routes also lead to
[`declared_residents`](../crates/giants-game/src/declared_residents.rs) and
[`actor_fields`](../crates/giants-game/src/actor_fields.rs), including the bounded
cold class215/state51 and class1060/1226/state98 profiles. These remain unknown
attributions; cold identities, payloads and list registration do not acknowledge
NPC AI, a world tick or the ordinary building-hit consumers.

This index publishes only selected names, reviewed navigation relationships and
attribution status. Binary/ELF inputs, raw tables, listings, matching outputs,
disassembly and detailed research remain private. The central method pins the
prototype identity and the PC identities used by reviewed attributions. Symbols
do not grant redistribution rights to those inputs or to game assets.
