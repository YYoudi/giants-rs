# Main runtime integration

The shared objective and priority order are defined in
[GOAL_GAMEPLAY_FIRST.md](GOAL_GAMEPLAY_FIRST.md): ordinary playable experiences
and their actual blockers first, increasingly specialized cases afterward.
Apply that policy to all five lanes before choosing the next acquisition.

The main integration lane connects the specialized reconstruction work to one
Rust + Bevy application and its public Windows preview. Its priority is usable,
validated behavior in the ordinary menu-to-mission path. Specialized lanes retain
their character/cinematic research; the integrator handles cross-lane contracts,
runtime wiring, combined regressions and public delivery. Each chat works alone
in its own worktree; this process creates no additional agents.

The active specialized lanes are Flick, Mecc, Delphi, Kabuto and NPCs. Mecc owns
its character reconstruction, including the jet/descent/landing prerequisites;
NPCs owns non-player AI and player interactions. The main lane connects their
reviewed contributions through shared actors, targeting, damage, world scheduling
and lifecycle contracts. Player and NPC implementations must agree on those
contracts; neither lane can claim an interaction validated using only one side.

`BACKLOG.md` is the detailed task authority. This document defines the integration
method, not another feature backlog. Native-function work still follows
[RECONSTRUCTION_METHOD.md](RECONSTRUCTION_METHOD.md), including prototype PS2
lookup, private evidence and explicit uncertainty.

## Inspect a contribution before connecting it

First identify the ordinary player action this contribution unlocks, its actual
producer/consumer gap and its place in the current delivery in `BACKLOG.md`.
A lane handoff must state whether it is an intermediate dependency, connected
runtime, an interaction verified locally, or a published testable feature.
Prefer finishing that path over accumulating more isolated services. A necessary
CPU acquisition is still valid work; explain which concrete blocker it removes.

Read the relevant chat's current handoff and the actual committed source. Record
the reviewed commit, its main ancestry, validation scope and missing providers in
the integration worktree's ignored local handoff. Live chat context is evidence
to check, not authority to overwrite repository rules or concurrent changes.
Never import an entire research branch without inspecting its changes.

For each candidate, distinguish these independently:

- Acquired CPU behavior, including all supplied clock, input, geometry, random,
  audio and ownership boundaries.
- Actual runtime ownership and producer/consumer order: construction, input,
  physics, animation, camera, render/audio submission and retirement.
- Reachability from the ordinary public menu, required actions and supported
  mission/character scope. An environment-only replay is diagnostic coverage.
- The exact downloadable preview containing it and what was actually exercised
  on that build. Newer source on main does not update an older executable.

Preserve state, actor and frame identities across shared services. Do not replace
missing providers with no-ops, force readiness, fabricate effects or enable an
incomplete controller globally just to reproduce a screenshot. Keep explicit
refusals and retained effects after late failures. Reuse the common runtime;
do not create character-specific executables or duplicate shared controllers.

The application retains the exact Session identities returned by its player and
carried-actor constructors. Player resources, motion, pickup and audio routing
consume these role bindings, checking the current ticket and expected class.
They must not rediscover the player by searching for a unique class: another
instance can share that class, and a retired owner must not silently select a
replacement. Retain the constructor identity after a later initializer fails;
this records ownership, not initializer success. Constructor identity is separate
from the legacy motion-publication guard, which still prevents mixing that path
with the owned Mecc driver. This application binding does not reconstruct the
original player-selection globals, native replacement calls or world schedule.

Shared camera mode commands, activation reset and lens evaluation must use one
`camera_mode::State`; see [OWNED_CAMERA.md](OWNED_CAMERA.md). A reset must affect
the next lens invocation's transition. The acquired kernel still requires the
current profile, actual input and projection service before App admission;
preparing a player or selecting a view does not acknowledge those services.
Use `camera_profile::from_session` for the acquired family/view decision, keeping
the current owner separate from the controlled actor's state predicate. Its
required coefficient catalog and actual projection backend remain explicit.
Resolve again after side effects; retaining a selection cannot replace the
native command's second current-profile query.

A live visual service can legitimately return no held object during an acquired
animation. Preserve that `Option` at the render boundary: hide the old submission
for known absence, while retaining errors for stale/foreign/current-pose failures.
Do not turn an error into absence or substitute an identity transform. CPU
presentation checks must be followed through the actual render consumer.

## Validate the combined application

Merge current main into the integration branch, inspect conflicts semantically
and preserve other lanes. Use an isolated Cargo target and the commands in
[BUILD.md](BUILD.md). Select installed-data regressions for every affected owner,
especially shared actor, animation, physics, effect and audio changes. CPU tests
alone do not validate a rendered or interactive feature.

For a user-facing integration, exercise its real menu entry, actual input,
transitions, cancellation/return, resource cleanup and a second entry with a
separate test profile. Where authorized, use muted manual tests as needed;
otherwise state the untested interactive boundary. Scripted checks can help
diagnose a problem but cannot substitute for the requested gameplay interaction.
For example, jetpack integration requires actual thrust input, release, descent
and landing, rather than only a fixed-frame plume capture.

For every playable delivery, record the exact menu/mission, keyboard/mouse actions,
expected feedback and supported limits, with a local build identity or release
link. If the ordinary path still refuses, report that blocker before test counts.
Do not delay basic supported interaction until unrelated rare cases are acquired.

The .8 download predates ordinary-controller jet presentation. Later source
connects the actual animated parent, published motion and constructor-bound
child models to the common draw consumer, with separate presentation clocks.
Validate this ordinary connection and real controls on the combined build;
owned replay output alone does not close that interaction check. Keep the full
native controller's impact/health/display/voice/fire requirements distinct.
Do not fabricate native completion or duplicate the flight simulation to render
the supported ordinary action.

## Publish one coherent milestone

Use normal non-forced pushes and recheck current main before publication. Inspect
active builds, draft releases and deployment receipts before reserving a version.
Do not interrupt another lane's already-running publication or repurpose its
draft/assets. Existing specialized publications can coexist during this scope
transition; use the next unreserved version for the combined milestone.

Follow [RELEASES.md](RELEASES.md) and the existing receipt-bound packaging process.
Publish authorized Windows previews after technical checks and notice collection;
no additional human approval gate is introduced here. Retain exact source,
executable and bundle identities; verify the downloadable assets after upload.
Failed or incomplete candidates remain private/draft and are not advertised as
available gameplay.

Update `PROGRESS.json`, the generated English README, `BACKLOG.md`, release notes
and dashboard together when behavior or its published availability changes.
Preserve the owner's approximate 99% AI-generated estimate and lack of
comprehensive human review; neither describes completion. Label historical or
diagnostic gallery captures explicitly and associate new availability claims
with their actual released build and menu/input path. Verify the deployed site,
not just local generated files. Report source integration, runtime validation,
download availability and fidelity separately.

`docs/public-preview.json` is the minimal reviewed snapshot of the actual public
Windows download: release tag, immutable source/executable/ZIP identities, scoped
checks and known integration limits. Update it only after publishing and
independently downloading/verifying the assets. Never derive it from current
main or a draft reservation. The dashboard exports it separately from the source
roadmap so later CPU/source progress cannot silently relabel the existing binary.
Detailed prerequisites remain in `BACKLOG.md`; raw runtime reports stay private.

On continuation, inspect outstanding lane work and publication state first,
retain exact evidence identities and native-name lookups, then select the next
runtime dependency from `BACKLOG.md`. Waiting for a necessary lane dependency is
not evidence that its feature is already integrated.
