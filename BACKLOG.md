# Detailed backlog

The complete scope is retained in PROGRESS.json and the generated README checklist.
No complete game feature is currently marked done. Acquired CPU services are not
automatically wired into the main menu-to-mission build.

## Parallel character work

Each chat works alone in its own worktree and `codex/` branch. The Delphi fork
starts from the current public main, not the older fork snapshot. Keep one menu
and mission runtime; do not create separate character executables or copy the
Mecc controller into another engine. Use an isolated Cargo target per worktree.

| Lane | Scope and primary files | Shared dependencies |
| --- | --- | --- |
| Mecc | Existing `mecc_*` services, owned primary/target/fire and live integration | Actor storage, physics, animation, camera, effects and audio |
| Delphi (active in `codex/delphi-native`) | `delphi_*`: declaration/resource preparation, cold initialization, land/water movement, energy, then sword/bows/spells | Reuse the same actor and mission services; acquire character-specific callbacks separately |
| Kabuto (active in `codex/kabuto-native`) | `kabuto_*`: declaration/resource preparation, cold initialization, movement/growth, then feeding/attacks/offspring | Reuse actor/world services; do not change Delphi or Mecc rules |
| Worlds and mission presentation (separate future lane) | Missing building/scenery admission, NPC scheduling, ambient emitters, loading/briefing flow | Shared scene loader and renderer; coordinate changes to runtime admission |

Formats, actor ownership/storage, collisions, animation clocks, effect pools,
audio and the menu/loader are shared infrastructure, not three character copies.
Prefer character modules and small additive integration hooks. If a shared service
needs a behavior change, preserve other character profiles and validate their
regressions. Merge the latest main before integration, retain concurrent work,
and serialize main/release publication; never force-push over another lane.
BACKLOG.md, PROGRESS.json, generated README and release notes are also shared:
update the relevant rows and preserve other lanes' evidence and limitations.

### Delphi next steps

- Implemented: `delphi_resources::Resources` projects the actual kind-3 mission
  marker, class fields, state template and body/state specifications. The shared
  declaration helper takes an explicit expected kind; the existing Mecc entry
  remains kind 2 only. A source-data check on Rstory1's selected mission passes;
  source resources and probe outputs remain private. This is resource preparation,
  not execution of movement or a playable controller.
- Implemented: `delphi_special::Factory` constructs the ordinary class and its
  cold special phase, including the auxiliary-copy/held-object group and initial
  equipment. Thirty-two native/Rust cases match 127,692 normalized header,
  payload and ordered-list words; six declaration-reader controls pass. Owned
  reference/replay/failure/clear checks pass. See docs/OWNED_DELPHI.md.
  These are CPU controls, not complete player initialization or live integration.
- Acquire the surrounding local-player type change, reset, activation and camera
  services, then copy/held-object callbacks and normal player initialization.
  Preserve unresolved work explicitly; neither an allocated state-3 payload nor
  a completed special phase is a fully initialized player.
- Acquire land movement, jump, water entry/exit, swimming and turbo/energy;
  compare controlled continuous cycles before admitting live controls. Reuse
  shared terrain/collision/input services with the acquired character parameters.
- Connect the validated controller through the existing selected-mission path,
  camera/HUD/audio and return/second-entry cleanup. Then acquire combat and spells.
- Keep Delphi movement/campaign roadmap rows at `todo` until actual runtime
  integration exists. Resource loading alone must not change completion status.

### Kabuto next steps

- The dedicated user-created fork owns Kabuto only, working alone with an
  isolated Cargo target and no windows, focus changes or sound. Preserve the
  Mecc/Delphi lanes and the single menu-first runtime.
- Implemented: the missing ordered ground-offset and configuration-mask class
  stores now match 256 native/Rust scalar controls (98,816 complete header words)
  and 768 additional native seeded-mask controls. `kabuto_resources::Resources`
  prepares the kind-1 marker and actual body/state resources and can construct
  the ordinary cold class through the existing owned Session.
- Two ordinary cold Kabuto constructors match the native headers, state payloads
  and ordered registrations. Typed resources, independent reference/shadow caches,
  atomic failure and release are checked. See docs/OWNED_KABUTO.md. This does not
  execute special/player initialization or relocate the actor to the mission marker.
  Unknown state-1 classes, legacy profiles and mission state writes remain refused.
- Acquire and compare the actual player setup, shared reset/state/camera services
  and activation; then land movement/jumping, size-dependent collision and growth.
  Static inspection of the special dispatcher shows Mecc/Delphi branches and no
  Kabuto branch. Do not copy their child creation into Kabuto or treat that return
  as complete player initialization; compare the surrounding shared setup first.
  Reuse owned actor, terrain, input, animation, effect and audio services with
  acquired Kabuto parameters and explicit phase order.
- Connect validated behavior through selected missions, camera/HUD/audio and
  return/second-entry cleanup. Keep the campaign/movement/growth roadmap at
  `todo` until live integration is established. Private probes are not runtime
  integration, a playable preview or visual-fidelity evidence.

## Immediate runtime prerequisites

- Desktop preview entry: giants-rs.exe detects the adjacent game installation or
  offers a folder picker, then opens the integrated menu with an independent
  Preview All profile and unlocked
  mission catalog. The overlay exposes mission browsing, Mecc test arena and
  installation changes. Synthetic routing/profile checks and interactive adjacent
  startup, mission browsing, arena launch/return and picker cancellation passed.
  Broaden installation/edition and full-session coverage. These changes do not restore
  original saves or implement missing mission/character services.

- Owned Mecc targeting: the explicit Session scan now publishes typed target
  links, aim point and terrain-reticle state. The 64 controlled source-backed
  CPU cases match the reference; 128 provider/epoch failure checks preserve the
  earlier publication. See docs/OWNED_TARGETING.md. The main driver does not yet
  execute this scan. OwnedHeaders now reads class initialization/copies, Mecc
  child inheritance and acquired Ripper/projectile context publications; 128
  producer cases match the reference. Complete mission mutation coverage and the
  class805 special-header producer remain unresolved. Effective declaration range 700;
  normal pistol caller distance 500. Do not substitute a default range of 1000.
- Local target/fire coupling: advance_local now executes the actual owned scan,
  range-reticle decision and tracked/untracked pistol transaction. Its 64-case
  full callback control matches 16,890 words, including 18 shots/15 tracked shots;
  eight backend failure/refusal checks pass. See docs/OWNED_LOCAL_FIRE.md.
  The optional owned primary phase now supplies physical/first-ANM storage,
  local time and shared FX/RNG, before jet/secondary/second ANM. A continuous
  64-frame walking/pulsed-trigger control matches 31,532 normalized words,
  including two projectiles, 64 held-world/reference caches and deferred FX.
  Missing muzzle and late sound failures quarantine without replay; FX/RNG survive
  the latter. This controlled partial Mecc-then-held cohort is not a full world.
  Armed walk, standing-shot/aim, side and turn selections are acquired. Five
  continuous CPU controls (576 frames, 22 shots, 341,220 normalized words) match
  player/projectile state, held caches, FX contexts and RNG. Standing shots
  restart without blending, retain their control flag until channel end, then
  select the aiming idle. Broader reactions and other weapon styles remain open.
- Connect the primary phase to actual main-driver camera/viewport producers,
  prior held muzzle caches and positional audio; acquire remaining secondary
  reactions and full scheduling. The application still supplies `fire: None`.
  A typed selected target cannot use the no-target fire
  profile merely because its raw pointer slots remain normalized.
- Actor/projectile/effect scheduling, damage, mission objectives and progression.
- Projectile terrain occlusion: the selected-controller pistol and development
  arena now limit object/target queries to the terrain-clipped segment, as the
  owned-flight path already does. The old code fails the new behind-terrain
  regression; corrected before/behind controls pass. This does not complete
  owned firing, actor reactions, live list admission or native trajectory fidelity.
  The GNU build passed both silent three-launch lifecycle endpoints and the real-data
  declared-obstacle, empty-obstacle and development-target damage/reset replays
  on 5 November 2026. The exact behind-terrain case remains a synthetic proof.
- Cinematic integration proceeds through reviewed runtime patches; preserve the
  concurrent gameplay implementation. See the dedicated coverage and prerequisites below.
- Campaign menu-to-mission and manual controls require interactive checks.
  Silent development-session cycles passed on 4 November 2026 for the recorded
  transition binary: three launches/three returns and three launches/two returns
  ending in development gameplay, with fresh state and owned-entity cleanup
  markers. Reproduce with tools.runtime_smoke; this does not establish campaign
  loading, complete memory reclamation, manual input or visual fidelity.
- Windows downloads are owner-authorized without mandatory human release review.
  Require complete dependency/toolchain notices, an exact build/review receipt,
  asset-free package and independent checksum verification. See
  docs/DISTRIBUTION_REVIEW.md. The constgebra Apache option and conservative
  Rust/GCC/MinGW notices are now included; missing notices still block packaging.
- Add native Linux and macOS previews later, with separate build, runtime and
  notice validation. Do not label the Windows binary cross-platform.
- Preserve the AI-assisted source-review limitations recorded in
  docs/PUBLICATION_REVIEW.md; future reconstructed code and fixtures need their own
  provenance checks. Escalate concrete protected-material concerns before exposure.
- Source cutover is complete after technical review and clean-clone checks; the
  owner-authorized repository is public, with Pages enabled for this project.
  Continue checking exact-revision hosted results and validate each public binary package.
- Dashboard: all 302 owner-authorized historical comparisons have public labels,
  draggable superimposed-image dividers and an optional side-by-side view, without
  private report paths. Use grouped roadmap tables and explicit state colors/counts.
  Keep historical captures separate from current-build proof.
  Future captures require the same metadata/provenance filtering and hash inventory.

## Cinematics / Flick: coverage and next prerequisites

- Scope: 545 historical sources across 16 worlds, including all variants. The
  retained research records 317 engine images, 57 native render observations and
  18 transport checks; **zero complete scenes have verified fidelity**. Historical
  evidence is private and is not proof of the current menu build.
- A fresh CPU preparation baseline on the preceding public runtime prepared all
  545 declared sources without a top-level error. Preparation deliberately retains
  deferred body-part, rigid-attachment and effect consumers. It does not verify
  GPU output, complete playback, voices, timing, transitions or gameplay admission.
- Reviewed runtime adaptations: unscaled point supports retain an identity basis
  for child references; terrain visibility preserves the retained exact-reference
  arithmetic order and division precision. The retained terrain control covers
  2,048 rays over 16 worlds. Neither result certifies scene rendering.
- Main-build text ownership: mission adoption selects the actual installation;
  film release drops its glyph nodes, camera, all atlas versions, font and cached
  language. Another ticket and a live standalone reader cannot be taken over.
  Synthetic release/re-entry tests pass, including preservation of unrelated images.
  Preview 2026.11.05.2 passed three offscreen selected-opening/cancellation cycles
  with fresh tickets and zero reported owned film assets, plus both main-build
  development lifecycle endpoints. Its exact GNU build, notices, package and
  downloaded checksums were verified. These short samples
  allocated no glyph atlas: GPU cleanup after visible subtitles, audio-clock
  progression and full subtitle visual/timing comparisons remain pending.
- Preview 2026.11.05.3: isolated, receipt-bound checks completed two natural
  source-terminal cycles for Story4 mission 1 (13 directors, 11 voices each,
  Gameplay) and Story1 mission 5 (one director, two voices, Mecc preview with
  campaign admission still incomplete). Both returned to the menu and re-entered.
  Story1's sync track retired before its director, with zero audio time and no
  fabricated tail. It declares no bitmap-text blocks/calls; automatic voice-caption
  policy is not established. Story4 retired 190 text entities and 12 atlas images
  per entry, including a last voice observed empty at retirement. Main-world
  owned image entries were gone; total GPU/process memory reclamation and subtitle
  pixel/timing parity remain unverified.
- Final synthetic validation used the isolated worktree target: 695 Rust passed,
  14 ignored, 20 Python passed, formatting/Clippy passed. The earlier shared-cache
  inventory was rejected. Both development lifecycle endpoints passed on the
  exact packaged executable, and the downloaded ZIP/checksums were verified.
  The completed .3 baseline passed 18/29 declared menu roots. Ten failed on the
  common unrelated Mecc cue preload; Story4 mission 2 failed on periodic-effect
  preview reserve exhaustion. Do not drop effects or enlarge a constant to hide
  that failure. Investigate actual scheduling, lifetime and render ownership.
  The .4 full matrix is running separately; transport passes do not certify
  complete scenes or all 545 sources.
- Published preview 2026.11.05.5 fixes fixed attached-emitter catch-up allocation.
  A diagnostic main build failed with four occupied slots after a measured
  215 ms callback; replaying its deltas with the actual source recipe reproduces
  the failure. Demand allocation preserves the fixed constructor's cold cache,
  requests, RNG order, lifetimes and retirement; expired slots are reused.
  Reference-channel admission is unchanged. The unlit renderer creates actual
  additional slots. Ordinary film updates now adopt late meshes, materials,
  buffers and draw entities, not only those created at director setup.
  The exact published executable passes two natural-terminal/menu/re-entry
  cycles each for Story4 missions 1 and 2, Story1 mission 5 and Rstory5 mission 2,
  plus both development endpoints. The previously failing long chain completes
  all 54 directors twice, with nonempty text-atlas cleanup, 38 late draw entities
  adopted per ticket and 7,702 tracked main-world film assets absent per release.
  697 Rust tests pass (14 ignored), 20 Python tests and Clippy pass. Independent
  downloaded package/notices/receipt checks pass. The later optional Mecc source
  work is preserved by merge and checked separately; do not attribute that newer
  source identity to the published executable.
  The subsequent combined-main source passes 699 Rust tests (14 ignored), Clippy,
  an isolated GNU build, both development endpoints and three selected-opening/
  cancellation cycles. That separate build does not repeat the long natural
  transport proof or verify the optional Mecc fire phase.
  Native allocator capacity, global scheduling, visual/audio/subtitle parity,
  skip input and complete GPU/process memory reclamation remain unverified.
- Read-only sync-track size inventory: all 545 files parsed, 51 declarations,
  49 distinct tracks; none falls below the acquired streaming-buffer size.
  This limits the immediate relevance of the unresolved short-stream constructor
  profile; it does not prove hardware timing, admission or the live caller state.
- Published preview 2026.11.05.4, validated on its isolated exact executable: Mecc-specific cue/carry/footstep
  preloads now require the actual single command11 declaration of kind2. Other
  or ambiguous starts retain generic actor lookup from their real catalogs and
  explicitly lack an implemented local-player cue profile. A failed Mecc player
  preparation must not suppress required Mecc sounds. Read-only packet controls
  identify kind3 in the failing Reaper entry and kind2 in the Mecc control.
  Two source-terminal/re-entry cycles passed for each of Rstory5 mission 2,
  Rstory1 mission 3, Story4 mission 1 and Kstory1 mission 1. Mecc retained its 22
  preloaded sources; other profiles reported no Mecc cues and no replacement
  samples. Both development lifecycle endpoints passed. The subsequent merged
  source passes 696 Rust tests/Clippy; its final package revalidation is pending.
  This does not implement Delphi/Kabuto audio or waive admission.
- Exact-reference audio research: 32 CPU query controls cover absent streams,
  supplied device failures/cursors and wrap state. Six additional controls execute
  the worker's retirement path after a supplied EOF/end notification, then the
  real cleanup and time query. Retiring the matching stream clears its clock;
  retiring another stream preserves the selected clock. No device, original
  thread or natural notification timing was verified by these controls.
- The common audio master now supplies zero after stream retirement, disabling
  drift correction while the director continues on its own clock. It no longer
  invents a realtime audio tail or rejects a director solely because its sync
  track ended first. Synthetic clock/restart/track-replacement controls pass in
  the isolated target. The exhausted-track Story1 menu control passed on the
  published executable. Native notification timing, small-buffer admission and
  device mix remain open. Analysis stays private.
- Preserve the menu -> selected mission -> preparation -> full movie -> skip or
  natural end -> gameplay -> menu -> second-entry acceptance path. The silent
  opening-cycle replay calls the real selection and cancellation services without
  key/mouse injection; it samples an opening, not natural completion or gameplay.
  Selected-opening master/voice systems now honor the explicit device-playback
  policy. A slow silent replay exposed players being queued without an audio
  plugin; the corrected combined build passes three cancellation cycles with
  zero queued players for each ticket. Prepared audio assets remain owned and
  released. Muted full-transport verification is separate from this control.
- Menu opening inventory: 56 selectable mission requests across 19 menu world
  entries; 29 declared opening chains, 26 without an opening request and one
  unresolved Kabuto name. Menu world entries include catalogs outside the
  16-world cinematic corpus. These are declarations, not playback coverage.
  The Kabuto case matches retained exact-reference name-resolution research:
  an absent table name does not justify an invented alias or another movie.
  Its live caller/world state and resulting mission flow remain unresolved.
- Common blockers remain: general effect dispatch beyond the admitted terminal
  binding, deferred mixed attachments/body variants, real world visibility inputs,
  glare/occlusion integration, exact audio-clock/end behavior and material/light/
  shadow/water comparisons. Review existing private evidence before new analysis;
  do not import the historical commit series or private fixtures in bulk.
- Do not broaden campaign readiness or invent placement, delays, effects or an
  audio tail to hide a mismatch. Supported partial previews remain explicit.
  Interactive controls, skip gestures and full-session validation are pending;
  current work is strictly offscreen, without focus changes, input or sound.

## All other game systems

Track concrete tasks here as each whole-game roadmap family is opened, including
Delphi, Kabuto, squad AI, bases/economy, multiplayer, saving, UI, audio and fidelity.
Preserve untouched families in the roadmap; do not replace it with recent CPU work.
