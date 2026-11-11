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

See [MEAT_COLLECTION.md](MEAT_COLLECTION.md) for independent evidence, version
differences, normal integration and the explicit disabled-objective boundary.

## Publication boundary

This index publishes only selected names, reviewed navigation relationships and
attribution status. Binary/ELF inputs, raw tables, listings, matching outputs,
disassembly and detailed research remain private. The central method pins the
prototype identity and the PC identities used by reviewed attributions. Symbols
do not grant redistribution rights to those inputs or to game assets.
