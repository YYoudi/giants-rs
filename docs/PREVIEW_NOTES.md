# Windows x64 development preview — preview-2026.11.05.1

Place giants-rs.exe beside the original Giants.exe and double-click it to open
the integrated menu. The adjacent installation is detected automatically. From
another folder, a picker asks for the installation and remembers the selection.
Includes the independent executable, launcher, instructions, license and notices.
Linux and macOS binaries are planned for later.

Approximately 99% of current project content is AI-generated according to the
owner's estimate and has not received comprehensive human review. This estimate
is not a completion metric. The owner authorized this preview without requiring
a separate human release review. Technical checks do not establish full gameplay
fidelity or third-party rights; see DISTRIBUTION_REVIEW.md in the source repository.

Compared with preview-2026.11.04.2: the preview pistol no longer reports an object
hit behind a terrain stop in the same simulation step. Object and development
target queries use the terrain-clipped segment, matching the existing owned
projectile flight ordering. Objects before the terrain still stop the projectile.
The fix reaches the selected-controller pistol and Mecc development arena;
it does not enable the separately gated owned firing/targeting services.

Retained from the preceding preview: normal startup requires no terminal;
the downloadable executable is named giants-rs.exe; Preview All is selected with
its mission catalog unlocked; Preview tools exposes mission browsing, a Mecc test
arena and installation changes. Existing profiles are preserved. No original
save is imported and unlocking does not implement missing gameplay services.
All five occupied profile slots are retained; free one to create Preview All.

## Accessible path

Supply your own installation by placing giants-rs.exe in its game folder, or
select it through the fallback folder picker.
The preview starts at the integrated menu and can load selected missions.
Supported entries include resident Mecc movement/jet and partial presentation.
Read `QUICK_START.md` before testing. User state is kept outside the installation.

The exact executable hash, source commit and compiler are in `manifest.json`.
This packaging revision includes the launcher, instructions, license-scope
NOTICE.md and distribution review. Local ZIP verification checks member names,
manifest identity and internal/external checksums without running the game. It
does not claim complete combat or campaign integration.

## Known limits

The new terrain-occlusion regression fails against the prior unbounded segment
and passes with the correction; an object-before-terrain control also passes.
This proves the bounded synthetic collision cases, not native image/trajectory
fidelity or complete mission combat. See the release description for this
revision's exact package identities in its manifest.

This revision passed 689 synthetic Rust tests (14 ignored), 18 Python tests,
Rustfmt, Clippy and the GNU Windows release build. Silent offscreen checks on
the installed data passed both three-launch menu/arena lifecycle endpoints,
four shots/four declared-object impacts, the empty-obstacle control, and nine
hits/one death/reset on the explicitly synthetic development target. GPU captures
were inspected. These are development-path regressions, not campaign completion,
manual input validation or native trajectory/pixel fidelity.

The preceding preview passed 687 synthetic Rust tests (14 ignored), 18 Python
tests, Rustfmt and Clippy. Its interactive checks observed no-argument adjacent startup,
the default unlocked profile, clickable mission browsing, Mecc arena launch and
Escape return; the native folder picker also displayed and cancelled cleanly.
The initial window is 1280x720 and resizable. These checks are not complete
mission-playthrough, edition-compatibility or fidelity validation.

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
Dependency/runtime notices and an inventory are included. Public packages require
a matching technical distribution review with no blockers. A checksum confirms file
identity, not legal clearance or absence of defects.

## Unreleased source work after this preview

The owned Session can now publish the local target prepass's typed relationships,
aim point and terrain-reticle flag. Controlled CPU comparison and failure/lifecycle
checks are recorded in OWNED_TARGETING.md. This API is not yet called at the live
mission primary/fire boundary and does not add an accessible weapon or reticle
to preview-2026.11.05.1. The existing Windows download remains unchanged.
