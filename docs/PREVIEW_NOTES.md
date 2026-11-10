# Windows x64 development preview - preview-2026.11.10.10

Runtime source: `9a0fb244159c89054bbdfc9a53705df30bb546ec`.

## Try the ordinary Mecc controller

Start the application with your own compatible PC installation, select **Start
New Single Player Game**, wait for loading and press **Escape** to skip the
opening. Default controls are **WASD** for movement, mouse motion for aiming,
**right mouse held** for thrust, **Space** for nitro while thrusting, **left mouse**
for the partial base pistol and **R** for the view toggle. **Escape** returns to
the menu; enter again to create a fresh mission owner. Profile control bindings
apply. No replay, script or experimental-backend flag enables these actions.

The normal controller presents the animated Mecc and constructor-bound jet and
nitro models. The acquired selector chooses the descent animation only when the
actual trajectory has sufficient vertical speed and floor clearance. Releasing
thrust hides the jet models; it does not itself force a falling pose.

This snapshot adds ordinary health and jet-fuel gauges, including low/critical
colors, alongside the existing numeric values. These display the partial
controller's health/fuel; they do not establish native combat or HUD fidelity.
The base pistol draws source projectile geometry/materials. Impact markers,
native projectile/FX/damage ownership and firing-pose selection remain partial.
This snapshot predates the later landing-effect work and does not include it.

## Shared character and NPC integration

The common renderer now supports the acquired initial Delphi and Kabuto body
inspection paths. This is initial-pose presentation, not their playable movement
or action controllers. Their startup, persistent camera lifecycle, physical and
animation/backend prerequisites still prevent ordinary gameplay admission.
External PC camera coefficients and the exact-owner look-angle command are
integrated dependencies; cold loader-image words do not establish mission-entry
globals or a warm camera reset.

NPC integration retains constructor/physical contact and connects fully completed
Vimp poses to the common renderer. Supported neutral state0 callbacks reuse the
shared nonphysical owner. Actual common-world admission, population, clock/RNG,
environment and remaining actor-family services are still required: **no normal
mission has active Vimp AI in this preview**. The renderer does not create a herd
or advance AI just to show it.

The executable preserves the cinematic Session/transport work. The separate
545-source cinematic census remains pinned to preview .7; this download is not
a new census or complete-film acceptance. Mission objectives, complete combat,
campaign progression, save continuity, bases and multiplayer remain incomplete.
No whole-game feature is marked complete by this snapshot.

## Validation scope

The source passes 953 portable Rust tests (149 external-data checks ignored),
44 Python tests, format/strict Clippy and 48 installed dependency controls.
The exact executable passes three supplied-controller movement/jet/nitro/release
and retirement cycles. Separate inspected GPU captures show the plume, source
projectile and acquired descent pose after release, with the actual HUD gauges.
Runtime lifecycle and selected source-terminal cinematic transport/re-entry
checks pass on this same executable, with real audio sinks muted in the latter.
A fresh normal profile completes two actual menu entries/opening skips and
returns, OS right-mouse jet pulses and release, left-mouse fire, the R view toggle,
visible initial HUD and normal desktop exit. These are brief pulses; sustained
real-input flight/nitro/descent/landing, complete native gameplay and visual
fidelity remain unverified. Mouse coordinate injection moved the camera downward;
the pulse shot reached a ground query, while the separate GPU sample shows the
projectile. The earlier interrupted attempt was followed by this completed retest.
Historical screenshots retain their own build and do not expand these claims.

This unsigned Windows x64 development build is incomplete. Approximately 99% of
the current project content is AI-generated according to the owner's estimate,
and has not received comprehensive human review. This is not a completion metric.
Technical notice/receipt/checksum validation does not establish third-party
redistribution rights. Original data is supplied by the user and stays outside
the package. Linux and macOS downloads are future work. Earlier previews retain
their original executable, notes and checksums.
