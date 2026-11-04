# Detailed backlog

The complete scope is retained in PROGRESS.json and the generated README checklist.
No complete game feature is currently marked done. Acquired CPU services are not
automatically wired into the main menu-to-mission build.

## Immediate runtime prerequisites

- Owned Mecc targeting: preserve the current implementation; source header context
  and special-header producer remain unresolved. Effective declaration range 700;
  normal pistol caller distance 500. Do not substitute a default range of 1000.
- Connect typed target/reticle results, actual camera/muzzle timing and fire phase
  at the real first-animation/physical boundary; then verify secondary reactions.
- Actor/projectile/effect scheduling, damage, mission objectives and progression.
- Integrate cinematic work through reviewed patches from its current branch,
  preserving concurrent gameplay changes. Pending branch work is not yet in main.
- Campaign menu-to-mission and manual controls require interactive checks.
  Silent development-session cycles passed on 4 November 2026 for the recorded
  transition binary: three launches/three returns and three launches/two returns
  ending in development gameplay, with fresh state and owned-entity cleanup
  markers. Reproduce with tools.runtime_smoke; this does not establish campaign
  loading, complete memory reclamation, manual input or visual fidelity.
- Binary distribution remains blocked by human provenance/release review, the
  constgebra 0.1.4 notice and compiler/runtime notice applicability. MIT applies
  within NOTICE.md's scope. No executable release is authorized by source exposure.
  The owner requested downloadable releases on 4 November 2026. An unpublished
  GitHub draft `preview-2026.11.04.1` now records the prepared build and review
  checklist; it has no assets. Rechecked: constgebra is a normal dependency via
  hexasphere/Bevy and its recorded upstream tree has no license file. Obtain
  authoritative notice material or replace the dependency and rebuild; do not
  invent a copyright notice. PE imports list Windows system DLLs, which does not
  resolve applicability for statically linked compiler/runtime components.
- Preserve the AI-assisted source-review limitations recorded in
  docs/PUBLICATION_REVIEW.md; future reconstructed code and fixtures need their own
  provenance checks. Escalate concrete protected-material concerns before exposure.
- Source cutover is complete after technical review and clean-clone checks; the
  owner-authorized repository is public, with Pages enabled for this project.
  Continue checking exact-revision hosted results and keep binary distribution gated.

## All other game systems

Track concrete tasks here as each whole-game roadmap family is opened, including
Delphi, Kabuto, squad AI, bases/economy, multiplayer, saving, UI, audio and fidelity.
Preserve untouched families in the roadmap; do not replace it with recent CPU work.
