# Windows x64 development preview — preview-2026.11.04.1

First downloadable Windows preview. Includes the independent executable,
launcher, instructions, project license and third-party notices. Supply your own
Giants installation. Linux and macOS binaries are planned for later.

Approximately 99% of current project content is AI-generated according to the
owner's estimate and has not received comprehensive human review. This estimate
is not a completion metric. The owner authorized this preview without requiring
a separate human release review. Technical checks do not establish full gameplay
fidelity or third-party rights; see DISTRIBUTION_REVIEW.md in the source repository.

Compared with the previous local draft, this release adds repeatable silent
lifecycle validation, completes dependency/runtime notice collection and enables
receipt-bound public packaging. Runtime Rust and locked dependencies are unchanged.

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

On 4 November 2026, silent offscreen development-session checks passed on the
recorded transition executable: three launches/three menu returns, then a separate
three launches/two returns ending in development gameplay. Fresh health/projectile
state, owned-entity cleanup markers, GPU capture completion and exit status were
checked. `tools.runtime_smoke` makes these checks repeatable with a build receipt
and user-supplied data. Private captures and logs are not release members.

Campaign menu-to-mission testing, manual controls and human gameplay review remain
pending; the owner has authorized manual tests. This is not complete mission
integration, total memory-reclamation proof or a fidelity comparison. Historical
offscreen comparisons retain their original executable identities.

## Package checks

The package contains the independent runtime and project documentation only;
no original archives, extracted textures, models, audio or screenshots are added.
Available dependency notices and an inventory are included. The manifest lists
unresolved notices and other publication blockers. A checksum confirms file
identity, not legal clearance or absence of defects.
