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
with 19 Python tests, Rustfmt and Clippy. These checks remain separate from live
mission input, complete primary scheduling and rendered fidelity.

Eight additional owned-backend checks cover origin disagreement before mutation
and missing-header failures with quarantine/no replay. These are refusal checks,
not parity claims for native error paths. Original data, listings, private probe
inputs and reports remain excluded from the public repository.

Physical/first-animation completion, camera/viewport snapshots, cache epoch and
time are supplied at this API boundary. The main mission driver still does not
execute it: actual primary/view/muzzle scheduling, complete actor admission and
mission context coverage, the class805 special header, projectile world updates
and secondary reactions remain prerequisites in BACKLOG.md. This does not add a
playable weapon to the downloadable preview or establish visual fidelity.

Private REA identities: viewport-origin accessor
`ev_63f15249b41c8db2b91092642deed71a7537f1f96384bdf7d405c2a7952200e3`;
distance metric
`ev_ef655294f8e6721bb2cde4f7e61dd45fc221fa959a66d736cb838c4620735c23`;
enclosing callback
`ev_4cbaf1fe40899a74005249b38e09aceed0a0426287582f4ec7302085179138a1`.
Static inspection and controlled execution are separate evidence. Neither is
redistribution clearance or proof of a complete mission.
