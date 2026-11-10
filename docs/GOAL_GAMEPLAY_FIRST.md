# Gameplay-first reconstruction objective

Owner direction, 10 November 2026: work from the most obvious and important
player experience toward increasingly specialized details. This is the shared
objective and task-selection policy for the main, Mecc, Delphi, Kabuto, NPC and
Flick lanes. The complete Rust + Bevy game remains the goal; this ordering does
not reduce the campaigns, characters or fidelity scope. `BACKLOG.md` remains the
single detailed backlog. [INTEGRATION.md](INTEGRATION.md) defines integration and
[RECONSTRUCTION_METHOD.md](RECONSTRUCTION_METHOD.md) defines native research.

## Choose work in this order

1. **Make the ordinary playable path work.** Launch with a user-supplied
   installation, enter through the menu, use real keyboard/mouse controls, see
   the controlled character move and aim, use the camera and basic character
   actions, then pause, return and enter again. Fix crashes, blocked controls,
   missing visible feedback and broken transitions on this path first. For the
   existing Mecc path, thrust/release/descent/landing and their animation/effects
   belong to this priority, alongside basic weapon use and its visible result.
2. **Make that play meaningful and continuous.** Connect actual world actors,
   basic NPC responses and interactions, hits/health, objectives, failure/restart,
   progression and the state needed to continue a mission. Bring the other
   characters' basic playable loops into the same application. Reuse shared
   ownership and scheduling; do not create separate character executables.
3. **Extend playable campaign coverage.** Complete the remaining missions,
   character actions, encounters, required scripted sequences/cinematics, sound,
   menus and persistence through real entry points. Prioritize broken or missing
   common experiences over rare branches within an already working subsystem.
4. **Improve broad fidelity and usability.** Address visible animation/camera/
   rendering and audible discrepancies, timing, performance and compatibility
   across the supported paths. A discrepancy that blocks an earlier experience
   moves up to that earlier priority immediately.
5. **Refine specialized cases.** Rare effects, obscure variants, exhaustive
   numeric matrices and unusual platform/version details follow the common
   playable paths. Work on one earlier only when evidence identifies it as an
   actual blocker of the selected higher-priority experience.

A necessary dependency inherits the priority of the experience it blocks. Native
research and CPU controls remain necessary when they resolve that dependency;
their size or number does not give them priority by itself. Do not wait for an
exhaustive effect/variant catalog before connecting a supported basic action.
Equally, do not fabricate missing services, force readiness or substitute default
global state to make an incomplete native path appear playable.

## Select an observable delivery before another isolated component

Before each task or continuation, name the player-visible behavior to unlock or
repair, its ordinary menu/input path and the first concrete missing dependency.
Consult current main and the latest actual preview. Prefer completing a producer
and its consumer over adding another independently callable service. Main owns
shared wiring, combined validation and delivery; each specialized lane follows
the same ordering within its domain and supplies reviewable committed changes.
Each chat still works alone in its own worktree. This policy authorizes no new
agents, cross-chat messages, worktree moves or changes to private research inputs.

Keep one current next delivery and its blocking prerequisites near the top of
`BACKLOG.md`; retain the detailed domain inventory and historical evidence below.
Do not replace the next playable delivery with a list of unrelated acquisitions.
Resumed context must retain this delivery, the producer/consumer gap and the
mandatory prototype PS2 lookup before any further native-function work.

## Evidence and delivery are separate

Use these explicit states in progress text and handoffs:

| State | Evidence required | What the owner can test |
| --- | --- | --- |
| CPU dependency validated | Scoped Rust/native or synthetic results, with supplied inputs and missing services recorded | Internal controls only; no gameplay availability claim |
| Runtime connected, interaction unverified | Actual shared owner, producers, consumers and lifecycle connected | Name the remaining interaction check; do not say playable yet |
| Interaction verified on a local build | Actual menu entry and real input, expected visible/audible response, release/exit and re-entry on an identified build | Give that exact local build and steps; do not imply it is downloaded |
| Available in a published preview | The verified interaction is in the exact technically validated, published and independently checked package | Give release link, mission/menu, controls, expected result and supported limits |

CPU means that a calculation or service runs without establishing its graphical,
interactive world integration. A replay/script is useful diagnostic evidence but
does not prove the owner can trigger the behavior with ordinary controls. Test
the real keyboard/mouse path for a claim of player interaction; retain authorized
muted/background operation and avoid unrelated focus changes.

Every handoff and milestone report must lead with:

- The observable behavior changed, or explicitly "intermediate dependency only".
- Where to enter and which normal controls to use, or "not yet available" with
  the first missing producer/consumer and the responsible lane.
- The exact tested source/build and what was actually exercised.
- Whether it is on main, locally testable, or in a named downloadable preview.

Keep detailed numeric results as supporting evidence. A successful control,
source merge, screenshot or CI package is not a playable feature by itself.
When an interaction becomes available, update its prerequisites, progress and
download instructions together, then publish according to the existing receipt,
notice and checksum process. Do not introduce another human approval gate.
Historical gallery captures retain their context; never imply that their contents
are available in a newer or older download without checking that exact build.

This direction changes prioritization and reporting, not evidence standards,
confidentiality, distribution scope or attribution. Keep README English, the
owner's approximate 99% AI-generated estimate and the absence of comprehensive
human review. None is a completion measure.
