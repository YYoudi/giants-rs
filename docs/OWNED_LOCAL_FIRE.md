# Owned local pistol callback

Select and resume work under the shared [gameplay-first
objective](GOAL_GAMEPLAY_FIRST.md). Prioritize the ordinary player-visible path and
its first missing caller/consumer before rare variants. Dated controls and
next-acquisition notes below retain their evidence scope; they do not override the
current delivery priority in `BACKLOG.md`. A handoff must identify the exact
testable build and normal controls, or state "intermediate dependency only" and the
next runtime blocker.

Further acquisition and implementation follow [RECONSTRUCTION_METHOD.md](RECONSTRUCTION_METHOD.md),
including the mandatory prototype PS2 name lookup. Existing PC evidence and
aliases retain their scope; a recovered name does not validate fire scheduling.
The central method's current-function follow-up records the primary, pre/post,
selector, placement and getter searches separately: all seven PS2 attributions
remain unknown despite the PC/Rust controls below.

## Primary versus subjective dispatch

The optional primary API now requires an explicit `first_person::Display` status:
the actual camera owner and whether the pre-world subjective representation is
active. This status is independent of `FireView::first_person`. A matching active
display means the native primary defers firing to its subjective post-world
phase. That phase is not bound by the immediate primary API, so its preflight
refuses this route before physical/ANM, actor-payload, FX or RNG updates. A foreign
mission display binding is also refused. This is an unsupported-phase guard,
not a completed world scheduler. The explicit deferred route below now composes
the acquired post phase; the application still supplies `fire: None`.

The dispatch predicate matches 512 controlled cases against the full original PC
helper and camera-owner getter, plus the prototype PS2 comparison body. Cases
include differing/null camera owners, nonzero active-flag values and camera-mode
changes. Valid source identities are nonzero; the PS2 comparison supplies the
getter result and translates two integer return moves within the admitted
32-bit domain. It excludes the EE entry/exit ABI and complete PS2 execution.

Private native pre/post-world observations and actual primary-API preflight
checks are recorded in [RELEASES.md](RELEASES.md). The owned subjective composition
below now produces its acquired channel/header/placement/ANM subset. Remaining
world binding must provide real camera timing, vertex/rendering integration and
audio with native ordering retained.
The existing presentation-only subjective player is not a substitute for those
services. A post-primary camera cache cannot supply an unacquired earlier view.

## Deferred primary and actual post-world fire

The produced-camera entry `Session::advance_mecc_subjective_post_produced` now
executes the inner camera between selection and placement, and passes its actual
camera/cache to local fire. The world camera owner retains separate inner/outer
invocation numbers and the prior completed globals. Four controls preserve all
320 frames and twelve shots with generated camera values; a further 64-frame
capture checks the inner view upload. See [OWNED_CAMERA.md](OWNED_CAMERA.md) for
word counts, startup/provider boundaries and the remaining runtime prerequisites.
The supplied-camera entry remains an explicitly separate numerical control.

`Session::advance_mecc_subjective_world` requires an actual Session pre-copy for
the matching source, resource and frame. Generic supplied-field preparation is
insufficient. It executes the ordinary world/primary/secondary path with firing
commands admitted, while skipping the local fire callback and its auxiliary reset
at the body boundary. Native sentinel observations independently distinguish this
preservation from the non-firing profile's reset. Completed bodies retain a post
obligation that blocks another history/primary until it is consumed or cleared.

After the actual held-pose producer has finished the same frame,
`Session::advance_mecc_subjective_post` derives current flags, selected pistol,
authored icon family/offset and object time from owned state/resources. It runs
virtual selection/placement and first ANM, publishes shared references, executes
the real local target/fire/projectile/effect callback, then runs zero-time ANM.
Its muzzle adapter reads the **current** held world cache; the immediate primary
continues to read the prior completed cache. Virtual reference writes never
reattach/recompute that frozen held cache from the newly overwritten body store.
The generic selector now consumes current model rates1/2 and angular rate; the
ANM updater retains all four current rates. Earlier supplied-header controls did
not establish the Session adapter's rate mapping.

Four continuous native/Rust comparisons pass 320 frames / 144 active subjective
updates / 12 shots (8 post-world, 4 immediate) / 391,552 normalized snapshot words.
They include stationary turning, view/resource transitions and held/pulsed input,
and compare complete source header/payload, acquired virtual fields, shared and
held reference caches, projectile construction, FX contexts, RNG and epochs.
Both ordinary and deferred fire execute in Rust. Camera production/eligibility,
initial camera ownership and the partial Mecc-then-held world population/order
remain supplied; the positional sound registry is explicitly empty. This proves
neither a complete mission scheduler, targeted subjective encounters nor visual
or audible fidelity. The original 576-frame / 22-shot primary matrix and its ten
failure controls still pass independently.

A source-backed lifecycle control covers preserved nonzero auxiliary fields and
clocks, genuine preparation, required current held pass, next-frame blocking,
successful/no-shot callbacks and late sound failure. The latter retains the real
born projectile, named FX, RNG and first virtual ANM; failed or completed posts
cannot replay. Cancellation invalidates the source identities. See BUILD.md.
Main admission still needs enclosing-world binding of the acquired camera and
position producers, subjective/body drawing policy, complete scene target
contexts and positional audio. Existing previews do not expose this owned fire
route. Prototype PS2 names remain unassigned for the pre/post/selector/primary
correspondences; no platform ABI or layout is inferred from candidate names.

## Shared subjective reference storage

### Owned subjective model positions - 6 November 2026

`Session::prepare_mecc_subjective_positions` now updates the existing shared,
model-resource-keyed position bank from the completed virtual channels and actual
virtual animation stamp. It uses the Session's prepared state specification,
without an independent presentation animation player or another position bank.
The matching position/cache getters retain source, frame and resource checks.
They expose the latest shared bank contents, not an immutable actor-owned pose.
Callers must prepare immediately before consuming or uploading a model; another
actor using the same model resource can replace that buffer through its own pass.
Skipped, inactive, incomplete, failed, foreign and cleared sources cannot publish
geometry through this entry. A late fire failure preserves earlier bank writes;
refusing a new geometry request does not erase those completed effects.

Native instruction/call inspection identifies the position buffer and its cache
on the model resource. These observations do not identify the copied auxiliary
header pointer as a position buffer. Its cleanup ownership test is separate; its full
semantic role and draw-time consumers remain unassigned. No pointer is recast
as a vertex buffer from address similarity or the header copy alone.

Two continuous controls pass 64 frames, 544 body/subjective model snapshots and
716,414 normalized words, including 641,376 vertex/cache words. They compare all
eight declared body model slots and the active subjective model, alongside source,
held/virtual references, cameras, actual fire/projectile/FX, RNG and epochs. One
control supplies forced recalculation; the other mixes forced and normal cache
gates. These are supplied slot/phase controls, not acquired live LOD/visibility
admission. Their 32-frame motion schedule also exposed a missing backward-air
secondary transition. Its acquired normal-pistol logical state now selects the
native backward-air clip; the original failing sequence completes unchanged.
Other directional-air variants and reaction providers remain separately refused.

Source-backed lifecycle checks additionally cover pre-post refusal, skipped
camera ownership, cached/forced repeats, unchanged invalid body-reference binding,
foreign/frame mismatch, late failure and clearing. Prototype PS2 lookups precede
the new acquisitions; their attributions remain unknown.
Geometry permission is acknowledged by the actual Session post. A generic numeric
post following a genuine pre-copy cannot certify that publication, even when its
local math reaches the finished phase; a source-backed bypass control verifies this.

The main driver's `fire: None` remains. Live model-slot/force admission, part masks,
materials and binding the acquired subjective/body submission policy, complete target contexts
and positional audio remain prerequisites. CPU positions are not a rendered
subjective scene, a complete mission or verified fidelity.

### Mecc body submission and held-parent pose policy - 6 November 2026

`mecc_draw_policy::submit` acquires the class-2 body/subjective dispatcher.
Inactive subjective display submits the body, with its separately gated ordinary
prelude. Active display preserves a non-controlled Mecc's parent suppression;
for the controlled Mecc it either suppresses submission or prepares/submits the
subjective actor, followed by the optional auxiliary actor. Camera mode alone
cannot select this route. Other character classes are refused before callbacks.

`mecc_draw_policy::update_held_pose` preserves a separate native rule: an attached object's camera-owner
query occurs even if subjective display is inactive or inhibited. Only an eligible
owner redirects its parent to the subjective actor during pose update. A detached
parent remains absent afterward; a remaining parent, including a replacement
made by the provider, is restored to the original identity. Rust provider failure
retains completed mutations and requires caller quarantine rather than replay.

The controls match 8,192 original-PC class-2 dispatches and 144 held-parent
dispatches, including strict height boundaries, NaN/infinity, parent-class/control
combinations, owner mismatch, inhibition and kept/detached/replaced parents.
The native class/flag predicates execute; prelude, preparation and model-submission
callbacks are intercepted rather than executed. The held owner getter and its
pose-update outcomes are supplied. Two synthetic checks separately verify other
class refusal and retained effects on provider failures. These are CPU contracts,
not a new live renderer, world admission, completed Session post or fidelity proof.

Further provider inspection corrects the earlier submission terminology: this
held wrapper updates an attachment transform and position/history, rather than
submitting graphics. The old `submit_held` entry remains a deprecated historical
alias; its 144 controls prove parent redirection, not rendering.

`mecc_wield_pose::sample_subjective` now projects the normal pistol's matrix from
the genuine completed virtual post, its affine/scale and the authoritative shared
reference allocation, plus the actual child's current handle reference. It reuses
ordinary attachment composition and advances no ANM. The child's real binding and
the frozen world cache used by post-world firing remain unchanged. This projects
a pose only: inherited opacity, position/history and spatial finalization are not
applied. A matching numeric post cannot authorize it; missing, skipped, failed,
stale, foreign, cleared or subsequently replaced shared references are refused.

Two controls span 64 source frames and compare 32 subjective held matrices
(544 words) against the original wrapper and normal attachment calculation over
captured post/reference snapshots. The position/history/spatial tail is intercepted.
This is separate from native whole-world execution and a live GPU upload. The
four installed-data lifecycle checks also exercise the new ownership boundaries,
unchanged firing cache and replaced-reference refusal.

Native call inspection also distinguishes the actual model-submission entry from
the previously examined position/draw helper. The submission entry routes parts
through immediate and deferred work; reaching it does not certify a GPU draw.
Reuse `render_parts`, `render_sort`, `lod` and the existing shared position bank
when acquiring its live callers and preparation, rather than adding a parallel
animation player or inferring material admission from the dispatcher. These
acquisitions followed prototype PS2 searches; their attributions remain unknown.

Next bind the dispatcher to the completed owned post and actual held identity,
acquire preparation and the selected immediate/deferred model consumers, and
retain the menu-to-world camera-startup, target-context and positional-audio
prerequisites. Main `fire: None` and the existing presentation route remain.

The native pre-copy shares both the Mecc state storage and the reference-matrix
buffer with the controlled source, while preserving separate subjective channels.
Treating the subjective actor as an independent state/reference copy loses these
observable writes. Posed-vertex storage and complete actor/header lifecycle
still need their own acquired integration; reference sharing alone is insufficient.

## Persistent subjective composition

`mecc_subjective::State` now preserves four virtual channels across pre/post-world
updates and inactive periods. The pre-copy retains those channels, copies acquired
source fields, motion, input buttons and ANM stamp, and holds the source identity
for shared state. It does not clone the source payload. World-shared weapon-change
history and initial channels are explicit lifecycle inputs, rather than invented
per-actor defaults. Actual pre-world camera/model eligibility remains external.

The post phase reads current controlled-player flags, selects a channel, places
the virtual header from supplied previous/current cameras and authored offsets,
then updates its affine. It advances ANM at object time and publishes into the
source's authoritative reference store before invoking the required fire service.
The acquired inhibition flag skips that fire call. A second, zero-time ANM pass
follows. Missing descriptors retain the native skip behavior, including dummy
backup channels; there is no fabricated clip or default successful fire service.
This profile requires genuinely prepared event-free ordinary Mecc resources.

`State::for_session` binds the actual typed subjective specification;
`prepare_from_session` copies pre-primary fields/rates and supplied current input.
`Session::update_mecc_subjective_references` checks the owner ticket, ordinary
profile and exact bound specification identity before writing shared matrices.
Normalized pointer words never imply a missing typed resource. The service borrow
ends before fire, allowing the actual source payload and services to remain owned
by the Session. Late errors retain completed animation/publication/service effects
and quarantine this state against replay. Invalid time is rejected before execution.

Two continuous private native/Rust comparisons pass 128 frames, including 64
active subjective updates and 42,880 acquired header/reference words. They cover
view changes, missing/present subjective resources, neutral/walk/side/back motion,
turning and held/released trigger pulses. Rust produces the persistent virtual
channels rather than receiving their completed native values. Initial camera-owner
binding, completed body channels, camera production and selector-getter results
remain supplied. The fire boundary explicitly supplies no virtual-channel edits;
native observations verify that boundary for this cohort. This is not an actual
Rust fire callback comparison, complete world scheduler or visual-fidelity result.

A source-backed lifecycle control covers exact resource ownership, foreign tickets,
invalid time, reference/fire failures, retained source writes, inhibited firing,
inactive channel retention and cancellation. It runs silently using a supplied
installation; see [BUILD.md](BUILD.md). The primary's active-display refusal and
the application's `fire: None` remain. The new deferred API supplies its own
obligation and actual normal-pistol callback, but inner/outer camera production,
posed-vertex/render consumers, complete targeting headers and positional audio
still need coherent main-driver binding. Existing presentation playback
does not satisfy these prerequisites. Prototype PS2 lookups for the pre/post and
selector routines remain unassigned; this module imports no PS2 signature/layout.

## Shared reference evaluation controls

`actor_references::Owned::update_shared_current` now evaluates a context's
specification into the authoritative source matrix storage. Rust evaluation
caches remain separate, but undeclared slots start from the latest shared values.
The source's previous body-channel sample binding is invalidated after another
context writes its matrices. The stored values remain available without claiming
that they were produced by the source body's own channels. The caller must use
the authoritative storage between evaluations, rather than an old context-local
snapshot. Neither publication changes on a failed evaluation.

Two continuous reference controls match 96 frames and 26,112 words, including
48 active subjective updates. They cover missing/present subjective resources,
view-mode transitions and neutral/walking/side/back motion. Native pointer and
buffer-content observations establish the sharing; Rust calculates the matrices
from the actual ANMs and supplied completed channels. Channel production/timing,
whole-world scheduling, events and firing are not proved by this comparison.

Four failure controls cover zero epochs with present/absent primaries, a nonfinite
clock and mismatched reference counts. A fifth checks that an absent primary with
a valid epoch preserves both publications. The older body binding is not
recertified by those operations, and the subjective binding remains retrievable.

The selector's flag input is now named `controlled_flags`: the native callback
reads current controlled-player flags after its primary, rather than the virtual
header's earlier copy. Its previous-weapon history belongs to the world selector,
not to a cloned Mecc payload. The presentation-only playback's explicit held-input
mapping remains a preview policy, not that native producer. The selector's PS2
name remains unassigned after the required lookup; no signature or layout is
imported from a candidate name.

`mecc_fire_primary::advance_local` now runs the actual Session local scan between
fire-clock publication and the normal pistol firing branch. It uses the same
source-backed projectile definition as the shot factory. A caller cannot supply
a ready-made selected target in place of that query.

The scan publishes its three typed target links and aim point. A selected actor
uses that point and its owned tracking identity in the existing shot transaction;
a miss still requires the independent normal aim service. Current muzzle reads,
constructor effect preparation, launch, source attribution, sound request,
cooldown, base-pistol consumption and common tail keep their existing providers.
The legacy nonlocal callback continues to refuse an owned selected target.

## View and reticle semantics

The terrain prepass uses the supplied camera eye and angles. The selected-target
range comparison instead uses the current viewport origin, also used by the
projectile/view-origin service. Local fire refuses mismatching origins before
publishing any frame state. Neither origin is inferred from the rendered camera
after the primary, or from a cached muzzle point.

The range comparison is strictly less than the declaration range, with the
acquired Double53 squared-distance ordering. Effective normal-pistol declaration
range remains 700; the independent shot caller distance remains 500. The target
range flag requires a selected actor with nonzero state. It is an advisory
reticle flag, not a new firing gate: an out-of-range selection can still reach
the shot request when the existing firing conditions permit it.

The selected out-of-range path initially preserves auxiliary timer words. The
inactive-trigger cleanup resets them later; a missing held object returns before
that cleanup. This distinction is covered with nonzero controlled sentinel
values, including cases where no projectile is created.

`Output::local_scan` exposes the actual query result and visits, alongside the
shot and target-range decision. Session cancellation clears the range owner.
Missing required headers/geometry fail the callback and quarantine its source;
the caller cannot replay the partially published frame as a fresh fire attempt.

## Evidence and remaining integration — 5 November 2026

The controlled full native/Rust local callback comparison matches all 64 cases /
16,890 words. It includes 53 selected targets, 32 in-range target flags, 21
out-of-range selections and 18 shots, of which 15 use tracking identities.
The complete source payload/control fields, normalized projectile header and
payload, query counts, reference reads, RNG, shot record and actual named effect
preparation are compared. The positional sound registry is explicitly empty;
no audio device or active effect playback is certified by this CPU control.

The legacy callback also passes 64 native/Rust regressions / 16,570 words,
including 18 untracked shots. All 696 synthetic Rust tests pass (14 ignored),
with 20 Python tests, Rustfmt and Clippy after the main integration. These checks
remain separate from live mission input, complete primary scheduling and rendered
fidelity.

Eight additional owned-backend checks cover origin disagreement before mutation
and missing-header failures with quarantine/no replay. These are refusal checks,
not parity claims for native error paths. Original data, listings, private probe
inputs and reports remain excluded from the public repository.

At the standalone callback boundary, physical/first-animation completion,
camera/viewport snapshots, cache epoch and time are supplied. The optional owned
primary integration below now produces part of this boundary. The main mission
driver still does not execute it. This does not add a playable weapon to the
downloadable preview or establish visual fidelity.

## Optional owned primary phase

`mecc_primary::Services.fire` now connects the real local callback after physical
forces and the first ANM, before jet, secondary and the second ANM. The primary
publishes its actual fields, payload, channels and references, uses its local
object time and shares its constructor-effect pool and RNG. It does not evaluate
an extra animation pass to create a muzzle sample. The caller still supplies
the actual camera/viewport, cache epoch, prior held-world muzzle and audio.

A continuous 64-frame native/Rust walking control with pulsed trigger input
matches all 25,724 normalized words: full player header/payload, both projectile
headers/payloads and all 64 held-world matrices/reference caches. RNG and shot
decisions also match. The initial frame has no held pistol; secondary creates
it, and the next frame can fire from its prior completed cache. This acquisition
executes a supplied partial Mecc-then-held cohort, not full native world order;
projectile world updates are excluded. The camera boundary is supplied and the
positional sound registry is explicitly empty. This is CPU evidence.

An extended control also compares all 5,808 words of active deferred constructor
contexts over these frames: 31,532 normalized words in total, with no differences.
The actual recipients remain bound to the two produced projectiles. This does
not execute those contexts' effects or certify their later rendered behavior.

The control exposed a missing armed-walk transition while an ordinary walk was
still active. The held-trigger timer now selects that acquired pistol clip.
Synthetic regressions cover this transition. The later controls below extend
the normal pistol's standing, side and turn selections.

Two additional backend-failure controls verify missing muzzle and late sound
failure, primary/shot quarantine and no replay. The latter retains its allocated
projectile, constructor context and consumed RNG. These checks establish owned
error behavior, not parity with native error paths. Private REA selector evidence:
`ev_21602305947167bb2bc69a4addfa617054d1154db3074740a845ced4112a8e77`.

The application still supplies `fire: None`. Actual main-driver view/audio
producers, complete actor admission and mission context coverage, the class805
special header, projectile world updates and the remaining secondary reactions
are prerequisites in BACKLOG.md.

Validation for this primary-phase change: 698 synthetic Rust tests pass (14
ignored), 20 Python tests pass, with Rustfmt, Clippy and exported-site JavaScript
checks. No additional playable capability is claimed by these checks.

## Normal pistol armed secondary transitions

The owned phase now supports a fresh standing shot, its held firing animation
and the subsequent aiming idle, plus armed side steps and stationary turns.
Standing shots force a restart even when the firing clip is already selected,
with zero blend time. Their control flag is admitted only with the optional fire
profile. A released trigger does not interrupt the active firing channel; the
channel-end transition selects the aiming idle. Side and turn playback keep
their actual model/angular rate sources. No extra RNG is introduced.

Five continuous native/Rust controls pass on the same executable: 128 frames
each standing, strafing, turning and mixing movement directions, plus the prior
64-frame walking/pulsed-trigger regression. In total, 576 frames, 22 shots and
341,220 normalized words match. This includes full player/projectile storage,
prior held-world matrices/references and deferred FX context chains; RNG and
shot decisions also match. Each acquisition still supplies a partial
Mecc-then-held cohort, view inputs, flat world and an empty sound registry.
Projectiles and deferred effects are not advanced by these controls.

The pre-change implementation refuses the standing, side and turn controls at
their first grounded armed selection. Synthetic regressions now cover same-clip
restart, released-trigger retention, channel end, side rate and angular rate.
700 synthetic Rust tests pass (14 ignored), with formatting and Clippy. Broader
reactions, other weapon styles, real camera/view timing and positional audio
remain prerequisites. The application still supplies `fire: None`.

Combined source `aebc47c04f6c4ae7434953a9ab0915dfcb09bbff` retains the Delphi
preparation and concurrent cinematic fixes. Its isolated GNU build passes 704
Rust tests (15 ignored), 20 Python tests, Rustfmt, Clippy, source/site checks and
the same complete 576-frame comparison matrix. Private source/runner snapshots
retain the inputs pinned by the final comparison reports.

The initial silent selected-opening replay exposed device-backed players being
queued with the audio plugin disabled. The common build now applies its actual
playback policy to selected-opening master/voice systems. The receipt-bound
replay passes all three opening/cancellation cycles, with zero queued players
at each retirement. Both silent menu/arena lifecycle endpoints also pass.
These runs create no window or audio device. They do not exercise the optional
fire phase, muted full transport, manual input or visual fidelity.

The asset-free local draft has ten allowlisted members, complete notices,
matching technical review/build receipt and no package-verifier blockers.
Executable SHA256:
`0573f047d307d3ad59f0b4758a5cef853e7be26ca02a4741333098b3808f6836`.
Archive SHA256:
`6c90d55c7102d01833ca47b7f13b37d1ee90bcea2dadc74cfa796a6133cbdf61`.
It remains private; the public download is unchanged by this CPU work.

Private REA identities: viewport-origin accessor
`ev_63f15249b41c8db2b91092642deed71a7537f1f96384bdf7d405c2a7952200e3`;
distance metric
`ev_ef655294f8e6721bb2cde4f7e61dd45fc221fa959a66d736cb838c4620735c23`;
enclosing callback
`ev_4cbaf1fe40899a74005249b38e09aceed0a0426287582f4ec7302085179138a1`.
Static inspection and controlled execution are separate evidence. Neither is
redistribution clearance or proof of a complete mission.

## Integrated Windows validation

Source `7023ac998e94792ff232a9e0b89e1e7eb9bf3143` preserves the concurrent
cinematic transport work and the local callback. Project crates, tests, both CPU
comparison runners and the GNU release were rebuilt in an isolated target;
workspace artifacts from the shared cache were excluded. The isolated comparisons
and all 696 Rust tests (14 ignored), 20 Python tests, Rustfmt, Clippy, source
inventory and site/JavaScript checks pass.

The receipt-bound executable passes both silent three-launch menu/arena
regressions and three cinematic opening/cancellation cycles. These runs disable
the audio device and create no window. They do not call the new local fire API,
test natural cinematic transport or verify manual input and visual fidelity.

An asset-free local draft has 10 allowlisted members, complete notices, a matching
technical review/build receipt, manifest and checksums, and no verifier blockers.
Executable SHA256:
`8b8092e0edc6b398ba3fa9669ce676fcbce9f7e641ff103e7e145901b7fc91ac`.
Archive SHA256:
`aab736b598904fef6dcca0d2963baa2d05eac006dcb7d85914ec006c5ae75e71`.
It remains a private draft; this CPU boundary does not publish another binary.

For the optional primary phase, combined runtime source
`36f992c362a0cfad6ba60aa3edae0c2ff9bc1fbe` also preserves the character-scoped
mission audio changes. Its isolated build passes 698 Rust tests (14 ignored),
20 Python tests, Rustfmt, Clippy and the source/site checks. The local draft has
10 allowlisted members, complete notices and receipt-bound review, and no
verifier blockers. Executable SHA256:
`d8d41a71a09f74525d6f1f1385116f0396f8e35ce5ddbbd0ba80855f0f9a70d2`.
Archive SHA256:
`30d6fd1cdf1aeab5ff25e73eaa7a7bce33624fdb216f75b0ee102189e31ab74c`.
The draft remains private; the public download is unchanged by this CPU step.
Both receipt-bound development lifecycle endpoints and three selected-opening
cancellation cycles pass with no window or audio device. These checks exercise
the common menu build, not the optional fire phase, natural cinematic transport,
manual input or visual fidelity.


## Ordinary pistol projectile presentation

The ordinary partial menu controller now uses the acquired mode2 straight launch
for both velocity and orientation. Its shot retains that launch basis while the
actual current motion supplies translation. The normal scene draws the installed
class38 projectile model, texture, UVs, vertex color policy and unlit materials
through the common material consumer. The previous yellow flight quads are
removed. The existing live-shot list produces submissions; removal from that list hides them,
and menu return removes the mission draw assets. Supported definitions retain
the acquired class38 base-projectile profile and require static unskinned source
geometry; unsupported animation/material profiles fail explicitly.

Entry: a supported ordinary Mecc mission, left mouse to fire. No replay flag or
experimental backend is required to connect this producer and consumer. This is
still the partial pistol/terrain/object-query composition. It does not construct
Session state77 actors, bind native fire phases, allocate constructor FX, provide
native range/retirement or impact callbacks/damage, replace the remaining diagnostic impact markers or
reconstruct firing-pose selection. Exact vertical/degenerate aims outside the
acquired launch basis are refused rather than given an invented orientation.
The complete arsenal is not marked complete.

Status: runtime connected, interaction unverified. Existing workspace checks and
an installed resource/owner control cover the shared implementation and source
visual preparation. A local GPU capture now shows the source projectile after
normal mission admission with supplied trigger commands. Real held input and
fidelity remain unverified. Prototype PS2 constructor and launch
name searches were refreshed first; both attributions remain unknown. This
reuses acquired PC services and does not import PS2 ABI or memory layout.


The local GNU build of source `d5a4ca8` (executable SHA256
`8874592b599a789c227c5afe65cd373be20f839eca52279e96a686615fb939c1`)
completed the existing offscreen one-shot diagnostic through normal mission
admission. Its supplied trigger produces one live shot; the actual class38 draw
consumer binds the source materials, and the private GPU capture was visually
inspected. A separate cyclic diagnostic observes two complete ordinary firing/menu-return
cycles, each with four live source-model shots and removal of all projectile draw
assets. Its third entry stops during the opening cinematic at the existing
300-second diagnostic deadline (exit101), so the requested three-cycle run is
not recorded as passed. No projectile draw failure is reported in that run.
Audio is disabled. These observations prove this local render connection, not
real operating-system input, native projectile retirement or full weapon FX.
The source was subsequently combined with concurrent main changes; synthetic
and installed owner/normal-flight regressions are checked separately from that
identified graphical build. Captures and executable stay private.
