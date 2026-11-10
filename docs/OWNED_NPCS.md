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
| Living Vimps | `vimp_actor`, `vimp_wander`, `vimp_physical`, `vimp_animation`, `vimp_frame`, `vimp_threat`, `vimp_response` | Owned birth/initialization/growth and admitted ordinary wander/physical/ANM composition; isolated threat consideration and required-provider response dispatch | Response vector/action bodies and actual input ownership, feeding, full-world order, live render and audio ownership |
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

## Threat consideration — 10 November 2026

`vimp_threat::consider` now preserves the observed signed-priority gates,
ordered/lazy nearby and peer reads, persistent signal flags, linked-presence
condition and remembered noise. Distance tests are strict and use XY; the noise
age limit includes exactly one second. The returned boolean reports new evidence
in that invocation, independently of existing flags. The noise level is stored
as a float before comparing the still-wide sum against its threshold.

Original-PC helper/distance execution matches 1,505 controlled cases /9,030
output/read-count words, including exact boundaries and a rounded-store threshold
case. The inputs are synthetic actor/payload fields, ordered lists, parameters
and noise under Double53. This is a helper comparison, not a complete primary,
source-world or live player-response comparison.

`Session::consider_vimp_threats_offline` resolves actual owned list order for an
initialized Vimp. It retains completed flags/noise writes on a late error and
quarantines that owner against replay. Two installed cohort controls check
once-per-noise memory, foreign/wrong invocation refusal, unsupported links and
late finite-profile failure. The latter tests Rust failure handling, not native
exception equivalence. Nonzero linked state still requires its owned producer.

The ordinary primary is not switched to this isolated entry. The response
dispatch below now preserves bounded selection. Its vector/route and action
bodies, linked/physical/session inputs, required audio/ANM services, caller timing
and actual world noise production remain open. No fleeing, AI mission admission or
visual availability is claimed. Prototype PS2 attributions remain unassigned.

## Response dispatch and session predicates — 10 November 2026

`vimp_response::respond` now preserves response priority, pending-flag consumption,
physical/channel entry gates, nearby/peer transitions, linked/noise target writes
and orientation. Required providers observe preceding stores. Each session query
supplies its current snapshot; the kernel executes `actor_session_predicates`
rather than accepting a supplied decision. The predicates preserve first-match
selection, signed count-minus-one fallback and low-byte actor-word comparison.
They do not produce live session or actor ownership.

Original-PC predicate wrappers and their actual callees match 2,840 contexts /
5,680 output words. Response controls match 1,024 contexts /28,672 projected
state/request/RNG/query words: 927 run to the helper return, 75 stop before the
required vector/route callee and 22 before the required action callee. Those
97 stopped entries are prefix evidence, not completed responses. No callee return
is substituted. Byte-only target requests remain distinct from nearby float/word
requests. The random interval retains a wide subtraction, and equal bounds still
consume a draw. Source TLS seeds and session/actor/physics/channel/affine/target/
noise fields are supplied; this is not a live ownership or full-primary proof.

Rust controls retain preceding priority/target/delay/RNG stores after required
provider failures. The vector/route and action implementations remain required;
no successful default is installed. The ordinary Session/primary is not switched
to this service. It still needs actual session snapshot and linked identities,
owned physical-flag provenance, vector/route and action providers, mode7
continuation, sound/ANM and precise caller timing. The current Vimp physical view
does not yet expose the entry flag word; it must not be guessed from a successful
CPU movement phase. No ordinary fleeing, rendering or gameplay download follows
from this milestone. Prototype PS2 attributions remain unknown.

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

The herd/marker Rust common-node dispatcher, outer world admission, clock/RNG
ownership and active FX/audio still need binding before ordinary mission AI.
Remaining tasks stay in `BACKLOG.md`.

### Owned herd/marker common callbacks — 10 November 2026

`Session::prepare_herd_world_history` and
`Session::advance_herd_world_after_history` now compose acquired class49/state17
herds and class679/state0 markers with the common node policy. Source construction
owns their nonphysical motion/finalization state. The entry retains primary
rate history, the acquired absent state0 primary and state0/state17 secondary
callbacks, object-local time, empty-channel ANM epoch and finalization. The
herd's birth clock counts primary calls separately from common frame numbers.
Required environment/footprint providers are explicit; no terrain is fabricated.
Empty channels and references are a checked profile, not a general ANM adapter.

A composed Rust `Session::walk_common` control now matches the original mutable
walk across 32 declared Story2 scenarios /96 snapshots /824,168 normalized words.
It includes 2,880 node visits, 2,432 prior history preparations and 448 newborn
first callbacks in their birth steps. It compares owned headers, payloads,
references, ordered lists, birth-height requests, prepared contexts and RNG.
The global ANM epoch is shared with Vimps, not restarted per marker. Separate
original ANM controls match 128 cases /4,864 words, covering signed zero,
positive time, force masks and epoch wrap with four empty channels.

Four installed controls additionally cover a three-step 30-node traversal,
constructor histories, current affine publication, cancellation, wrong/foreign
entry and invalid-clock preflight. A supplied sleep/wake gate additionally
keeps skipped primaries separate from common frame counts. A late environment
failure retains the fourteen births, preceding ANM epoch and partial environment write; actor and
list tokens prevent replay. Loss of the controlled actor stops after birth
without acknowledging fallback; callers must stop the whole walk on that
outcome. These fault controls establish owned failure
handling, not native exception equivalence.

Cohort/control identity, clock, quiet noise, flat terrain, empty collision and
platform cohorts, TLS seed and absent sound bank remain supplied. This binds
bounded Rust composition, not outer world/global services, native live RNG
ownership, ordinary mission AI, active effects/audio or rendering. The next work
is the actual admission/phase producers and required behavioral services in
`BACKLOG.md`. Prototype PS2 counterparts remain unassigned.

### Mutable common-list acquisition — 10 November 2026

A separate original-PC-only control now executes the actual mutable callback
walk, after a supplied active-node history pass. Across 32 Story2 scenarios and
three steps each, it records 2,880 node visits and 2,432 history preparations.
All 448 newborn Vimps are visited in their birth step without an earlier history
copy. The successor is read after the callback; a snapshot of the pre-birth
population would omit these visits.

Herd parents and basic markers also execute their common secondary dispatch,
local time, ANM and finalization paths. The acquired PC state0 primary and
secondary entries are absent; state17 has the acquired herd primary and no
secondary entry. That establishes these bounded dispatch absences, not permission
to skip the common fallback. Source-marker controls still write their floor,
animation epoch and affine cache. Herd finalization additionally calls the real
environment service. The bounded Rust binding above now retains these dependencies; ordinary
Ripper-only finalization is not an interchangeable adapter.

This acquisition retains the declared cohort, nonnull controlled herd reference,
TLS seed, cadence, quiet noise, flat world and absent sound bank. It excludes the
intervening global services, outer admission, actual player and rendering. Its
96 recorded snapshots are native observations, **not a Rust comparison**. Keep
them separate from the validated newborn comparisons above. Prototype PS2
wrapper/dispatch lookups remain unassigned; no ABI or types are imported.

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
