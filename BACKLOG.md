# Detailed backlog

The complete scope is retained in PROGRESS.json and the generated README checklist.
No complete game feature is currently marked done. Acquired CPU services are not
automatically wired into the main menu-to-mission build.

## Prototype PS2 attribution method — 5 November 2026

Apply [RECONSTRUCTION_METHOD.md](docs/RECONSTRUCTION_METHOD.md) before every
native-function acquisition or implementation, including resumed tasks in every
character lane. The read-only corpus audit verifies 4,769 literal function symbols;
the reviewed summary confirms three routine identities, keeps two candidate
matches probable and one unassigned. No gameplay status changes follow from names.

- Next Mecc work: retain completed name lookups for the ordinary-camera producer
  and approach helper; carry probable wall/profile candidates and their missing
  PS2 comparisons without inferring types from them. Acquire pre-primary and
  subjective fire timing, positional audio and native mesh/list producers, with
  separate PC/PS2 identities and platform differences.
- Revisit acquired services incrementally on touch: primary/animation, aim/target,
  collision, physics, wield/projectiles, audio and shared actor services. Preserve
  old identifiers as aliases. Use the central continuation record and the lookup
  tool's profile/alias fields so resumed work retains its exact evidence identity.
  Do not batch-apply naming tables.
- Delphi/Kabuto and cinematic continuations must record the same lookup for their
  next actual functions; resources or an inferred module name prove no controller.
- Recover ambiguous demangling and any useful `.mdebug` types privately only when
  needed; validate layouts against the actual PC build. Retain explicit unknowns,
  rejected candidates and missing evidence in each handoff.
- Publish additional minimal attribution rows only after the central method's
  evidence/confidentiality review. Corpus-wide mapping and type recovery remain
  incomplete; `BACKLOG.md` retains these prerequisites, not a second task database.

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
- Implemented: the surrounding cold local-player fields restore class scale,
  transfer supplied history, reset pitch, select the initial animation and set
  activation after special construction. Thirty-two native/Rust cases match
  123,792 normalized words across full actor headers/payload and ordered lists,
  with both blending modes and varied copy/equipment/frame inputs. Projection
  reset is explicitly pending. Prototype PS2 naming was searched; attribution
  remains unknown pending independent algorithm/caller review.
- Acquire projection completion and mission frame producers, then copy/held-object
  callbacks and normal player scheduling. Dynamic live retyping is not admitted.
  Preserve unresolved work explicitly; neither an allocated state-3 payload nor
  a completed special phase is a fully initialized player.
- Implemented: Delphi's first-primary physical initializer loads its seven
  declared physical parameters, preserves the acquired opaque inline word,
  stores scale, clears controls and applies the signed upper limit. The shared
  inline default kernel is reused by Mecc. A separate owned Session entry binds
  physics presence without pointer emulation and rejects replay/foreign profiles.
  The original initializer matches 256 seeded native/Rust controls (311,552 full
  header/payload words). This does not execute motion, a complete primary turn
  or copy callbacks; these are still the next controller dependencies.
- Implemented: the standalone Delphi jump/hold/release kernel, including impulse
  gates, launch latch, context-dependent hold acceleration, source-ordered inline
  request reduction, local-time countdown and release flags/height. It reuses the
  shared object-local clock. Native/Rust controls match 3,543,904 full header/payload
  words across 864 scalar cases and 64 persistent sequences (2,048 steps).
  Contacts, orientation/rates, context and external motion remain supplied
  boundaries; gravity, trajectories and Session/live-controller consumption are
  not implemented by this kernel.
- Implemented: owned ordinary dry physical/jump/ANM/history composition uses
  persistent Rust velocity, terrain/world contacts, affine state, class-shared
  event descriptors and the actual 80-record history ring. The generic ANM core
  is shared with Mecc without broadening its admission. Sixty-four continuous
  native/Rust steps match 510,144 normalized words across all 22 actor headers,
  reference buffers, full owner payload and fifteen ordered lists. Real class
  animation bounds are applied on both sides; commands, flat terrain and time
  are supplied boundaries, not native velocity/trajectory outputs. This executes
  the entered primary prefix, jump and physical body, not a full primary turn.
- Acquire primary mode/clip selection, heading/pitch producers, the second ANM
  pass and auxiliary callbacks; compare against the acquired complete selected
  actor iterations. Then acquire water entry/exit, swimming and turbo/energy;
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
  relocate the actor to the mission marker.
  Unknown state-1 classes, legacy profiles and mission state writes remain refused.
- Implemented: the public cold player-field initializer matches 64 native cold
  setup controls (25,856 normalized header/payload words and list counts).
  It uses the common primary-channel selector, preserves cold payload/caches,
  enforces factory ownership and atomic refusal, and returns a pending projection
  reset. This does not implement live type-change cleanup or camera completion.
- Acquired, native only: eight first-primary contexts return with actual auxiliary
  resources; the callback creates two auxiliary actors, nine effects and inline
  physics. A separate 288-case full size-refresh control passes on supplied size
  pairs. The owned size candidate is now implemented separately below; no owned
  first-primary or growth-trigger comparison exists yet. See docs/OWNED_KABUTO.md for the supplied boundaries.
- Implemented: `kabuto_size::prepare` prepares the full debug size-dependent
  parameter/scale/category/radius candidate with Double53 arithmetic, typed source
  resources and atomic refusal. It reuses the shared radius writer. The service
  does not choose a growth target, advance growth or publish a Session phase.
  All 544 native/Rust controls match 219,776 complete header/payload words,
  including variable parameters/scales and absent-class cases.
- Implemented: auxiliary geometry vertex/topology preparation matches 112 native
  controls (173,808 words), preserving generated normals, seams, UV offsets and
  height accumulation. The isolated cold initialization helper is acquired in
  eight contexts separately from the primary remainder. Generated material,
  owner/global-list registration and deformation are still prerequisites; this
  geometry allocation must not be treated as an audio sequence.
- Implemented: the cold auxiliary factory uses actual source classes, constructor
  masks and declared effect-state allocation followed by the acquired empty-state
  destructor/release. Eight contexts match 88 constructor/reset outcomes and
  16,984 normalized header words plus list counts. Nonempty effects remain
  refused atomically; parent/reference binding and complete initialization are
  separate. See docs/OWNED_KABUTO.md.
- Implemented: post-prefix physical generation refresh composes shared inline
  reset, Kabuto parameters and the acquired size/radius tail. All 512 native/Rust
  controls match 206,848 header/payload words. Same-generation absence is retained
  and cannot acknowledge gameplay readiness. Full first-initializer composition,
  parent links and material/animation resources remain pending.
- Implemented: atomic twelve-actor cold group field composition now matches
  16 native contexts (53,728 words), including all actor headers, owner payload,
  typed links, geometry buffers and ordered lists. Late failure preserves the
  existing Session; clear/re-entry and generation-without-physics checks pass.
  The group's pending camera/animation-cache/material/global-geometry services remain
  explicit; this is not full first-initializer behavior or gameplay readiness.
- Complete first-initializer behavior beyond the acquired field group: animation
  definition-cache lookup/clamping/reset lifetime, generated material/global
  geometry registration and camera reset
  consumption. Acquire the native Kabuto-specific teardown and complete primary;
  the current release proof covers whole-Session clear only.
  Reuse `physical_initial::reset`. The native effect auxiliaries pass through
  their declared effect state before becoming inactive; preserve the acquired
  transition/cleanup when composing the group, rather than using only a final-state seed.
  Acquire marker relocation and camera consumption, then land movement/jumping,
  size-dependent collision and actual feeding/growth triggers.
  The special dispatcher returns for Kabuto without Mecc/Delphi child creation.
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
- Subjective dispatch: the optional primary now requires actual camera ownership
  and pre-world display status separately from camera mode. It refuses a matching
  active display before primary mutation because the required subjective
  post-world execution is unbound. Predicate controls match 512 PC/PS2-body/Rust
  cases; 128 native pre/post observations show the correct transfer and include
  first-person mode with a missing subjective model. The existing 576-frame,
  22-shot primary matrix remains identical. Persistent subjective composition now
  produces acquired virtual channels, header/ANM stamp, placement/affine and two
  ANM passes around a required fire-service boundary. Two continuous controls
  match 128 frames / 42,880 header-reference words, with body channels, cameras,
  selector getters and a no-edit fire boundary supplied. Bind the real deferred
  primary obligation and actual local fire, not presentation playback. Retain the
  explicit initial camera-owner and partial-world acquisition boundaries in
  docs/OWNED_LOCAL_FIRE.md; main `fire: None` remains.
- Subjective shared storage: native observations confirm that source state and
  reference matrices are shared while channels remain independent. The acquired
  reference composition now writes the authoritative source cache, preserves
  undeclared slots and invalidates the old body's sample binding. Two reference
  controls match 96 frames / 26,112 words; four failure and one empty-primary
  control preserve publications. See docs/OWNED_LOCAL_FIRE.md. Next bind the actual
  pre-copy/header and shared state to the world scheduler; the new Session-bound
  composition produces both ANM passes and placement but its actual fire callback
  remains required. Acquire posed-vertex storage
  and its exact consumers before choosing their ownership; do not substitute
  independent playback/vertex copies for an unverified sharing model. Current controlled flags are read after
  primary; copied motion/channels and world-shared weapon history have distinct
  producers. This remains a CPU prerequisite, not full subjective integration.
- Ordinary camera: the owned mission inspection driver now owns sequential
  direct-angle camera production after primary/held updates, using actual scene
  terrain and prepared rigid meshes. The presentation consumes the generated
  view/cache instead of recomputing it from the player look. Camera and clipping
  CPU controls pass 64/256 cases respectively; see docs/OWNED_CAMERA.md and the
  receipt-bound runtime checkpoint in docs/RELEASES.md. Normal campaign admission,
  native list/model/part preparation, configured angle smoothing, special camera
  modes and pre-primary/subjective firing timing remain prerequisites. The
  existing inspection gate and `fire: None` remain; this is not a default-route
  owned readiness claim.
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

- Published preview 2026.11.05.6 / FX-prefix milestone validated: the real source/ancestry audit covers 208 active
  requests in 72 sources. The former guard conflated Flick34 shadow descriptors,
  text state, ordinary draw range and numeric audio with placement mutations.
  Two command32 records affect actors outside their FX carrier ancestry. Typed
  framing/payload checks now admit these non-placement prefixes, while relevant
  carrier/ancestor offsets, scales/animation changes, unknown commands and cycles
  remain refused. Retained shadow construction research is reused, not converted
  into a claim of shadow raster fidelity.
  The full 545-source inventory now reports zero prefix refusals, 56 general
  nonterminal FX queue refusals, and 59 sources with implementation/preparation
  gaps (486 without reported gaps remain unverified). Fifty sources moved to the
  next real blocker; only two additional terminal-only bindings became admitted.
  Strict-profile review stays separate at 282 sources; zero complete scenes are
  certified. The 52-source bucket is not 52 corrected renderings.
  Exact-binary silent controls pass both 54-director natural/re-entry cycles for
  Story4 mission 2, both Story4 mission 1 control cycles and both development
  endpoints. The previously refused terminal pair actually spawns/retires in
  director42 on both long-chain entries. Text atlases and tracked film assets
  release; total GPU/process memory reclamation and native timing remain open.
  The long chain still ends in its explicit Mecc partial preview, with readiness
  gates intact. The control still reaches Gameplay.
  Next: the 56-source general queue. A private program survey separates 22 root
  programs, including a bounded persistent-attachment family with retained native
  construction/startup controls. Existing visual preview is not sufficient to
  acknowledge live queue, owner, opaque constructor fields or cleanup contracts.
  The source-bound follow-up finds 28 persistent requests in 26 sources. Twenty-
  three sources across four worlds contain only that nonterminal family (25
  requests); three sources also need other families. All 28 use attached mode
  and pass the existing initial class-facing profile. Prioritize that coherent
  23-source cohort for shared queue/backend integration, then mixed families.
  These are implementation candidates, not newly admitted or visually verified
  sources. Reused native startup controls supply placeholder owners; real owner
  updates, world admission, slot reuse and source-marked cleanup still require
  their own integration evidence. Prototype PS2 candidate names were checked
  against the pinned ELF before continuing; correspondence remains unknown.
- Owner's active work order (5 November 2026): address the shared 52-source FX
  prefix refusal first, then general queue admission and partial FX services,
  followed by remaining audio/credits/resume prerequisites. Work alone, strictly
  in the background. Re-run the whole-corpus inventory and actual silent menu
  lifecycle/transport controls after coherent integrations, updating this backlog,
  PROGRESS, the dashboard and receipt-bound MinGW previews. Keep unknown visual,
  temporal and functional behavior explicitly open; do not waive a real mutation
  or readiness guard merely to reduce an inventory count.

- Previous whole-corpus inventory baseline on the recorded public-runtime source: 545
  sources / 16 worlds, zero top-level preparation errors, 61 sources with a
  reported implementation/preparation gap. The 484 others remain unverified;
  they are not certified accurate. The independent strict-profile review queue
  contains 282 sources and overlaps the gap set. See the aggregate dashboard
  table and docs/cinematic-inventory-summary.json; per-source names, reasons,
  command counts and evidence remain in private JSON/TSV/HTML reports.
- Active priorities after the prefix correction (counts overlap):
  - General nonterminal FX queue: 56 sources exceed the admitted
    terminal-only profile. Connect real creation, update, retirement and ownership
    through the main dispatcher; partial preview geometry is not complete binding.
  - Partial visual FX services: 15 sources / six worlds retain contacts, sounds,
    trails, texture/density/cadence or cross-director lifetime limits. These counts
    overlap the refused binding set; do not add them as distinct broken scenes.
  - Numeric audio: 23 commands in two sources lack a live Flick consumer. Retained
    native table snapshots found the concerned IDs absent at sampled instants;
    audible impact at every dispatch remains unknown. Do not invent sound aliases.
  - Credits: nine commands in two sources lack the cinematic credits renderer.
  - External FX resume: one source still requires a real resume producer; do not
    substitute an arbitrary delay.
- Inventory review controls excluded 265 specialized attachment ownership routes
  from missing-geometry counts. Another 28 empty visual-preview requests across
  14 sources have admitted terminal-only mission bindings: no particle geometry
  is expected from those programs. Two remaining visual-preview refusal sources
  stay in the review queue. Strict-parser rejection is not evidence of a missing
  live draw, and the legacy opaque_blocks metadata field is effect-slot capacity.
  FLK and MDV opcode spaces remain distinct; this batch does not audit MDV music.
- Continue visual/reference comparison for all 545 sources, with exact source,
  terrain, build, camera and time attribution. Lighting, shadows, water, materials,
  temporal behavior and complete functional fidelity stay unknown until compared.
  This batch changes diagnostics only; it does not add cinematic playback features.
  The subsequent merge preserves the separate cold Kabuto lane; its combined
  source passes 705 Rust tests (16 ignored), 30 Python tests, formatting and
  Clippy. These checks do not relabel the pinned 545-source audit as a fresh run
  of that later executable.

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
  The .4 matrix stopped after 24 completed, passing roots out of 29; its last
  partial case and remaining roots are unverified. Transport passes do not
  certify complete scenes or all 545 sources.
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
