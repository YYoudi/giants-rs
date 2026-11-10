# Windows x64 development preview - preview-2026.11.10.9

## Try the ordinary Mecc presentation

Launch the common application, select **Start New Single Player Game**, wait for
loading and press **Escape** to skip the opening. Default controls are **WASD**
for movement, mouse motion for aiming, **right mouse held** for thrust, **Space**
for nitro while thrusting, **left mouse** for the diagnostic pistol and **R** for
the view toggle. **Escape** returns to the menu. Bindings follow the active Mecc
control profile. Enter again to exercise a fresh mission owner.

This candidate connects the normal controller's actual animated Mecc and jet/nitro
state to the constructor-bound jet models and the shared renderer. The plume no
longer requires an experimental backend or replay. The normal animation selector
also prepares and selects the acquired descent clip when its actual vertical
speed and floor clearance satisfy the shared rule. Releasing thrust hides the
jet models; descent selection depends on the trajectory, rather than just the
absence of thrust.

The normal base pistol now draws the registered projectile model and materials
from each live shot's acquired straight-launch orientation and current position.
Use **left mouse** in the same mission. This replaces the provisional shot trail;
impact markers remain diagnostic. Native projectile actor/effect construction,
full fire/impact/damage ownership and firing-pose selection remain incomplete.

These changes extend the existing partial controller. Separate presentation
clocks do not acknowledge native world completion. Full native impact, health,
weapon, camera and world ownership remain incomplete. Source/GPU diagnostics do
not establish real sustained input or complete visual fidelity; see the exact
validation scope accompanying this release. Historical screenshots are separate
from the contents and acceptance of this executable.

## Other integrated work and its limits

The same application preserves the Flick cinematic Session ownership and source
transport work. The 545-source cinematic census remains pinned to preview .7;
this newer build does not constitute a fresh census or complete-film validation.

Reviewed NPC herd/Vimp services and registered Story2 resource binding, Mecc
completion orchestration, Delphi terrain/shared camera and declared-group mission
construction, and Kabuto cold-group mission construction are
integrated dependencies. Their remaining caller/backend/initialization boundaries
stay explicit. They do not enable ordinary NPC AI, playable Delphi or Kabuto,
complete combat, mission objectives, campaign progression or save continuity.
No whole-game roadmap feature is marked complete by this preview.

## Install

Supply your own game installation. Put **giants-rs.exe** beside the original
**Giants.exe** and double-click it to open the integrated menu. An installation
picker is available when no adjacent installation is found. Keep the included
license and notices. **Play.ps1** is optional; no script is needed for the normal
jetpack controls. Windows x64 only; Linux and macOS downloads are future work.

The unsigned development build remains incomplete. Approximately 99% of current
project content is AI-generated according to the owner's estimate and has not
received comprehensive human review. This estimate is not a completion metric.
The owner authorized Windows previews without a separate human review gate;
technical package validation does not establish third-party redistribution rights.
Earlier previews retain their original executable, source, notes and checksums.
