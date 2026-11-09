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
- The central method records current primary/subjective function
  lookups as unassigned, with their exact PC aliases, current owned Rust
  relationships and missing comparisons. Reuse those identity-bound searches;
  preserve older presentation-module references as historical aliases rather
  than attributing the owned pipeline to that presentation implementation.
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
- Implemented: the owned dry sword locomotion phase composes physical/history,
  the acquired secondary selector and a second zero-time ANM pass, preserving
  latched jump states, random idle/walk variants and clip-based reselection.
  Six continuous 256-step native/Rust cases match 12,243,456 normalized words
  over all 22 actor headers/reference buffers, owner payload and ordered lists,
  plus shared RNG endpoints. Rust owns its trajectory, contacts and history;
  native snapshots are outputs only. Clip identifiers are supplied definitions,
  checked against the owned class resources; no original mode table is bundled.
  `delphi_animation_definitions::Definitions` now reads both equipment columns
  from two independently inspected external PE data layouts. The shared bounded
  reader preserves Kabuto's three existing layouts. All 188 identifiers per input
  match the inspected data references; a retail-metadata replay retains the
  reference-profile behavior. No original tables or executable are bundled or run.
  Other Delphi executable layouts remain unacquired; header recognition is not
  authentication or execution proof for every build.
  Impact/slide effects, nonordinary actions, water and equipment changes remain
  explicit unsupported boundaries. This does not execute the full primary.
- Implemented: the manual heading/look kernel matches 2,531 supplied-boundary
  cases / 488,483 header words in Double53. Its explicit owned controlled profile
  composes entered prefix -> heading/look -> jump/physical/history -> secondary
  -> zero-time ANM. Five 256-step comparisons match 10,202,880 normalized words
  over 22 actors, references, payload and ordered lists, plus shared RNG endpoints.
  Controls cover signed input, wrap/clamps, pause/unpause with held jump, variable
  global time and ordered slow areas/owner exclusion. Look stays actor-owned;
  actual class rotation rates and source-position object time are consumed.
  Paused physical work keeps first ANM/history but skips integration/placement/
  new rates/forces. Three-owner lifecycle/profile/quarantine checks pass.
  Automatic target heading, full primary and live admission remain pending.
- Implemented: controlled body pitch/roll alignment follows jump and precedes
  physics, retaining the acquired conversion factor, shortest-turn normalization,
  object-local time and paused-frame ordering. Four 256-step tilted-start cases
  match 8,162,304 normalized words across the complete 22-actor cohort and RNG
  endpoint. Scalar verification separates 3,484 original-instruction emulator
  cases from a 3,964-case hardware arithmetic translation; the documented huge-
  angle emulator discrepancy is excluded from original-instruction parity.
  Special pose inputs do not implement water detection, swimming or turbo.
- Acquire paused positional-event lazy transform production before admitting
  events needing an absent/stale affine cache. ANM preserves cold absence and
  the older cached pose during body settling; no placeholder matrix is used.
- Implemented: the common owned reference-camera sampler and Delphi's local/mode
  gate, current-reference/animation1 fallback, reset/retained-view and supplied
  target branch. A 2,048-case cached-reference arithmetic comparison matches
  36,864 matrix/lens words. The frozen authored-camera profile now executes the
  callback inside the controlled Session phase after the prefix and before heading,
  sharing the ANM epoch. Three 256-step combined-entry compositions match 6,137,856
  normalized actor/reference/payload/list/override words plus exact RNG. Fallback
  sampling advances the shared ANM epoch. Mode/aspect/local-owner and self-target
  are supplied boundaries; target selection, viewport production, production
  full-primary ordering, normal camera/render/mission wiring remain open. The
  four-owner lifecycle check covers profile refusal, unconsumed invalid inputs,
  foreign targets, retained completed camera effects and replay quarantine.
  Fifteen repeated legacy controls preserve 30,608,640 words and exact RNG.
  These APIs do not admit live Delphi controls or establish visual fidelity.
- Implemented: the owned ordinary third-person/direct-angle camera slice binds
  the completed controlled actor's pose/look/flags and reuses the common camera
  kernels with Delphi's distance policy. Scalar controls match 5,566 cases /
  11,132 words; the complete native normal entry matches 768 steps / 14,592
  pose/chase/cache words with floor excluded, no-hit walls and captured final
  view arguments. A separate class-2 control preserves 768 steps / 14,592 words.
  The immutable base profile loads read-only from acquired external debug data;
  no table or executable is bundled. Source-data checks exercise 64 owned camera
  updates and pre-provider frame/profile refusals. Full camera dispatch, target
  selection, dynamic profile/FOV/projection, underwater floor, angle smoothing,
  shake, real world providers and renderer/mission wiring remain prerequisites.
- Implemented: common lens transition/scaling and finite PC perspective kernels,
  composed in source order. Controls match 25,091 lens cases / 50,182 words,
  9,720 projection cases / 184,680 words and 1,571 complete lens-to-matrix calls /
  31,420 words. Debug and optimized probes repeat the same native inputs.
  Actual PC profile/kind helpers execute in controls; profile FOV, input flags,
  viewport and depth terms remain supplied boundaries. Only device publication
  is skipped. Override preserves the transition; projection refusal retains a
  completed lens update. No renderer or visual-fidelity claim is made.
  The next composition below binds the ordinary profile selector and invocation
  order. Actual input/viewport/depth producers and the common Bevy projection /
  coordinate adapter remain pending. Full dispatch and alternate branches stay open.
- Implemented: the owned local class-3 mode-zero ordinary view selects external
  base/scaled profiles, then executes lens, projection and direct-angle pose in
  source order. The actual native dispatcher branch matches 384 invocations /
  14,976 words across three persistent sequences. A separate 384-call control
  executes original view/cache math and matches 23,424 words, with each next Rust
  depth input produced from its own prior Rust view. Initial cache remains supplied.
  Device publication is skipped,
  walls are no-hit and final view arguments are captured. The source lifecycle
  adds 64 composed owned views with actual actor fields/kind and pre-provider
  refusal checks. Completed lens/projection effects survive later failure and
  cannot replay. Actual input/viewport/depth producers, initial view, complete
  dispatch and renderer/mission scheduling remain prerequisites; no live Delphi
  admission or fidelity status changes follow.
- Compose the surrounding full primary, camera/target/item producers and
  auxiliary callbacks against the acquired complete selected
  actor iterations. Native-only acquisition now traces 3,009 original primary
  calls and 576 phase calls without changing the original 64 final cohort states.
  Three additional 256-step selected-owner controls cover landing, return to rest
  and both random idle variants, with all 22 actor snapshots retained. Supplied
  Windows TLS/error APIs permit the original CRT lookup and RNG instructions to
  run; the earlier missing-environment stop is not a game defect. This is not a
  Rust full-controller comparison. Preserve Delphi's latched jump states and
  source-ordered post-physical secondary/second-ANM selection rather than copying
  Mecc locomotion. Acquire the remaining camera/target producers and callback
  composition with owned state, then water entry/exit, swimming and turbo/energy;
  compare controlled continuous cycles before admitting live controls. Reuse
  shared terrain/collision/input services with the acquired character parameters.
- Connect the validated controller through the existing selected-mission path,
  camera/HUD/audio and return/second-entry cleanup. Then acquire combat and spells.
- Keep Delphi movement/campaign roadmap rows at `todo` until actual runtime
  integration exists. Resource loading alone must not change completion status.

### Kabuto next steps

- The dedicated user-created fork owns Kabuto only, working alone with an
  isolated Cargo target. The owner permits necessary test windows; avoid unrelated
  focus changes and sound. Preserve the
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
- Implemented: explicit external executable-data provider and immutable cold
  animation-definition cache, reusing shared ANM reference resolution. Three
  inspected PE layouts produce the same 62 normalized records as the native
  reference (61 resolved nodes), including ranges, flags and opaque parameter
  prefixes. No source table or native instructions are bundled. The optional
  factory/group cache retains specification ownership and releases with its
  owners. Native warm/reset controls preserve the cached records; process/world
  reset scheduling is still pending, so `Pending::AnimationCache` stays explicit.
- Implemented: owned cache lifecycle now preserves cumulative bounds and prior
  node-source ownership across reset/rebind, with atomic failed refresh and warm
  identity guards. Six native phases match 372 normalized records, including an
  absent animation after reset. The actual Kabuto hook is removed by native reset
  dispatch, deduplicated on registration and registered again on reuse. Group
  composition refuses a supplied provider awaiting refresh. Full world teardown
  ordering and runtime reset dispatch remain pending; no readiness gate is cleared.
- Implemented: normal cold procedural texture generation reuses the shared RNG
  and matches 64 native seeds (131,072 source pixel words and 64 final RNG states).
  The Factory/group shares the CPU image, consumes no draws on reuse and releases
  it with its owners. Native constructor controls also observe two-entry geometry
  insertion and the already-resident material bypass. GPU material interpretation,
  that bypass in the owned backend, world lifetime/reset and geometry registration
  remain pending; CPU image data does not clear `Pending::ProceduralMaterial`.
- Implemented: ordered normal-cold CPU geometry registry, last-entry material
  retirement and regenerated RNG phase match 32 native scenarios / 144 removals.
  Registered group construction stages actors, registry and RNG atomically.
  The acquired custom cold cleanup reuses common deletion marking, preserves
  actor/list ownership, and retires its geometry/image. Eight complete native
  group contexts match 36,624 normalized words, all lists and 88 mark calls.
  Same-ticket foreign scopes, replay and dirty/partial publication refuse.
- Implemented: normal geometry-entry submission prepares the acquired transposed
  transform and seven ordered backend commands using the common basis/matrix,
  SIMD trigonometry and shared RNG services. Sixty-four native controls match
  24,876 words, including complete vertices/indices and the final RNG state.
  Disabled entries consume nothing; owned invalid profiles refuse before RNG
  publication. Backend failure retains the completed prefix and consumed draw.
  The scalar/mode kernel is acquired below; registry visitor/world scheduling and real GPU
  consumer remain prerequisites; this is a CPU packet, not rendered Kabuto.
- Implemented: the geometry-header tail selects the supplied local-camera or
  body/target route, preserves inactive fields, transforms the current attachment,
  and measures the target distance. All 384 native controls match 4,992 complete
  header words across modes, selected/unselected owners, target presence and
  matrix flags. The shared Double53 point kernel preserves its distinct operation
  order without changing the existing aiming kernel. The scalar/mode kernel is acquired below;
  animation/sound composition and active-extra teardown remain open. Header preparation
  alone does not complete this helper, full primary or world rendering.
- Implemented: the scalar/mode and target-reference phase matches 768 native
  controls, 7,816 final/notification words and all 209 notification calls. It
  preserves source bit toggles, Double53 timer stores, scalar updates, identity
  comparisons and notification-before-target / mode-before-notification order.
  The eligibility guard retains the first matching peer, fallback and low-byte
  owner comparison. Transport is an explicit supplied boundary, not a network
  implementation. Five synthetic controls cover ordering, inactive bypass,
  atomic refusal, signed zero and unchanged reference identity. Actual Session
  field binding, preceding/following transitions and world dispatch remain open.
- Implemented: separately prepared class1199 auxiliary construction uses actual
  class/specification resources and the shared Session constructor. Sixteen
  native contexts match 3,872 normalized header/list words. Its required class
  command `0x4e` is a one-byte mask operation; 512 seeded/mixed native controls
  match the public configuration kernel. No declaration bytes are bundled.
- Implemented: atomic owner binding and explicit mark/reference-clear use the
  common ticket-owned actor IDs, attachment metadata and deletion-bit writer.
  Twenty-four native geometry-prefix contexts match the auxiliary lifecycle
  slice (11,596 words, four creates, three marks, every extra header and all
  fifteen lists). Marked actors retain ownership and attachment; re-entry
  allocates a new ID. Active-extra group cleanup is refused until that teardown
  profile is acquired. This auxiliary-only receipt excludes animation/sound/
  epoch/event comparison; the owned transition receipt is recorded below.
- Implemented: the standalone template clock preserves base-before-truncation
  and the signed-64 conversion's low word; 512 native cases match, including
  signed coefficients and wrapping/indefinite conversions. The existing channel
  converter retains its separate arithmetic. The Session geometry-gate query
  uses its current primary channel and immutable owner descriptor, refusing
  foreign/stale, retired and nonselected contexts. This query alone does not
  advance clocks or publish references.
- Implemented: owned geometry transition composes upstream mode publication,
  ordered selections/sound-service calls, the 1,600-tick gate, typed auxiliary
  create/mark and the shared actor-local ANM/reference/epoch/event pipeline.
  Eighty-four native contexts match 23,322 words over four channels, eleven
  reference matrices, epochs, auxiliary flags/links and fifteen lists, including
  enabled/disabled blending and below/equal/above-threshold cases. The sound bank
  is absent in these controls; device audio and fired-event effect cadence are
  unverified. Upstream mode/physical/contact producers and time areas are supplied
  boundaries. Late provider failure retains prior publication and quarantines
  replay. Cold-only cleanup refuses entered transitions. Full primary, dynamic
  cleanup, geometry-header/render consumption and live admission remain open.
- Implemented: second-auxiliary normalized-pose selection uses the shared channel
  setter, source distance arithmetic and ordered local-list minimum stores. The
  public kernel matches 1,024 original-helper controls / 197,632 full child-header
  words, including absent-child, local/remote, signed-radius and finite-extreme
  profiles. Actual Session child/size/pose/list binding matches another 64 source-
  resource contexts / 6,720 channel/clock/list words. Root identity and preceding
  poses are supplied; this does not produce a live local player or world pose.
  Invalid inputs/foreign identities refuse before publication. Owner payload,
  allocations, lists, unsampled references and pending gates remain unchanged;
  entered subphases refuse cold-only cleanup. Next acquire the following helper,
  physical/mode producers and full primary; ANM cadence and movement remain open.
- Implemented: factory-bound input/orientation and force-to-position prefix now
  owns its model/velocity/history fields and reuses common primary history,
  actor orientation, object time, integration and position limiting. A 384-context
  typed projection matches 89,472 words; a separate 384-context Session control
  matches 155,136 complete owner-header/payload words. Incoming forces, resolved
  commands, preferences, poses and ordered time areas are supplied boundaries.
  The owned profile is unparented; the kernel separately covers frozen-parent
  behavior. Source input-mask order and the opaque header-mode gate are retained.
  The prefix consumes a real history phase, publishes atomically, and refuses
  replays/new histories while contacts, classification, new control forces and
  the primary remainder have no consumer. All four initializer gates remain.
- Implemented: factory-bound target decision reads the actual owner state threshold
  and target, preserves the shared wide integer conversion and four-value wrap,
  and publishes the target before its notification and the reset after it. The
  existing notification guard is reused. A 512-context kernel control matches
  2,509 words / 237 service-boundary observations, including old/new state timing.
  Another 512-context actual Factory/Session decision + size + auxiliary control
  matches 315,440 words. Sound executes the original null-bank return; notification
  transport is supplied. The threshold field's producer and gameplay role remain
  unassigned: no feeding, keyboard trigger or complete primary is inferred.
- Implemented: factory-bound size pursuit publishes the supplied target, retains
  actor-local time as Double53 through the fixed step, clamps current size and
  invokes the existing complete parameter/radius refresh whenever the incoming
  sizes differ (including zero time). Equality bypasses time selection. Composed
  with second-auxiliary selection, 1,024 native contexts match 627,712 normalized
  owner/child header, owner-payload, physical-buffer and refresh words. One
  512-step trajectory reaches all four sizes and retargets/shrinks; controls
  include ordered time areas and owner-skip/first-hit behavior. Source targets,
  preceding poses and root identity are explicit inputs, not growth/feeding rules.
  Owned factory identity, invalid-input rollback and clear are checked. No cached
  world collision/affine refresh, target producer or live growth is acknowledged;
  full primary, following helper and movement remain open.
- Complete the remaining custom cleanup profiles: the active additional-reference
  teardown, GPU material release and dynamic/entered turns. The acquired geometry
  prefix stores class1199 in that reference; its broader gameplay role is unassigned.
  The owner remains state1 after the custom callback; common state transition
  and actual marked-actor retirement must be acquired separately. `Session::clear`
  alone does not dispatch the explicit external geometry registry.
- Complete first-initializer behavior beyond the acquired field group: animation
  cache/reset service lifetime, generated material/global
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
  and pre-world display status separately from camera mode. The immediate API
  retains its active-display refusal. The new Session-bound route requires a
  genuine pre-copy and defers body fire without prematurely resetting auxiliary
  fields/clocks. Its pending post blocks the next frame. The actual held pass,
  virtual ANM/shared publication, local target/fire/projectile/FX and zero-time
  tail now compose at CPU scope. Four controls match 320 frames, 144 active
  subjective updates, twelve shots and 391,552 normalized snapshot words, with
  camera production/eligibility and partial-world population/order supplied;
  positional sound is explicitly absent. The 576-frame/22-shot immediate matrix
  and ten failure controls remain identical. A source-backed lifecycle control
  covers genuine preparation, current held ownership, no-shot/successful fire,
  retained late sound failure, pending-post blocking and replay refusal.
  Predicate evidence remains separate: 512 PC/PS2-body/Rust cases and 128 native
  pre/post observations. Main `fire: None` remains; bind actual camera production,
  posed-vertex/drawing policy, scene contexts and audio before main admission.
- Subjective shared storage: native observations confirm that source state and
  reference matrices are shared while channels remain independent. The acquired
  reference composition now writes the authoritative source cache, preserves
  undeclared slots and invalidates the old body's sample binding. Two reference
  controls match 96 frames / 26,112 words; four failure and one empty-primary
  control preserve publications. See docs/OWNED_LOCAL_FIRE.md. The Session-bound
  post now executes the actual fire callback using the current held cache rather
  than reattaching from overwritten source references. The position producer now
  uses the existing model-resource bank and actual virtual channels/stamp. Acquire
  its real draw consumers and admission; do not substitute an independent
  presentation player. Current controlled flags are read after
  primary; copied motion/channels and world-shared weapon history have distinct
  producers. This remains a CPU prerequisite, not full subjective integration.
- Ordered subjective camera: the Session-produced post now calculates the inner
  view after selector execution and before placement/ANM/fire. Its world owner
  tracks camera invocations separately from world frames, then publishes the
  matching outer view for rendering. Four controls pass 320 frames/12 shots with
  generated cameras; a separate 64-frame capture also checks inner renderer
  matrices. Preserve explicit startup/eligibility/static-world boundaries. Next,
  bind the acquired position producer to actual subjective drawing before
  enabling main-driver fire; complete target contexts and positional audio remain.
- Subjective vertices: the Session API refuses incomplete, skipped, failed,
  inactive, stale, foreign and cleared post states. Two controls pass 64 frames,
  544 model snapshots and 716,414 words, including full positions/cache and the
  earlier world/fire snapshots. Forced and mixed cache gates are explicit inputs;
  live LOD, part masks, material submission and world draw policy remain open.
  The same trajectory exposed a backward-air secondary transition, now acquired
  for the normal pistol. Other directional-air variants and reactions remain
  separate prerequisites. These CPU checks enable no new campaign or live fire.
- Subjective submission: the class-2 body/virtual dispatcher and temporary held
  parent redirection now match 8,192 + 144 native cases. Callback interception is
  not geometry execution. Next bind actual completed-post/held identities and
  acquire the preparation and immediate/deferred model consumers. Reuse existing
  `render_parts`, `render_sort`, `lod` and the shared model bank; the observed
  submission entry is distinct from the previously examined draw helper. Keep
  other-character rules separate and retain detached parents and late failures.
  Main-driver startup camera, target contexts and positional audio remain open.
- Subjective held pose: provider inspection identifies the held wrapper as an
  attachment/world-pose phase, not graphics submission; `submit_held` remains a
  deprecated alias of `update_held_pose`. The normal pistol now samples the actual
  completed virtual affine and shared references without advancing ANM or changing
  its real binding/frozen firing cache. Two 32-frame controls match 32 matrices
  (544 words); position/history/spatial finalization is intercepted. Lifecycle
  checks refuse stale/foreign/cleared/skipped/numeric and replaced-reference input.
  Next acquire the enclosing held pose/finalization scheduling and connect actual
  body/subjective/held rendering through live culling, LOD, parts and materials.
- Owned body models: `mecc_model_draw` binds the actual completed outer camera,
  source/specification and shared-channel positions to the main inspection
  renderer. The native selector and full model snapshots match 1,280 selections /
  256 snapshots / 309,472 words. Six receipt-bound Bevy endpoints cover automatic,
  forced and absent models; model 0 uses 736 vertices/eight parts, model 7 uses
  57 vertices/six parts. Quality/override settings remain explicit inspection
  inputs. See docs/OWNED_MODEL_DRAW.md. Next bind native culling, node/part masks,
  material/normal context and immediate/deferred scheduling; model selection
  alone is not full world or subjective draw admission.
  Migrate the retained model-0 diagnostic presentation consumers before claiming
  native shared-bank ordering across actors; this inspection still prepares that
  diagnostic pose separately from its selected-model upload.
- Extended owned movement remains incomplete. The inverted secondary height
  comparison is corrected after 1,049 controlled original-PC branch executions;
  both velocity and height thresholds are strict. The corrected general replay
  previously refused the unacquired high-descent transition at step 143.
  That ordinary transition now matches 128 complete native-selector/channel
  controls with blending disabled/enabled (41,920 words), including hold and
  precedence over later reactions. The ordinary contact selector now owns logical
  state 60 and a real prepared slide-effect context. Native controls match 14
  selector calls / 4,622 words and 256 preparation/empty-cleanup pairs / 17,920
  words, including the previous-state delay, held-channel preservation, source
  ownership and three shared RNG draws. See docs/OWNED_MECC_CONTACT.md.
  The actual owned Bevy replay prepares its context at step 159 and reaches step
  190, where the actor crosses below zero height and the acquired ordinary profile
  explicitly refuses further updates. Retain the earlier step-143/152/158 failures.
  Next acquire the original subzero primary/secondary path and its actual
  physical/environment/health inputs; do not assume water, death or a clamp.
  The living selector's absolute-negative-height refusal is now removed after
  144 original/Rust controls (47,520 words). A 128-frame native acquisition agrees
  with Rust for 40 frames, including seven negative-height endpoints, then reaches
  another still-refused object-flag profile. Preserve that failed full comparison
  and acquire the following flag/state/health path; this is not a completed run.
  The new exact Bevy replay passes the former step-190 gate, stops at step 242
  without a device, and reaches step 348 with the real reserved/muted backend
  (three actual sound starts). The later unequipped depletion acquisition now
  covers all admitted control-mask combinations: 288 native branch controls
  invoke no services and only zero fuel; 1,536 jet-prefix controls still agree.
  Source f24febd passes step 348 without fuel refill or dummy effects, reaches
  step 441 with seven actual muted sink starts, then refuses the independent
  impact-reaction provider. Acquire that reaction before extending the replay;
  equipped shield/cloak depletion remains excluded. Long-run cancellation/re-entry
  remains unvalidated. The shared positional hard-landing preparation now matches
  256 native controls / 512 operations / 17,920 words, including its integer cue
  draw and two ordered floating intervals. Invalid late operands preserve RNG.
  This is preparation only. Bind the actual impact damage context and retained
  health/channel writes, contact precedence, positional cue transport, active
  effect execution and emitted-object cleanup before admitting the reaction.
  Keep the low-floor flag/state/health
  acquisition as a separate prerequisite rather than inferring it from this path.
  Bind the prepared effect's later active interpreter, emitted-object cleanup
  and real world scheduling, then independent impact reactions. Prepared-only
  cleanup is not active-effect cleanup. The 128-step inspection
  cycles cover resource retirement separately and do not validate this trajectory.
- Ordinary camera: the owned mission inspection driver now owns sequential
  direct-angle camera production after primary/held updates, using actual scene
  terrain and prepared rigid meshes. The presentation consumes the generated
  view/cache instead of recomputing it from the player look. Camera and clipping
  CPU controls pass 64/256 cases respectively; see docs/OWNED_CAMERA.md and the
  receipt-bound runtime checkpoint in docs/RELEASES.md. Normal campaign admission,
  native list/model/part preparation, configured angle smoothing, special camera
  modes, enclosing world admission and main-driver subjective fire/drawing binding
  remain prerequisites. The
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
  Three reviewed engine captures and the cover now render directly in HTML;
  the complete comparison gallery appears before the progress tables.

## Cinematics / Flick: coverage and next prerequisites

- Persistent selector follow-up: 900 private original-PC/Rust controls now
  execute the real model selector and prepared-format query instead of supplying
  slot zero and a positive alpha answer. The controlled descriptor formats cover
  0 through 8, with automatic/forced selector entry modes, birth and four owned
  preparations, and near/boundary/far draw-range cases for two source programs.
  All 2,376 format answers and 1,188 model selections match the existing public
  kernels. The source model table and both nonzero material-routing scalars also
  match resources in all four cohort worlds. Within this bounded domain the
  eligible parts remain deferred across all supplied formats; this does not
  acquire the actual resident descriptor producer, GPU preparation, global queue
  ordering or pixels. No runtime code, dependency, downloadable capability or
  inventory count changes; the exact preview and receipts above remain separate.
  Prototype PS2 searches precede this follow-up; attribution stays unknown.
  Retained ordinary-world population evidence remains diagnostic: the exact-
  binary capture contains only a post-attachment sample, while the older loading
  trace has a separate, incomplete identity chain. Neither proves an atomic
  allocator census or whole-world admission. Next: acquire identity-bound world
  population/scheduling and actual texture/material consumers, then the general
  and mixed queue families. Do not derive a shared quota from a list length.

- Persistent draw-traversal milestone: first owned constant-effect preparation
  now consumes the acquired constructor gate and publishes active geometry,
  including zero-delta preparation. The common film submits each bounded
  attached part through the ordinary and adjacent effect traversals. Only the
  ordinary route uses the current source camera/director range; both use the
  same owned matrix, channel, mesh and material, without a second clock.
  Private original-PC/Rust controls match 50 actual source-object part-selection
  cases and 14 preparation states, with supplied camera, alpha-query and model-
  slot boundaries. Both entry/release cycles of all 23 sources / 25 requests
  pass near/far route checks. Prototype PS2 lookups remain unknown attributions.
  Exact GNU source `da01d3b` passes three live-object cancellations, both
  development endpoints and two Rstory1 mission-index-2 source-terminal/re-entry
  cycles with silent voice/text release. Actual muted menu controls on Rstory5
  original mission index 2 pass two 19-director natural endings, then a third
  entry with double-click after birth and both rendered routes, Escape back to
  the menu and tracked cleanup. Three profile files remain unchanged after
  initial unlock. These controls retain the partial free-camera endpoint and
  gameplay refusal. Original global sort, raster/material equivalence, complete
  world occupancy/admission, native cadence and full visual/audio/subtitle parity
  remain open. Draw counts do not certify a scene or reduce the gap inventory.

- Persistent attached-FX implementation milestone: the main film now consumes
  actual FLK39 events through source-marked, ticket-owned contexts and objects,
  using the shared attached constructor and current owned channel/reference
  outputs in the common renderer. Retiring a context keeps its object alive;
  repeated source slots retain earlier objects on the same current support.
  Director changes and film release discard the marked objects without lifetime
  children. The existing mission-readiness refusal is retained.
  The refreshed whole-corpus inventory on source `da01d3b4306a168ea6aecfbcf864a7be6167a8e6`
  prepares all 545 sources / 16 worlds with zero top-level errors; it records
  23 bounded persistent sources / 25 requests. CPU creation/update/retirement
  controls pass both entries of all 23 (50 objects created and released), and
  find no later carrier/ancestor link, offset or scale commands in those 25
  requests. Four native/Rust supplied-support matrices match all 68 words.
  Actual native source-helper reuse controls retain both objects on one support;
  fixtures supply zero actors, resident resources and outer allocation services.
  Prototype PS2 lookups precede this work; correspondence remains unknown.
  Main-menu natural/re-entry controls pass both Rstory1 mission 3 and Kstory1
  mission 1 on that exact executable, with real persistent births, owned clocks
  and zero remaining objects/contexts at release. Rstory1 also records its actual
  visible mesh parts. Both 54-director Story4 mission 2 natural/re-entry cycles
  also pass on that executable, with persistent/terminal dispatch and tracked
  resource cleanup. The subsequent exact GNU source
  `0b050fbcf89eeae0ff0dd473133d496ac3496aa7` passes three service cancellations
  after actual persistent objects advance, with zero remaining objects/contexts,
  two Rstory1 natural/re-entry cycles and both development lifecycle endpoints.
  The cyclic observer now stays pending until its requested object exists;
  whole-world presentation and one-shot capture completion cannot end this probe
  early. This tests cancellation services, not interactive skip gestures.
  A later silent interactive control of that same preview uses the actual main
  menu and Rstory1 mission 3: Escape skips after a persistent object is born,
  a second entry reaches natural completion, and a third entry skips with a
  double-click before the object is born. Each returns through the partial
  free-camera scene to the mission menu; tracked session entities and owned
  assets are released, with zero objects/contexts remaining. This does not admit
  Delphi gameplay or verify other scene/gesture combinations, audible playback,
  native pixels or whole-device resource lifetime. Double-click with an active
  persistent object was not tested on that earlier preview; the later combined
  build's Rstory5 completion control below covers that gesture separately.
  Native support-generation acquisition now covers 153 bounded producer cases
  (2,461 generation words) plus 20 original command-processor wait cases (1,294
  calls / 1,274 time updates). Within supplied silent scene/header boundaries,
  an unsatisfied wait updates the clock and renews director then actor generations;
  a satisfied wait dispatches onward without that update. Initial clock reset
  still renews generations. Actual source construction/render/cache phase and
  world scheduling remain prerequisites; these controls add no runtime behavior.
  Identity-bound prototype PS2 lookups remain unknown, separate from PC behavior.
  Follow-up source-actor controls cover 158 actors in all 23 bounded sources.
  Forty have independent channel clocks, but the source/reference audit finds
  none used by these 25 FX supports. With actual source camera binding, the
  original time/epoch routine and actual actor-channel updates, four supplied
  phase deltas produce 100 support samples / 1,700 words matching both the
  explicit-clock Rust projection and the current runtime clock selection.
  Twenty-five initial support caches are cold; a clean repetition agrees.
  This closes no whole-world admission or fidelity requirement. Full VM/frame
  scheduling and capacity remain to acquire; do not add
  a synthetic delay, first-frame pose or independent-clock substitution based
  only on the count of independent actors. The phase records and raw proofs
  remain private.
  The native active-context/preparation path now advances two retained object
  programs through eight preparations, including birth-frame channel advancement.
  Rust matches all 496 compared words; the context can retire while its object
  survives. The private harness now supplies sufficient stack for the real
  interpreter; no byte patch or invented first-frame delay is used. This bounded
  control does not establish whole-world allocation, capacity or scheduling.
  Source-declared completion replay is now integrated into Preview tools and the
  existing menu/loader/film lifecycle, with a distinct inspection launch intent.
  All 23 cohort sources have declared routes: four startup and 19 completion.
  Five completion choices across four cohort worlds preserve original mission
  indices; changed source bytes are refused. No guessed alias is added.
  Exact implementation source `4ef72a1` passes actual muted UI controls for
  Rstory5 mission 2 (19 directors) and mission 3 (33): both natural endings,
  then mission 2 re-entry and Escape after persistent birth. Tracked film assets,
  objects/contexts and session entities are released; the three preview profile
  files remain unchanged after initial unlock. Existing attachment omissions and
  a reserved Mecc audio-source refusal remain visible. The replay route does not
  reconstruct native live replay state, mark victory or admit Delphi gameplay.
  Combined source `c96c4a2` also passes actual muted UI double-click after birth
  and draw, returning to the menu with tracked cleanup and no reserved Mecc audio
  override. This verifies that gesture on Rstory5 mission 2 only; other scene and
  gesture combinations remain open. Both development lifecycle endpoints pass.
  Physical object-stock acquisition now executes the original initializer and
  constructs 3,000 simultaneous source-program objects with supplied resident
  resources/support. Three marked objects still occupy slots until the actual
  global cleanup pass; their freed slots are reused in observed order. All 3,003
  retirements finish with zero active objects and all 3,000 free slots restored.
  Three empty-free-head fixtures reach the original diagnostic entry; its fatal
  handler and recovery are not executed or certified. Failed early controls and
  an unproven generation-field interpretation remain private and rejected.
  A separate batch constructs all 158 Flick actors in the 23 bounded sources in
  an actual initialized stock: their construction leaves its counter, list and
  free head unchanged and never calls its object allocator. This observation
  excludes later VM callbacks, source FX and mission population. Do not equate
  Flick actor counts with ordinary-world occupancy or invent a per-film quota.
  Ordinary mission placement header follow-up: the shared owned declaration
  constructor now applies the acquired command3/4 object/status marks to its
  generic state0/state23 mission profiles after successful construction. World
  declarations and character-specific cold startup keep their existing routes;
  the already-acquired mission herd-marker path reuses the same header helper.
  A native 512-case control checks all 395,264 object bytes across the bounded
  template-copy/flag-store fragment, stopping before the initial object update.
  Rust agrees on all 512 header cases. The real source preparation path in four
  cohort worlds constructs and marks 32 mission actors and releases all tracked
  residents; its 40 existing declaration refusals remain explicit. Synthetic
  ownership controls reject failed/duplicate declarations without publishing or
  allocating another identity. This corrects owned header inputs, not physical
  stock accounting, complete mission admission or the omitted initial callbacks.
  The physical capacity/recycle boundary is acquired; real mission occupancy,
  shared admission, error handling and full world scheduling remain open. No
  shared Rust capacity policy or new gameplay admission follows from CPU proof.
  The 59-source gap total, 56 general-queue admission refusals and 282 strict-
  profile reviews remain: this is a partial runtime path, not 23 fully admitted
  or faithful scenes. Native in-mission producers remain required for gameplay triggers; the
  source-backed inspection route does not implement those callbacks. Live support generations, native world scheduling/admission and pool
  capacity, complete rendering/audio/subtitle parity and broader interactive coverage remain
  open. Next: finish these persistent contracts, then the mixed/other queue
  families and the overlapping partial FX/audio/credits/resume priorities below.

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
  Initial queue plan: a private program survey separates 22 root
  programs, including a bounded persistent-attachment family with retained native
  construction/startup controls. Existing visual preview is not sufficient to
  acknowledge live queue, owner, opaque constructor fields or cleanup contracts.
  The source-bound follow-up finds 28 persistent requests in 26 sources. Twenty-
  three sources across four worlds contain only that nonterminal family (25
  requests); three sources also need other families. All 28 use attached mode
  and pass the existing initial class-facing profile. Prioritize that coherent
  23-source cohort for shared queue/backend integration, then mixed families.
  These were implementation candidates, not newly admitted or visually verified
  sources. Reused native startup controls supplied placeholder owners; real owner
  updates, world admission, slot reuse and source-marked cleanup still require
  their own integration evidence. Prototype PS2 candidate names were checked
  against the pinned ELF before continuing; correspondence remains unknown.
- Owner's active work order (5 November 2026): address the shared 52-source FX
  prefix refusal first, then general queue admission and partial FX services,
  followed by remaining audio/credits/resume prerequisites. Work alone, prefer
  background checks; necessary test windows/interactions are authorized, with
  sound muted and no unrelated focus changes. Re-run the whole-corpus inventory and actual silent menu
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
