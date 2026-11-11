# Windows development preview - preview-2026.11.11.1

Runtime source: `bc730cafe197d1d3512ace766ec8c16b4700317b`.
Executable SHA256: `619164999dbbec8278052a3262d2c13e3b81fc3d14da2508fe814d6f39f7a175`.

This common Rust + Bevy build consolidates the reviewed Meccs, Delphi, NPCs,
Kabuto and Flick contributions. Their research worktrees were preserved.
Supply your own compatible PC game installation; no game assets or original
executables are included.

## Normal Mecc controls

Select **Start New Single Player Game**, wait for loading and use **Escape** to
skip the opening. Default controls are **WASD** movement, mouse aim, **right
mouse held** thrust, **Space** nitro while thrusting, **left mouse** the partial
pistol and **R** the view toggle. **Escape** returns to the menu. Profile bindings
apply. These connections need no replay or experimental flag.

Brief fire presses now survive until the next controller step, while the actual
pistol retains cooldown authority. The three jet children use their canonical
state0 callbacks, ANM/reference owners and shared model buffers; the renderer no
longer advances a copied channel clock. Ordinary air-to-terrain landing now
creates the acquired dust effect with positional audio and canonical retirement.
The descent pose follows actual vertical speed and floor clearance; releasing
thrust does not force a pose. Health/fuel gauges and source projectile rendering
remain available. Native damage, full firing/impact ownership, slide, carried/
special profiles and complete world admission remain unfinished.

## Inspect Delphi and Kabuto without a replay

Open **Preview tools -> Browse all missions**, choose the character's first
world and a mission, then skip its opening or wait for free-camera inspection.
Press **F7** to focus the initial body; **R** resets that inspection view.
Free-camera movement uses WASD/arrows, right mouse look, Space/Ctrl vertical
movement and Shift speed. Escape returns to mission selection.

Delphi uses the existing declared-start inspection view. Kabuto uses its actual
owned look angle and size with compatible external camera coefficients from
the supplied installation; an unsupported profile keeps the existing view and
shows a status message. The configured external profile path remains supported.
The projection now survives free navigation/reset. These are static body
inspections. Delphi and Kabuto gameplay controls remain unavailable; no startup
requirement or native projection reset is acknowledged by focusing a body.

## Preserved advanced work and remaining wiring

- Mecc: ordered burst activation and immediate-child preparation/zero birth pose
  are retained. Their canonical command/construction/contact/audio backend is
  still required before replacing partial projectile impact feedback.
- Delphi: generic class0/state7 shell, ordered projection checkpoint, shared
  look angle and completed-phase body caches are integrated dependencies. Real
  same-identity retyping, persistent camera entry and complete physical/held/
  water/input scheduling remain required.
- NPCs: reference-bearing and eventless animated state0 callbacks, typed Mecc
  children and class230 awakening/marker birth are retained. Normal missions
  still have no active Vimp AI; world admission, populations, clocks/control/noise
  and RNG interleaving cannot be replaced by installed-test contexts.
- Kabuto: the common profile-camera kernel and owned body inspection are
  connected. Projection reset, animation cache, procedural material and geometry
  registration remain pending, with startup/warm order and the live primary.
- Flick: canonical virtual-support timed storage, scoped expiry children and
  cancellation are preserved alongside normal film ownership/transport. The
  timed menu renderer still has preview playback; migration needs real source
  clock/RNG/caller-slot and populated environment providers. Per-director seed1
  is an explicit preview policy. The 545-source census remains pinned to .7.

BACKLOG.md records the concrete producer gaps and the central reconstruction
method governs future prototype PS2 name lookup. This integration establishes
no new confirmed attribution. A recovered name or CPU result is not gameplay
or fidelity validation. Complete campaigns, native combat/world behavior and
visual/audio/timing fidelity remain unverified.

Earlier previews and screenshots retain their original source/build identities.
Only the display captures explicitly labeled for this snapshot belong to it.
The owner estimates approximately 99% AI-generated project content, with no
comprehensive human review; this is not a completion measure.

## Validation of this exact executable

The combined workspace passed 960 portable Rust controls (159 installed/optional
controls ignored), 60 separately executed installed-data controls and 44 Python
checks, formatting and strict all-target Clippy. Seven GPU scopes covered three
normal Mecc admission/fire/jet/nitro/release cycles, real terrain landing and
muted positional audio, plume/projectile/descent, Kabuto profile camera and
Delphi initial body. Two runtime lifecycle modes and the full selected cinematic
source-terminal/reentry transport passed with real muted sinks and zero retained
mission entities/assets. Transient shader pipeline retries resolved with no
permanent failure.

Actual OS interaction on this executable verified two briefly remapped keyboard
shots, default mouse fire after GUI reset, a right-mouse jet pulse, R view, two
normal F7 entries each for Delphi and Kabuto, six clean menu returns and normal
desktop exit. Native F9 captured the two character inspection views. Sustained
OS-held flight and native fidelity remain unverified; supplied-controller GPU
checks are separately scoped.
