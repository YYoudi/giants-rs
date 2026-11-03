# Windows x64 preview — draft for review

**Not cleared for public distribution.** Dependency notice applicability and
the source/provenance review are still pending. Project source is MIT-licensed
within the scope described in NOTICE.md. This draft is a review artifact,
not a completed gameplay milestone or a public release promise.

Approximately 99% of current project content is AI-generated according to the
owner's estimate and has not received comprehensive human review. Automated
checks do not replace that review.

## Accessible path

Supply your own game installation with `Play.ps1 -DataRoot 'D:\Games\Giants'`.
The preview starts at the integrated menu and can load selected missions.
Supported entries include resident Mecc movement/jet and partial presentation.
Read `QUICK_START.md` before testing. User state is kept outside the installation.

The exact executable hash, source commit and compiler are in `manifest.json`.
This packaging revision includes the launcher, instructions, license-scope
NOTICE.md and distribution review. Local ZIP verification checks member names,
manifest identity and internal/external checksums without running the game. It
does not claim a new combat or campaign integration.

## Known limits

Mission firing is still refused while required services remain disconnected.
Recent CPU weapon and target-query checks do not establish playable combat.
Active actors, damage, squad behavior, objectives, saved continuity, Delphi,
Kabuto, bases, multiplayer and full visual/audio fidelity remain incomplete.
Some mission entries may stay inspection views.

This session performs silent source/build/package checks only. Interactive
menu-to-mission testing on this exact package and human gameplay review are
pending; the owner has since authorized manual tests. Historical offscreen
comparisons retain their original executable identities.

## Package checks

The package contains the independent runtime and project documentation only;
no original archives, extracted textures, models, audio or screenshots are added.
Available dependency notices and an inventory are included. The manifest lists
unresolved notices and other publication blockers. A checksum confirms file
identity, not legal clearance or absence of defects.
