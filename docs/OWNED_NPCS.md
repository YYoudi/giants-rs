# Non-player actors and player interactions

The NPC lane resumes the existing source-backed services rather than starting a
parallel AI framework. `BACKLOG.md` remains the detailed backlog. This inventory
identifies implementation boundaries; it is not a claim that these inhabitants
already run in normal missions. Character-player controllers remain in their own
Mecc, Delphi and Kabuto lanes, and cinematic direction remains in the Flick lane.

## Existing reconstruction

| Family | Existing Rust services | Established boundary | Missing ordinary gameplay connection |
| --- | --- | --- | --- |
| Herd population | `herd_generator`, `herd_markers`, `declared_residents` | Source herd/marker construction, authored properties, ordered marker allocation, offline clock and Vimp births | Mission scheduler, native live clock/RNG ownership, complete population and retirement |
| Living Vimps | `vimp_actor`, `vimp_wander`, `vimp_physical`, `vimp_animation`, `vimp_frame` | Owned birth/initialization/growth and admitted ordinary wander/physical/ANM composition | Threat and feeding providers, full-world order, live render and audio ownership |
| Vimp injury and remains | `vimp_damage`, `vimp_hurt`, `vimp_recovery`, `vimp_living_scalars`, `vimp_chunks`, `vimp_death_proxy` | Bounded damage, hurt/recovery and owned chunk/proxy services | Actual player hit dispatch, active effects/sounds, collection/economy and complete death cleanup |
| Rippers | `ripper_perception`, `ripper_steering`, `ripper_pursuit`, `ripper_primary_turn`, `ripper_attack`, `ripper_awakening`, `ripper_emergence`, `ripper_turn_completion`, `ripper_ground`, `ripper_dirt` | Ordered target selection, supplied-view behavior kernels and persistent Session adapters for acquired branches | Complete actor/world primary scheduling, required firing/effect/audio providers, remaining modes and ordinary mission admission |
| Shared enemy response | `state23_entry`, `state23_damage`, `state23_hurt`, `state23_animation`, `enemy_projectile`, `enemy_ballistic`, `enemy_flight` | Acquired state23 entry/damage/animation, enemy projectile birth/launch and isolated ordinary flight arithmetic | Complete enemy projectile world/contact/damage composition and all enemy classes/bosses |
| Smartie handling | `smartie_carry`, `smartie_drop`, `carried_actor` and Session pickup/release services | Acquired Mecc carry/detach and owned relationship services | Complete Smartie initialization, independent AI, rescue/follow/work/fear, speech and worker economy |
| Other interactive inhabitants | Shared actor, targeting, physics and damage services | Reusable infrastructure only | Other fauna, allied squad decisions/orders, faction behavior, buildings/shops/vehicles and scripted inhabitants require separate acquisition |

`mission_runtime` constructs and binds supported authored declarations, including
herd generators and markers. That construction does not execute their primary
callbacks or populate a live herd. `vimp_frame` explicitly requires world/history
admission and required services. It must not be enabled by fabricating sound-stop,
feeding or threat callbacks. Ripper perception uses ordered current views and
validated health/ownership; a synthetic candidate array does not prove that the
complete live contact population is available.

## Evidence and reuse

Read the previous private research archive in place. Its source controls for
herd birth/markers, Vimp lifecycle, Ripper perception/primary sequences and
Smartie carry/drop are investigation aids, not public fixtures or an alternative
runtime branch. Inspect individual services and their evidence before adapting
anything. Retain supplied boundaries, rejected cases and previous identifiers.
Keep raw listings, control transcripts, source snapshots and native binaries out
of Git; publish only reviewed independent code and minimal validation summaries.

Follow [RECONSTRUCTION_METHOD.md](RECONSTRUCTION_METHOD.md) before revisiting a
native function: pin the actual PC binary, search prototype PS2 names, retain raw
symbols separately from ambiguous demangling and keep attribution confidence
separate from behavior/runtime validation. Initial herd-primary and Vimp-primary
lookups remain unassigned; matching vocabulary alone establishes no attribution.

## Source-resource regression

### First callback after a herd birth — 10 November 2026

`Session::advance_vimp_newborn_world` now consumes a real owned herd birth's
first common-list callback. It retains the constructor's position histories,
initializes the actor and continues into living movement/physics/ANM inside
that same callback and shared RNG stream. A separate initialization call or an
extra world-history copy would change that composition. Warm actors continue
through the separately prepared or combined routes below.

This entry requires a cold, active, unparented class8 herd child at local frame
zero. The caller must establish that this birth followed the current world-history
prepass; cold metadata alone does not establish that ordering.
Parentless allocations, already initialized actors, wrong/foreign frames
and route replay refuse. Initialization and completed physical work survive a
later required effect-namespace failure; the failed world token forbids replay.
The entry does not fabricate sound/effect providers or bypass outer world gates.

Original-PC constructor/birth/first-primary controls match 32 scenarios and 96
snapshots / 824,168 normalized words, including 448 first callbacks. The declared
Story2 cohort uses authored markers and original/supplied kind1 parent selectors,
16 TLS seeds each, positive supplied cadence, quiet noise, flat terrain and a
supplied nonnull control reference. Normalized headers, payloads, references,
ordered lists, prepared effect contexts and RNG are compared. Later two callbacks
use the existing per-node history boundary. This is neither a complete world
traversal nor live process-seed, sound transport or rendered fidelity evidence.

Two installed-source controls additionally cover a 14-child cohort through its
first callback and two separately prepared continuations, constructor-history
preservation, cold/initialized route separation, inactive/foreign/parentless
refusal and retained late failure without repeated draws or services. These
controlled source checks do not establish complete world admission or native
error handling. All detailed controls remain private; prototype PS2
correspondence is unassigned.

The full herd/marker common-node dispatcher, outer world admission, clock/RNG
ownership and active FX/audio still need binding before ordinary mission AI.
Remaining tasks stay in `BACKLOG.md`.

### Separate Vimp world-history entry — 10 November 2026

The normal PC world path prepares active actors' position histories before its
primary callback traversal. The common traversal reads each successor after
the callback. An actor born later in that traversal must not be assumed to have
received the earlier history pass. Outer world gates and intervening global
services remain required; recovering this ordering does not implement them.

`Session::prepare_vimp_world_history` now seals an initialized, active owned
Vimp's history for its current local frame. It advances no clock, AI, physics,
animation or audio. `advance_vimp_world_after_history` consumes that preparation
without copying the history again. Wrong/foreign frame identities, inactive
preparation, repeated preparation and mixing the split and combined routes are
refused. A missing preflight service keeps the pending history; an attempted
callback consumes it and retains its failed iteration against replay. Existing
`advance_vimp_world_offline` remains the combined inspection route.

The pure position-history operation is shared with ordinary actor finalization.
Actual original-PC execution of the helper and its real return tail matches
256 supplied header cases / 2,304 position/history words in Rust, including
signed zero, infinities and NaN payloads. The native control separately checks
49,408 header words for preservation outside the written history fields; that
is not a complete-header Rust comparison. These are instruction-level controls,
without world startup, live clock/RNG or callbacks.

Three installed-resource controls cover two separately prepared actors, a
position change between phases, a late terrain failure with retained history,
replay/foreign/inactive refusal and a missing required recovery-sound provider.
An eight-frame source-backed prefix matches the existing combined route for
position histories, channels, configuration, payload, RNG and ANM epoch using
supplied flat terrain, time and seed. It is a Rust composition regression, not
an original-PC comparison of eight complete frames. The controlled actor
identity is supplied; no complete world or ordinary mission is admitted.

The newborn route above supplies the separately acquired first-callback
composition. Actual world admission, clock/RNG ownership, the complete cohort
and required threat/feeding/effect/audio services remain unbound. The next tasks
are retained in `BACKLOG.md`. Prototype PS2 attribution remains unassigned.

### Authored herd births

The optional `source_npc_herd_births_keep_ownership_and_quarantine_failures` test
loads authored world herd/marker placements from a user-supplied installation.
It keeps their values and order, composes source Vimp construction and first
initialization, checks parent/session identity, repeated-initialization refusal,
terrain-failure quarantine, non-replayed RNG/service calls and Session cleanup.
The flat floor, timestep and random seed are supplied test inputs. It executes
no original program and proves neither full living AI nor visual fidelity.
See [BUILD.md](BUILD.md) for its command and required local configuration.

The first installed-data run examines 28 authored parents, initializes 313
children and checks 26 injected terrain failures. Two parents become inactive
without eligible markers in this world-only fixture; they must not be scheduled
again as active herds in that fixture. Mission markers are not loaded here.
No other configured-parent profile was excluded in this run. Two source world
files are refused by the current parser and remain outside this control's
coverage. These counts depend on the installation and do not certify all worlds,
native whole-frame behavior or the original executable's current RNG stream.

Runtime integration must retain real player-target identities, common physical
and ANM ordering, actual input/contact/damage producers, resource ownership and
cancellation/re-entry. A rendered diagnostic animal is insufficient to establish
interactive NPC behavior. Future handoffs must retain these boundaries and the
next required native-name lookup.
