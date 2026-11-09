# Main runtime integration

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

On continuation, inspect outstanding lane work and publication state first,
retain exact evidence identities and native-name lookups, then select the next
runtime dependency from `BACKLOG.md`. Waiting for a necessary lane dependency is
not evidence that its feature is already integrated.
