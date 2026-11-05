# Owned local pistol callback

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
