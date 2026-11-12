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

The literal prototype symbols `fx_logic` and
`fx_processOpCodes__FP6FxDataiP6OBJECT` lead to the related work in
[`effect_ground_emission`](../crates/giants-game/src/effect_ground_emission.rs)
and the source constructor in `effect_actor::Template::ground_positioned`.
Both relationships remain **unknown**. The acquired PC contact subset decides
whether to request a child and retire its parent; source names do not validate
the physical query, child factory, whole callback or film integration. The
source constructor retains its mutable budget in the canonical actor payload.

## Publication boundary

This index publishes only selected names, reviewed navigation relationships and
attribution status. Binary/ELF inputs, raw tables, listings, matching outputs,
disassembly and detailed research remain private. The central method pins the
prototype identity and the PC identities used by reviewed attributions. Symbols
do not grant redistribution rights to those inputs or to game assets.
