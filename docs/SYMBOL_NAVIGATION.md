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
| `ripper_death_logic__FP6OBJECT` (raw symbol; base name `ripper_death_logic`) | [`ripper_corpse::ripper_death_logic`](../crates/giants-game/src/ripper_corpse.rs) | Confirmed routine identity; bounded PC class73/230 corpse helper, without ordinary lethal integration or positive mana-creation acceptance. |

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

## Ripper search route awaiting attribution

The literal prototype PS2 symbol `ripper_move` is a navigation candidate for
[`ripper_primary_turn`](../crates/giants-game/src/ripper_primary_turn.rs),
[`ripper_attack`](../crates/giants-game/src/ripper_attack.rs) and their
owned `Session` callers, including
[`ripper_calls`](../crates/giants-game/src/resident_states/ripper_calls.rs).
Its exact relationship to PC debug-1.1
`FUN_004d61f0` remains **unknown**. The PC callback and its class-specific
continuations remain independently acquired; the symbol does not transfer a PS2
signature, layout or implementation claim. The related shared launch bridge is
[`primary_enemy_fire`](../crates/giants-game/src/primary_enemy_fire.rs); its PC
helper at 0x004dbaf0 remains unassigned after prototype ripper/missile searches.
A real actor launch does not establish ordinary projectile flight or damage.
The class230 continuation adapters also lead to
[`cooldown`](../crates/giants-game/src/ripper_primary_turn/cooldown.rs) and
[`ripper_retreat`](../crates/giants-game/src/resident_states/ripper_retreat.rs):
independently checked PC timer/animation/marker contracts, not newly confirmed
prototype identities or complete combat delivery. Both binary identities are
pinned by the central method. Old address aliases remain valid.

`ripper_chase_location__FP6OBJECTP3P3Df` is another literal prototype search key
in [`ripper_steering`](../crates/giants-game/src/ripper_steering.rs). Its exact PC
correspondence is **unknown**. The separately acquired PC angle helper alias
`FUN_004dbd10` and the class230 primary caller remain the behavior references.

## Ripper injury search route

The literal prototype PS2 symbol `ripper_shot` is a search candidate for
[`state23_damage`](../crates/giants-game/src/state23_damage.rs) and
[`state23_hurt`](../crates/giants-game/src/state23_hurt.rs). Its relationship to
PC debug-1.1 `FUN_004dafd0` remains **unknown**. Existing callback controls do not
establish an ordinary partial-pistol hit adapter, lethal consumers, platform
layout equivalence or a confirmed prototype attribution.
The same unknown route now also leads to the scoped direct callback in
[`resident_states::ripper_damage`](../crates/giants-game/src/resident_states/ripper_damage.rs).
Its canonical partial publication and failure/replacement protection do not
establish an ordinary pistol hit, positive lethal services or a playable death.

The raw prototype symbol `ripper_death_logic__FP6OBJECT` leads to the acquired
corpse helper in [`ripper_corpse`](../crates/giants-game/src/ripper_corpse.rs).
Its correspondence to PC debug-1.1 `FUN_004d92a0` is **confirmed** from the
class-dependent mana parameters, ordered clock/creation/removal branches and
independent callee roles. See the central method's reviewed rationale. The
separate raw symbol `dead__FP6OBJECT` occurs at two prototype addresses and is
not a unique attribution. The PC helper's global countdown, conditional mana
request and removal mark are distinct from lethal damage, successful creation,
ordinary primary integration and canonical resource cleanup. No prototype
signature or layout is imported; both binary identities and old aliases remain
in the private record. This dependency does not establish a playable death.

## Ripper animation search route

The literal prototype PS2 name `ripper_animselect` is a navigation candidate
for `Session::update_state23_animation` and the separate ordinary ANM consumer
in `ripper_calls`. Its correspondence to the PC debug-1.1 secondary callback
at 0x004db670 remains **unknown**. Animation selection, clocks, reference/event
evaluation and successful ordinary frame completion are distinct contracts.

The child marker primary/secondary at PC debug-1.1 0x004dc540/0x004dc630
remain **unassigned**. The literal prototype PS2 symbols `mound_move`,
`mound_animselect` and `mound_shot` are additional search candidates from the
PC state's debug label. Their presence in the ELF is established, but the label
and names alone do not confirm any PC correspondence. Their related PC kernels
are in `surface_marker`, with owned ordinary phases and bounded retirement in
[`surface_frame`](../crates/giants-game/src/resident_states/surface_frame.rs).
Literal `object_destroy` and `object_free_all_allocated_data` remain **unknown**
search candidates for the separately acquired common cleanup/reference/data
release path at PC debug-1.1 0x0049de20/0x004a08d0/0x004a05d0.
Old PC address aliases remain valid. These names establish neither equivalent
signatures, prototype layouts nor complete gameplay availability.

## Attached effect preparation route

The literal prototype PS2 name `fx_create` is a search key for
[`attached_effect_context`](../crates/giants-game/src/attached_effect_context.rs).
Its correspondence to PC debug-1.1 `FUN_0044c0a0` is **unknown**. The acquired PC
extended attached preparation preserves the recipient's flag400 as context
flag10000. This establishes neither effect-object activation nor rendered FX.
The enemy-projectile mode8 preparation entry uses the same acquired PC wrapper:
mode8 selects positional input, separately from retained context flags. The
bounded original control supplies camera/quality inputs; canonical invariant
selection does not invent them. `fx_create` attribution remains **unknown**.

## Radial damage search route

The literal prototype PS2 symbols `hurt_object` and `hurt_object_do_multi`
remain **unknown** correspondence candidates for [`radial_damage`](../crates/giants-game/src/radial_damage.rs)
and its [`Session` adapter](../crates/giants-game/src/resident_states/radial_damage.rs).
The latter reads actual ordered radial registrations and acquired live health,
writes the owned Vimp signal and requires attributed hit consumers. A completed
name lookup or traversal calculation establishes neither a damage callback nor
ordinary explosion/combat availability. Original PC aliases remain in the private
identity-bound record; no prototype signature or layout is imported.
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

The literal prototype PS2 symbols `fx_setupUVScrolling__FP10FXSTACKTAGff` and
`fx_setupVerticleFade__FP10FXSTACKTAGfffff` are **unknown** attribution search
routes to [`effect_scene::RenderCommand`](../crates/giants-game/src/effect_scene.rs).
The PC construction ledger retains the actual source-ordered scroll/fade
operands as pending services, including repeated setup. One bounded source3
frame also has a canonical constructor in `Timed::scrolling_vertical_object`;
[`effect_render_parameters`](../crates/giants-game/src/effect_render_parameters.rs)
owns its UV/fade callback parameters and
[`vertical_fade`](../crates/giants-game/src/vertical_fade.rs) supplies its single
segment. Literal `fx_move` is an **unknown** callback search route here too.
The current parameters come from the same Session body; vertex-buffer use,
the other source bodies, delayed phase and whole mixed-program execution remain
unbound. No prototype signature or memory layout is imported.

`fx_processOpCodes__FP6FxDataiP6OBJECT` also remains an **unknown** search route
to `Timed::mixed_root_object` in
[`timed_effect_object`](../crates/giants-game/src/timed_effect_object.rs) and the
checked source-offset constructors in `effect_actor`. The acquired PC root
prefixes preserve their own positional, hidden and physical distinctions;
constructing a root does not execute children, consume waits or acknowledge the
whole program. Existing PC aliases remain unchanged.
Literal `fx_process_one` and `fx_logic` are additional **unknown** search routes
to [`mixed::Execution`](../crates/giants-game/src/attached_effect_context/mixed.rs).
This canonical source constructor owns root/child births, retained waits and
context retirement. The related
[`population::Population`](../crates/giants-game/src/attached_effect_context/mixed/population.rs)
retains those actual contexts across caller frames, selects body-local clocks and
owns separate cleanup over its Session cohort. Scoped controls do not establish
complete native scheduling, render/audio completion, damage, prototype
signatures/layouts or a confirmed attribution.
The literal prototype name `object_render__FP6OBJECTiG7f32quat` is an **unknown**
search route to the acquired mixed-body draw view in
[`timed_effect/draw`](../crates/giants-game/src/resident_states/timed_effect/draw.rs).
The shared [`smoke_particle_draw::Geometry`](../crates/giants-game/src/smoke_particle_draw.rs)
consumes that same prepared body's UV/vertical-alpha inputs. Construction alone
does not admit drawing, and this wiring does not establish native queue/pixel
fidelity or a prototype signature/layout correspondence.
The related `unlit_material::prepare_with_vertical_alpha` path uses a separately
pinned debug renderer module, SHA256
`ae000c1aa2e90a321b89c38833dd1e0c933689425a92f9b089691f4d92399318`.
Its shader-name lookup in the prototype PS2 returned no match; the relationship
remains **unknown**. The acquired PC gradient descriptor differs from a plain
material fade. Native state controls supply explicit capabilities and a recording
device, and do not establish live renderer selection or GPU pixel fidelity.

The literal prototype PS2 name `object_lock_logic` is an **unknown** search
route to the acquired PC debug-1.1 helper at0x00473b30 and the canonical
[`immediate` placement adapter](../crates/giants-game/src/resident_states/timed_effect/immediate.rs).
Its reviewed normal relative-scale branch uses the shared attachment math and
actual current references, clears the temporary binding once and retains the
next launch callback. Translation-only, absolute-scale and inherited-transparency
branches are not admitted by this adapter. Direct PC/helper controls do not
confirm a cross-platform attribution or the ordinary world schedule. No prototype
signature/layout is imported and prior PC identifiers remain retained.

## Mecc hurt search route

The literal prototype PS2 name `mecc_shot` leads to
[`mecc_hit_source`](../crates/giants-game/src/mecc_hit_source.rs) and
[`mecc_damage`](../crates/giants-game/src/mecc_damage.rs). Its relationship to the
independently traced debug-PC hurt callback at 0x0050c2a0 remains **unknown**.
The separate Rust `mecc_shot` module owns weapon firing; its name does not assert
this prototype identity. Source-gate and arithmetic controls do not supply full
attributed health/impulse/voice/death execution or ordinary combat availability.
The suppression snapshot has both console and temporary completion-countdown
writers; neither generic film activity nor a value without its lifecycle supplies those owners.
Prior PC aliases, raw names and uncertainty remain retained.

The same unknown route also leads to
[`mecc_enemy_hit`](../crates/giants-game/src/mecc_enemy_hit.rs), the offline
nonlethal class90/232 source-bearing kind0/2 composition of health, current
direction and pain/shared voice consumers. Its supplied-world original-PC
controls do not establish Session/controller binding, lethal services, shield/
gyro/critical-voice coverage or ordinary combat availability.
The canonical cold-player publication adapter is
[`resident_states::mecc_enemy_hit`](../crates/giants-game/src/resident_states/mecc_enemy_hit.rs).
It retains the same **unknown** prototype relationship and requires real world,
controller and audio consumers. Original constructor/control evidence is kept
private; no associated-player layout is inferred from a null relationship.
The literal `mecc_logic` is an **unknown** search route to the independently
acquired countdown fragment in
[`controlled_mecc_hit_clock`](../crates/giants-game/src/controlled_mecc_hit_clock.rs)
and its ordinary partial-controller clock binding. The PC shared initializer and
both original clock calls stay distinct from full primary/world scheduling;
their controls do not confirm a prototype function identity or ABI/layout.

The literal `hurt_object` search route also reaches
[`resident_states::enemy_object_contact`](../crates/giants-game/src/resident_states/enemy_object_contact.rs),
the independently acquired positive class90/232 object-hit wrapper and distinct
retirement receipt. This relationship remains **unknown**. Original-PC controls
of the direct dispatch and nonlethal receiver do not confirm a prototype function
identity or install ordinary world/Mecc reaction providers. Previous identifiers,
raw symbols and demangling uncertainty remain in the private lookup records.

These unknown routes also lead to the ordinary
[`rippers::combat`](../crates/giants-app/src/mission_runtime/rippers/combat.rs)
consumer wiring and its mode68 melee adapter. They borrow the current partial
Mecc health/velocity and real shared audio device. Shared-world guards remain
unavailable; this names no confirmed PS2 identity and establishes no rendered hit.

## Projectile expiry routes

The literal prototype PS2 names `projectile_logic` and `projectile_cleanup`
are **unknown** search routes to
[`resident_states::enemy_expiry`](../crates/giants-game/src/resident_states/enemy_expiry.rs).
The acquired PC expiry fragment requests sound before FX and conditional radial
work, then marks for its distinct retirement. Supplied-tail controls do not
confirm a PS2 routine identity, a whole primary or ordinary combat availability.
Prior PC aliases and raw-symbol/demangling uncertainty remain private.

## Declared platform query route

The literal prototype PS2 names `collgrid_make_grid` and
`collgrid_forcemake_grid` are **unknown** search routes to the independently
acquired debug-PC selector at0x0040fff0 and raster at0x00410f40. Find the related
Rust work in [`footprint_policy`](../crates/giants-game/src/footprint_policy.rs),
[`footprint_builder::platform_bits`](../crates/giants-game/src/footprint_builder.rs)
and [`declared_platform_scene`](../crates/giants-game/src/declared_platform_scene.rs).
The typed projection supplies only platform membership over actual prepared
geometry, not full footprints, neighbor mutations or registered world owners.
Exact-PC controls and the static Wife-world body lifetime control do not confirm
prototype function identities, signatures/layouts or ordinary combat delivery.
Prior PC aliases remain retained; detailed records and native controls stay private.

## Publication boundary

The literal prototype PS2 route `mission_fail` leads to
[`mission_failure::State::request`](../crates/giants-game/src/mission_failure.rs),
whose correspondence to the acquired PC failure producer remains **unknown**.
Signed-count gates, shared failure/time writes and the required failure-menu
boundary are independently acquired PC behavior. Private controls stop before
that menu's effects; this establishes neither its UI/restart owner nor live
world initialization, guards or ordinary Ripper combat. Earlier PC aliases remain
in the private identity-bound record; no PS2 signature/layout is imported.
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

The literal prototype PS2 name `mission_fail_flick__Fv` is an **unknown**
search route to [`mission_timer`](../crates/giants-game/src/mission_timer.rs).
The PC countdown and display-call gate retain separate clocks, selected-source
metadata and required failure/film/menu consumers. Original controlled slices
do not establish full display scheduling, live timer initialization, those
consumers' availability or ordinary Ripper injury. Prior aliases remain private;
no prototype signature or layout is imported.
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

This index publishes only selected names, reviewed navigation relationships and
attribution status. Binary/ELF inputs, raw tables, listings, matching outputs,
disassembly and detailed research remain private. The central method pins the
prototype identity and the PC identities used by reviewed attributions. Symbols
do not grant redistribution rights to those inputs or to game assets.
