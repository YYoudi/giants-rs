# Windows development preview - preview-2026.11.11.4

Runtime source: `f92f7ad4f71b08f7d0638f11d73c44230379dbee`.
Executable SHA256: `2d2219e6fae03ae07b5981d5b4b7c42cf9e04641529a287a632500c867447e16`.

Supply your own compatible PC installation. Original assets, executables and
private research are excluded from this download.

## Try normal meat collection

Open **Preview tools -> Browse all missions -> Mecc -> Starving!** (Story2),
enter the Mecc preview and skip the opening with Escape. WASD/mouse move and aim,
right mouse uses the jet, Space adds nitro, left mouse fires the partial pistol,
R changes view and Escape returns to the menu. F8 toggles the herd/meat hints.
The authored start is roughly 2,000 source units from the herd.

Shoot a Vimp, wait for its dropped meat to settle, then approach the pieces.
Collection is automatic through the ordinary controller; there is no pickup
script or special activation flag. The carried-meat count is shown in the hint
overlay. The acquired PC callback accepts pieces while the carried amount is
below 20; the final piece can take it above 20. Full inventory leaves remaining
pieces to expire normally. Pickup plays its real local-player cue. Picked pieces
stop drawing and retire through their actual owners, separately from expiry.
Vimp collision, danger/hurt/recovery, blood/recoil, death sound/FX and ghosts remain
connected to this same menu-first build.

The prototype PS2 lookup began with the literal names `mecc_collect_food` and
`mecc_check_food_chunk_collect`. Their PC relationships remain probable; the
retirement search key remains unassigned. No new attribution is confirmed, and
no PS2 ABI or layout is imported. See MEAT_COLLECTION.md and SYMBOL_NAVIGATION.md.

## Validation and limits

On this exact executable, the graphical hunt collects 4 real dropped
pieces, reaches carried amount 4, retires all 4 picked owners and
starts real muted pickup sinks. The complete passage renders actual hurt/death,
blood, ghosts, meat and FX without a component refusal. Cancellation after actual
collection releases the session/audio and returns to zero remaining entities.
The separate living-contact passage records 3219 player responses without
shots or forced poses. First-death capture, three common Mecc/fire/jet cycles,
actual landing/dust/audio, two menu lifecycle modes and full selected cinematic
source-terminal/re-entry transport also pass on this executable.

Commands are supplied to the ordinary controller in these background graphical
tests. Real OS-held input, audible parity, native timing and complete visual
fidelity remain unverified. No manual Windows gameplay test is claimed for this
executable. The initial local candidate exposed a collected-versus-expired cleanup
refusal and was replaced; its evidence is not publication acceptance. A later
three-cycle diagnostic exceeded its single-entry overall wall budget while the
third source opening was still advancing. The diagnostic budget now scales with
the requested cycles; actual source clocks, readiness and completion predicates
are unchanged. Another graphical run exposed serial positional-audio
backlog under a real burst. Consecutive same-cue fixed-position one-shots now
stage together in request/RNG order, with real per-voice sink confirmation. Actor,
reuse, control, other-cue and other-ticket requests stay ordered barriers; no
queue limit is enlarged and no request is dropped or falsely acknowledged.

979 portable Rust passes / 168 ignored; 45 Python passes; formatting and strict
workspace/all-target Clippy pass. Fresh exact-PC controls compare 192 offline
collection callbacks (319,008 selected words) and 602 proximity plans (2,007 words).
The callback control additionally checks separate collected/expired retirement
and one external release. Its supplied mission gates, births, clocks and external
release are scoped CPU inputs, not graphical/sound proof. Pickup sinks and cleanup
are independently observed in the graphical application.

Campaign objective credit is deliberately disabled because the preview has no
running objective VM. No quota, mission prefix/gates or successful quest is
fabricated. Feeding, deposits, economy, replenishment, Delphi mana, other staged
AI, complete native world/Mecc primary and mission progression remain unavailable
or incomplete. Death FX retain explicit host-frame scheduling, terrain-only
contacts and source-owner-lifetime splat policies. Delphi/Kabuto retain initial
body inspection; the separate cinematic census remains pinned to .7.

Technical collection includes 338 dependency and eleven toolchain notice entries,
29 system DLL imports and zero package blockers. The ZIP is bound to the compiler,
linker, runtime-source and executable receipt and is downloaded independently
before and after publication. This is technical validation, not third-party-rights
clearance or comprehensive human review. The owner's approximate 99% AI-generated
estimate remains an authorship estimate, never a completion metric.
