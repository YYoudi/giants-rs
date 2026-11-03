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
- Menu-to-mission, controls, return-to-menu and cleanup require interactive checks.
- Source/provenance review, the constgebra 0.1.4 notice, compiler/runtime notice
  applicability and initial public approval remain release blockers. MIT has been
  selected for independent project source; its scope is recorded in NOTICE.md.
- Revalidate subsequent publication-preparation changes in hosted CI. The baseline
  Windows and dashboard runs passed; no binary or Pages upload was performed.
- Before source exposure, review the provenance of reconstructed implementations
  and synthetic fixtures, including whether any protected source expression or
  original data was carried over. A history-pattern scan cannot decide this.
  Record the review scope and unresolved questions; see docs/PUBLICATION_REVIEW.md.
- After source-publication approval, integrate the reviewed candidate into main,
  validate that exact remote revision and enable Pages for this repository.
  Keep binary distribution gated separately by its notice/applicability review.

## All other game systems

Track concrete tasks here as each whole-game roadmap family is opened, including
Delphi, Kabuto, squad AI, bases/economy, multiplayer, saving, UI, audio and fidelity.
Preserve untouched families in the roadmap; do not replace it with recent CPU work.
