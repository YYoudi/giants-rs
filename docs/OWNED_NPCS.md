# Non-player actors and player interactions

Normal collection and the Starving twenty-meat return are connected to the common
partial Mecc controller in preview .12.1. Inspection/unsupported records retain
uncredited collection; general feeding/economy and full native admission remain
open. Authored herd admission follows source declarations, with Story2 and Story5
actual populations checked. See [MEAT_COLLECTION.md](MEAT_COLLECTION.md) and the
central [mission method](MISSION_PROGRESSION.md) for exact scope and PS2 evidence.

## Ripper integration in progress

The ordinary class230 mode67 primary now validates its current target, produces
real turning/forward commands and selects mode68 or69 using the actual class
parameters and shared RNG. It does not set actor positions. A strict melee-range
boundary consumes no draw. The required cue runs before selecting mode68; a cue
failure retains steering and the consumed draw, latching against replay. Six
private original whole-primary controls cover both selections, equal/outside
range, missing target and turning. Two synthetic controls check strict selection
and retained cue failure. Six further original secondary controls match the first
two channels with disabled blending, including the actual slots10/51 selected by
mode68/69. Those selections now use the existing ordinary secondary/ANM path;
mode69 and actual melee hurt remain required as described below.
These controls use supplied target/mode/seed inputs after native mode66 in a flat
world with an empty sound registry. They are neither natural encounter nor GPU
acceptance. `ripper_move`, `ripper_chase_location` and `ripper_animselect` remain
unknown prototype PS2 correspondence routes, with earlier PC aliases retained.

The ordinary mode68 primary now faces its current living target without forward
input or a destination write, waits for the actual primary-channel finished flag,
then returns to mode66. Only a target strictly inside its three-dimensional melee
range produces the required kind3 hurt request, using the real class-global damage.
This differs from the planar approach decision and from a kind0 projectile hit;
no projectile, hitpoint, health subtraction or hurt acknowledgment is fabricated.
Missing hurt ownership retains the preceding mode/steering and latches failure.
Six complete original primaries cover waiting, turning, lost target and non-hits
at equal/outside range or a vertical separation. A seventh original control stops
at the actual kind3 service entry without returning a result. Rust matches that
pre-hurt state and refuses the absent consumer; this is not a completed hurt call.
Two synthetic controls check the finished/range gates and retained failure.

The offline `mecc_enemy_hit` composition also admits an actual class73/230
state23 source with kind3 and no projectile owner or hitpoint. Projectile kind0/2
and direct kind3 identities cannot be exchanged. Eight private complete original
melee-dispatch controls match its nonlethal Mecc health, flags, directional impulse,
reset and RNG across callback-wide inhibition and health-only suppression. Their
world guards, source motion and dormant voice timers are supplied. The generic
dispatcher's post-hurt mission tail also runs on this path; projectile cleanup
must not substitute for it.

`Session::update_ripper_variant_melee_with_mecc` now connects the actual owned
class230 mode68 callback to canonical cold-Mecc health, velocity and voice-clock
publication. A separate melee attempt binds its target, controlled-motion step
and current primary epoch. Its controller response validates that active receiver
and epoch; a projectile receipt, stale epoch, delayed delivery or double copy is
refused. No projectile metadata, query point or projectile retirement is created.
Required world, controller, voice and generic post-hurt consumers run in order.
Late failures retain already published health/velocity/timers/RNG and latch the
primary against replay. Incomplete attempts retain their target reference; a
completed receipt preserves historical identity without adding a strong reference.
Scope cancellation cannot restore the removed source or accept its old response.

Five supplied-frame controls cover publication/order, health-only copies for
stationary sources, suppression/immunity, late dependency failures and scope
cancellation. Four private original class230
dispatcher observations also match this canonical composition. These tests supply
the owned frame/mode/finished-channel boundary and all service consumers. The
ordinary host still needs its real shared-world guards and controller/audio
providers; this API alone does not acknowledge their availability or a rendered
hit. The older kind0 projectile adapter keeps its distinct source validation.

The current isolated Wife approach consumes mode67 and its mode68 animation
selection at step3251, follows that attack through its real ANM clock, then
refuses the missing kind3 hurt consumer at3293 with the source already in mode66.
This diagnostic still fails; it is not a complete encounter pass. It supplies
ordinary player commands, excludes the Vimp component and uses no GPU or OS input.

The local GNU build for the preceding mode67/health-response milestone is source
`9a537e987598c7f07578cb30066bf6927ae85e3d`, executable SHA256
`f5b603687e3639fb22a8a45a8ed33673b77d0063b412867159f4f043a7a11f9a`,
Rust1.98.0/MinGW16.2.0. Its receipt is local; no download is published. Workspace
checks pass1,041 tests with212 ignored, plus strict Clippy and formatting. There
is no new graphical or Windows-input acceptance on this executable. Use the
ordinary menu procedure below for The Wife; no NPC activation flag is required.
The earlier clock lifecycle and failed combined encounter retain their own builds.

The newer local mode68/kind3-kernel GNU build is source
`525b0a47437a0d148fb71523c8077f4c96caffa4`, executable SHA256
`53b9b6a275a3db0dab5790b5eb1cf419949c8999b985c8c907bc59e0e1561ffd`,
Rust1.98.0/MinGW16.2.0. Workspace checks pass1,044 tests with214 ignored;
strict Clippy, formatting, the eight kind3 controls, earlier32 kernel controls
and48 canonical projectile-dispatch controls pass. The ordinary kind3 consumer
remains absent, and the supplied isolated encounter still fails at3293. No new
GPU, OS-input, reciprocal injury or Ripper-death acceptance is claimed on this
executable. It is a local handoff receipt, with no concurrent preview publication.

Earlier milestone evidence below includes the old mode67 frontier. The first
combined-game graphical blocker remains direct projectile damage to the player;
do not transfer the isolated Ripper-only approach or newer CPU selections to that
rendered scene. Reciprocal injury, Ripper death and complete Windows encounter
acceptance remain open. No new preview is published by this lane.

The ordinary component now handles an actual missed ground shot through the
retained query clock, named effect population, positional sound transport and
projectile retirement. A read-only proof checks the entire current owned radial
list: no eligible side-effect radius may be reached, including Vimp alert range.
It supplies no cross-context default, timer, fabricated health or empty hit sink.
A possible hit/alert still requires its real consumer. Canonical mixed bodies
use the shared RNG/clocks, current ordinary colliders and static declared platform
projection; changed collider input refuses. The common particle geometry pass
then owns post-preparation cleanup, independently of GPU/device availability.
The supplied Wife control observes the real step2755 impact, 26 immediate births
and 25 particle geometry frames, queued positional sound and actual shot release
on two fresh entries. Continuing the same approach reaches physical actor contact
at step3242. That earlier longer encounter refused the real class230 mode67
at step3251; reciprocal damage/death and rendered encounter/menu acceptance remain
open. Original native traversal controls cover 80 supplied contexts, including
inclusive hit/strict alert boundaries and genuine callback counterexamples.
These are runtime wiring and CPU observations, not a published preview or GPU pass.

The first graphical candidate at source `33b3168` rendered an explosion but
quarantined the Ripper component when simulation advanced before the next render
cleanup. It is a failed candidate despite a saved image and exit0. The ordinary
controller now consumes the same real particle geometry between simulation steps
using its actual camera, then runs scoped cleanup. The renderer can read the
completed buffers again without owning simulation or RNG. The two-ticket CPU
regression continues64 post-impact steps without a renderer update; it observes
27 cumulative births,24 final body submissions and repeated preparation without
RNG consumption. This repairs phase ownership; a new exact-build graphical run
must independently validate the result.

The corrected local GNU executable is source
`0714920d86eed8e7a2a3e5830be19b41a891ac42`, SHA256
`9632deaa1a0d450f1846a89dda8bdf8dfeecd1bd800127e85d562d95aaa25e7e`.
Its new supplied-input Vulkan run includes the normal Vimp population and actual
29-object projectile scene. The first shot launches at step4513, then reaches
an actual object contact with the controlled Mecc before any ground impact.
The missing direct-hit continuation refuses, and the diagnostic terminates
without a capture. This is a failed encounter, not acceptance of explosion
rendering on the corrected executable. The earlier saved explosion image belongs
to the failed `33b3168` candidate, not this receipt. The Ripper component CPU
fixture excludes the live Vimp component; do not transfer its specific miss or
physical-contact route to the combined game. Next connect projectile-to-Mecc
object damage/retirement and the selected close-range attack paths, preserving real world
and controller owners. No new prototype PS2 attribution, actual Windows input,
Ripper death, menu/reentry or public preview is established by these runs.

`mecc_enemy_hit::Machine` composes the independently acquired source gate,
debug-PC health, direct/radial velocity addition and pain/shared voice tail for
offline nonlethal class90/232 shots owned by a living state23 Ripper, and direct
class73/230 kind3 requests under the bounded source profile above. It requires
actual source/recipient motion, current radial center, world gates and equipment
state. A shared health-suppression gate does not suppress impulse and voices;
global callback inhibition skips the whole receiver. Required associated-player
and voice consumers run in native order. Failed consumers retain preceding state
and RNG effects and cannot replay. Shield, gyro, critical-voice and lethal
continuations remain explicit prerequisites. Canonical Session publication is
now provided by the adapter below; ordinary controller recoil/HUD and real world
gates still need their live consumers. This kernel alone does not acknowledge an
ordinary hit backend.

`Session::damage_mecc_from_enemy_object` binds that receiver to the real pending
kind0 contact, canonical accumulated damage/flags/shield/reset fields, retained
pain/shared timers and the current controlled-motion step. An original cold
constructor write trace independently establishes the cleared associated-player
and character-voice links; the adapter preserves only that acquired empty-link
profile, without inventing a player-record layout. State publishes before the
required recoil and local-cue consumers. Late failures retain health, velocity,
timers and RNG and latch the projectile receiver against replay. Source marking
and terminal processing remain in the enclosing direct wrapper. A callback-wide
inhibition still returns before the receiver's state/timer/impulse work.

The 48 private original dispatcher controls now also match this cold Session
adapter, including its actual canonical health and motion publication. Three
synthetic controls cover consumer ordering, successful voice timing and retained
gate/controller/voice failures. Their gates, controller sinks and cue consumers
are supplied fixtures. The ordinary host still needs source-backed shared-world
initialization/transitions and actual recoil/audio consumers; this API does not
acknowledge their presence. The voice-clock producer is now bound below.

The required controller consumer now receives an opaque `HitResponse` containing
both actual before/after health and velocity. A health-only change must be copied
even when impulse leaves velocity unchanged; damage is not subtracted a second
time. The response validates the actual source/recipient, active callback, current
pose/step and canonical after state. A changed local before state, replay or late
delivery after source retirement is refused. The partial controller has the copy
method, but its ordinary direct-hit host remains unbound pending real world gates
and cue transport. This is not an ordinary damage availability acknowledgment.
One additional private original-PC zero-direction control matches the complete
dispatcher response, including damage with unchanged velocity. Two synthetic
controls exercise health-only copy and rejection of a desynchronized controller.
These controls supply their world gates; they add no rendered encounter result
or confirmed prototype PS2 attribution.

The ordinary partial controller now advances its actual Mecc pain/shared voice
countdowns through `controlled_mecc_hit_clock::Clock`, before the admitted NPC
consumers. It uses the same live global clock sample, independently of NPC
availability. Multiple fixed physics steps and repeated preparation within one
sample do not subtract the frame delta twice. The acquired PC initialized shared
value is **ten seconds**, not zero. The application retains the actual post-cue
value before Session teardown and transfers it into the next fresh mission;
menu idle time does not advance it. This does not change the separate experimental
owned-primary clock policy or acknowledge complete native primary scheduling.

Thirty-six private controls execute the original countdown fragment and both
original global-clock calls, matching canonical Session field publication. Three
synthetic controls cover sample deduplication, foreign ownership, reentry, actual
post-cue reset and retained failures. An installed Wife control uses two fresh
Session identities under one application clock, 120 real sampled-clock/controller
updates, duplicate calls and actual scope teardown, with no RNG consumption. Its
first produced global delta is 25ms; it is not replaced with a zero delta.
These are CPU/runtime binding controls, not a new rendered encounter or OS-input
acceptance. Real world gates and recoil/audio consumers still block direct hits.

A separate visible Windows run validates this clock lifecycle on local GNU source
`42dcd04765fdca1ba496f095949c0d7d059d9376`, executable SHA256
`d14eea1cc791ead4b7508e2679a99e5eaf01b5a278d051ff3b4ab8e49d6b2666`.
Actual Windows clicks follow Preview tools -> Browse all missions -> Mecc Story2
-> The Wife. Escape skips the first opening and reaches rendered ordinary Mecc
gameplay; Escape returns to mission selection. Selecting The Wife again reaches
its opening and ordinary gameplay under a fresh ticket, then Escape returns again.
The two real Session tickets are1 and3. The second clock starts from the actual
first released value minus its actual first global delta, rather than resetting
to ten or clamping a negative timer. No clock/NPC component refusal occurs in
this idle lifecycle. The exact test window is then closed and its process is no
longer running; a numeric exit code was not retained and is not claimed.

Launch through the ordinary `--mecc-menu --data-root <installation>
--user-data-root <fresh profile>` path. This Windows run uses no replay/probe or
NPC activation flags; `GIANTS_AUDIO_MUTE=1` only silences the real audio services.
It exercises menu/entry/reentry with actual OS input, without gameplay pose,
health or readiness writes. It does not exercise held movement, jet, firing,
a complete Ripper encounter, reciprocal injury or Ripper death. The earlier
combined-game direct-contact failure remains the combat frontier; do not transfer
its supplied encounter or explosion observations to this newer clock executable.
This is a local build and bounded lifecycle result, not a published preview.

`Session::complete_enemy_projectile_object_contact` now owns the separate
positive class90/232 direct-contact continuation. It consumes the actual retained
actor identity, hitpoint and tag, reads damage from the source header, and calls
required world-gate, hurt and terminal consumers in native order. Secondary
immunity returns early; primary immunity skips hurt/mark but retains the terminal
tail. Callback-wide Mecc inhibition can skip the receiver while the wrapper still
marks the projectile. This branch neither creates the ground explosion nor
subtracts the ground lifetime tail. Its distinct object receipt admits
`cleanup_enemy_projectile_object_contact`; neither cleanup API accepts the other
reaction's receipt. Failures retain earlier service/RNG/mark effects and prohibit
replay. A completed receipt releases the pending target reference, preserving its
historical identity without making it a transform parent.

Forty-eight complete original-PC dispatcher controls match the composed Rust
continuation and nonlethal receiver across supplied failure/completion guards,
callback inhibition, health-only suppression and both immunity flags. They
compare damage, flags, velocity, source position/lifetime/mark, reset word, RNG
and reached callback/terminal counts. These supplied headers/world guards are
private controls, not producers for ordinary gameplay. The new continuation is
not yet bound into the ordinary direct-hit host: actual shared world-state
ownership, controller recoil consumption and real cue transport
remain the next requirements. The latest graphical failure and exact
executable above remain authoritative; no new encounter or death pass is claimed.

`Session::complete_enemy_projectile_ground_flight` continues an actual retained
terrain contact from either flight-query API. The query retains its own global
delta and ordered slow areas: callers cannot replace the contact point or clock,
rerun movement/history, or substitute object contact. Required effect, radial
and sound services precede the original lifetime tail. Partial failures retain
their side effects and forbid replay; successful marking still needs separate
resource retirement. The ordinary missed-shot host now binds those phases under
the no-effect proof above. Active damage and the other contact/expiry families
still require their acquired consumers; prepared context ownership alone is not
available damage or proof of rendered pixels.

The canonical mixed-program construction owner is now
`attached_effect_context::mixed::Execution`. It consumes actual source records,
creates root and immediate-child Session identities in source order, retires
each completed child context and retains the root context across its declared
wait. Zero/half-step suspension and exact-deadline continuation preserve the
original RNG stream, construct the delayed body and actually retire the context.
A second wrapper cannot consume an already-active context; failures retain the
constructed prefix and prevent replay. Completion means source construction,
not body lifetime, render readiness or damage completion.

`mixed::population::Population` now owns the following host phases over those
canonical contexts and bodies. Named impact preparation retains a real context;
activation consumes the caller's global clock, and each body independently
selects object-local time from current fields/slow areas. Body callbacks and
cleanup follow the actual Session common-list order within this owned cohort.
The caller consumes prepared geometry before invoking cleanup. Duplicate frames,
foreign pools/tickets and missing cleanup are refused before further work; late
world failure retains the executed prefix and prohibits replay. No random stream,
world service or render/audio acknowledgment is manufactured. The ordinary
missed-shot host now binds that owner to its impact and surrounding frame phases;
this is not a complete native world/interleaving or combat delivery.

An installed/native control compares all 44 medium/large births, including the
two delayed bodies, on 2,596 selected words and both phase RNG results. A second
installed control runs four ordinary history/callback/finalization/placement
passes over the first-phase bodies against The Wife's actual declared terrain
and collider preparation. Its 168 preparations use the common world services,
without a cinematic scope. It explicitly selects the existing prototype PS2
zero-threshold progress policy; no debug-PC caller bits are invented. This is a
source-world service control, not a native whole-world schedule or GPU check.
The missed-shot host above binds the ordinary effect/world/audio transport phases.
Reciprocal hit owners remain necessary for complete encounter acceptance.

The common mission unlit consumer now accepts these canonical mixed bodies only
after completed ordinary preparation and one-shot placement. Its current vertex
references receive the same Session UV offsets and vertical-fade values; Bevy
updates UV/color buffers and the existing vertex-alpha material branch. The
renderer owns no simulation clock or RNG. Hidden carriers remain absent from
draws. The installed source-world control produces 160 actual render-input frames,
including initially transparent and subsequently changing gradient/UV data.
It retains 9 declared colliders and 23 unresolved platform candidates: a needed
unprepared platform still fails rather than becoming a default surface. This
validates CPU input wiring, not actual rendered pixels, native queue order,
complete world scheduling, body cleanup or ordinary reciprocal combat.

The longer lifetime control exposed the vertical material's distinct descriptor
path when object fade starts. It now uses the acquired deferred vertex-alpha
override: RGB addition, diffuse alpha times the material/object factor and
source-alpha/one blending with depth writes disabled. This is checked against
the exact debug renderer module, separately from the executable; no prototype
shader-name match or new attribution is established. Texture-alpha gradients and
other gradient RGB programs still require their own evidence.

The longer control now uses `declared_platform_scene::Prepared` with the actual
source selector, terrain-plane gate and ordered transformed mesh. A distinct
`PlatformBits` projection represents only raster platform membership; native
terrain/neighbor writes cannot change that bit. It supplies neither full
footprints nor neighbor/list ownership. Existing-footprint and building-query
requirements remain unresolved. Matching candidate fields are required when
binding this query owner to the world; the default unresolved path stays strict.
The exact debug PC selector matches 1,024 controls and platform bits match 420
native controls across terrain and neighbor inputs. This source-world composition
then runs all 44 mixed bodies through 9,853 ordinary preparations, current geometry
inputs, real expiry and scoped cleanup, releasing payloads/specification/state
references/registrations and refusing stale owners. Native expiry/destructor
controls remain separately scoped to direct callbacks and the global cleanup
walk. This does not establish complete native world scheduling, GPU rendering,
ordinary impact delivery or NPC death. Declared rigid projections require a new
snapshot/invalidation contract before use with moving or changed world owners.

The current immediate-particle dependency is CPU-validated, not an ordinary
explosion delivery. The canonical timed owner now constructs the acquired
source-declared particles, retaining their actual temporary parent/reference
bindings, forward launch, gravity, contact parameters and source-order RNG.
The constructor also preserves the PC one-shot placement mark; a second callback
before its real placement is refused. Placement uses current canonical parent
and child references through the shared relative-scale attachment math, consumes
that binding once, and leaves launch pending for the next callback. Parent
reference selectors vary with the declarations; they are not a fixed reference1.

A private installed/native control compares32 children from both explosion
programs at birth, first callback and actual one-shot placement:5,664 selected
words, plus launch speed, reference selectors and constructor/final-stream RNG.
All agree. Source resources, original memory/pools, clocks, quality and math mode
are supplied boundaries. The canonical ordinary-preparation tail now calls this
same placement helper, but the control exercises direct callbacks/helper calls;
it does not validate the complete native world walk or preparation schedule.
The missing-provider control deliberately fails the next physical callback and
retains failure. A further supplied-static-world control now exercises three
full original physical callbacks for each child against the actual Story2
terrain. All96 callbacks match6,720 selected words, including current/previous
velocity, contact plane, floor and inline forces. Thirty-two callbacks reach
ground contact; the remaining64 are airborne. This exposed and corrected the
timed adapter's friction/damping field selection; the shared gravity kernel is
unchanged. The post-gravity contact tail uses the existing velocity-target force
kernel with actual payload parameters and contact flags, after completed motion,
ground query and gravity, once per callback. It supplies no synthetic contact.
The common static-world adapter now binds that required tail. This control uses
supplied clocks, static/no-ripple terrain preparation and direct callbacks; it
does not certify the complete world/preparation schedule or object population.
Live mixed-program scheduling, graphics, damage and retirement still block impact.
No new PS2 attribution, executable or rendered encounter is established here.

Enemy projectile impact preparation now has a typed mode8 entry. The PC request
mode selects positional input; it is not context flag8 and supplies no inherited
flags in this profile. An original mode8 control, with an explicit supplied
camera/quality boundary, reproduces the same source construction and96 physical
callbacks. Canonical preparation still requires normal/lite selection to resolve
to the same actual entry; it invents no camera/load state. This prepares only the
context. It does not acknowledge mixed-program execution, render or damage.

The combined main/NPC local build `0fd1cccbe5e736e57c408e87d9cf822a5fc67f14`,
executable SHA256
`121abb1e325a21b7ce697bce359110df6b24c6338de6cd929ffba32d6f8d33f3`,
passes the supplied-input first-flight GPU check after the main mission/FX merge.
It exits0 after4,521 steps, with1,681.1061 units of normal Mecc movement; the real
class232 shot starts at4489 and reaches captured draw revision32 at4520.
The reviewed image shows Mecc facing the Ripper and its projectile, with no
audio failure or component quarantine in this bounded run. Its private capture
SHA256 is `d70ce5022e7a2c19565fd78a74ca1fe189c1988acdef5c4fef4e10d61bdc29c7`.
The preceding combined build `e764b7c` failed with audio backlog and no capture.
Only the diagnostic pilot's supplied RMB release/refuel and hover thresholds
changed; the ordinary jet, audio budget, NPC state and readiness rules did not.
This is muted Vulkan with supplied commands, not Windows input, impact/damage,
death or menu/re-entry acceptance. It does not establish full combat availability
or a published preview. The earlier evidence below retains its original scope.

An earlier supplied-input GPU attempt (`a450206`, local executable SHA256 prefix
`19f651e9a211`) failed without a capture. A mixed positional-audio backlog closed
the backend before the NPC impact adapters refused the missing audio ticket.
The latter error is a consequence, not permission to acknowledge an empty sink.
The audio queue now batches consecutive already-prepared one-shot cues, retaining
each cue, position, RNG order and real-sink start requirement. Cold/looping cues,
actor/control commands and ticket boundaries still stop batching. The 256-request
failure cap remains.

The fresh GNU build `a6c99c71a447bd385a41e3d2dc4828f3fd8c5a1f`, executable SHA256
`004165cacc0f1abbf612891399b293e70b5cbc3cc6e4de08254a6e28b86adb0d`,
finishes the same guarded GPU approach with exit0. It submits the actual five
Rippers and fourteen Vimps, observes a canonical class232 projectile at step4343,
then captures its draw revision32 at step4374. Supplied normal Mecc commands move
the player 1,518.7316 units over 4,375 steps. The reviewed 1920x1080 image shows
Mecc, a Ripper and its small blue/purple projectile ahead in the ordinary terrain.
No audio failure or NPC quarantine is logged in this bounded run. The retained
private image SHA256 is
`361bdbfb5e3c772ea5b17b5efa46cc2e1c5f20d4b8714c0a009c1f7b3d8023e3`.
Original assets and evidence remain private. This is supplied-input, muted Vulkan
render evidence before impact, with no Windows input, inflicted damage, Ripper
death or menu/re-entry acceptance. It is not a published preview. Genuine radial
active-hit health/feedback and remaining contact consumers were the next gaps.
The missed-ground host above supersedes that ground-impact refusal; it does not
substitute an empty sink for genuine damage.

The next radial bridge now reads the actual first additional Session list,
retaining its registration node across callbacks and reading the next edge
afterward. It does not iterate common list0 or take a frozen actor census. Its
live actor view uses current header positions/flags/context and acquired health
owners for Mecc, Rippers and Vimps. Missing health stays absent and is refused if
the eligibility path needs it; it never becomes an invented zero or dead actor.
An actual state6 impact request must match the owned payload parameters, position,
phase and launch context. Real Vimp signal stores precede the mandatory hit
consumer. A late hit error retains those stores and the single-use failure latch.

The installed Vimp/list/signal control passes with an explicitly failing captured
hit consumer; it proves that failure is not acknowledged as damage. The Wife
two-ticket first-flight control reads actual Mecc/Ripper health and inventories
the dedicated list. Its initial inventory exposed additional authored state0
recipients without a generic header-health owner. A separate exact-PC control
executes real generic pool allocation/zeroing, class/state copies and insertion:
18 cases retain zero accumulated damage despite poisoned free storage and varied
template damage/limits. The state0 class selection and limits are supplied;
outer allocation/archive choice/math mode remain control boundaries. This is
constructor evidence, not a damage callback or native mission run. Ordinary
state0 construction now retains that independent header once before publication;
reads use current damage and class limit and never supply zero for missing data.
Other unowned state families, the actual cross-context global producer and
attributed Mecc hit/feedback services still require connection before ordinary
explosions. No new graphical or complete encounter acceptance is claimed for
this intermediate bridge.

The marker's generic header health is now owned too: its actual constructor and
69 retained PC frames keep zero accumulated damage. The private historical
`state0_damage` field is now the shared passive header owner, not a value created
when health is queried. A living, unmarked state0 recipient has no hurt callback;
class219/state48 uses an empty shared PC callback. In this explicit single-player
profile, the complete wrapper's kill/counter tail is also inactive. The bounded
no-effect result is invariant under either wrapper guard. Thirty-two original
wrapper controls cover both recipient states, positive/negative amounts,
source presence and both guards, preserving every recipient header byte; the
empty callback and real tail are observed executing. The marker header is from
actual construction; source/impact inputs and the SP mode are supplied.

`Session::enemy_radial_passive_no_effect_singleplayer` validates the actual
canonical impact source and this living passive profile. It neither assigns an
unknown global nor supplies an empty hit backend for active/dead recipients.
Other recipients return a requirement for their real consumer; absent/foreign
sources and missing health remain errors. This does not complete ordinary damage.
The next impact dependency also includes the real mixed explosion FX program:
six source root bodies, nested debris and still-unbound operations must use the
common canonical effect families. An attached preparation or one supported
sprite cannot acknowledge that whole program. Main's effect and mission services
are now preserved in this lane's baseline; the published preview receipt remains
the main lane's unchanged artifact.

The construction ledger now retains the source-ordered UV-scroll and vertical-
fade setup as pending renderer commands, without consuming random samples or
collapsing repeats. This lets both actual explosion programs be inspected through
their delayed second phases and nested debris. All referenced class templates
load through the canonical effect-template validator. No constructor or callback
is acknowledged by this inspection: the visual-preview, surface and ground
consumers explicitly refuse these unbound renderer services. Ordinary impact
admission still requires the live canonical program, body, render and retirement
owners; this ledger is not their replacement.

One supplied-seed PC control per explosion now executes original named
preparation and the first activation through its actual nested child constructors,
stopping at the source wait. The medium and large programs construct16 and26
bodies respectively. Their ordered classes, final RNG and126 source-configured
size/duration words agree with the Rust ledger. Source indexes, outer memory,
textures, free pools, positional origin, quality, TLS/seed17 and math mode are
supplied. Earlier harness stack/budget failures remain retained privately.
This bounded constructor comparison does not cover service failure prefixes,
callbacks, the delayed phase, canonical Session adoption, vertex rendering,
contacts, damage or an ordinary encounter. It confirms no new PS2 attribution.

The source3 scroll/fade frame now has a bounded timed constructor that validates
its actual record offset, complete source image/program, class and positional
context. It does not consume the mixed context or create its siblings. The
ordinary canonical timed owner can retain its UV/fade parameters; Session
accessors read that same owner. UV advances in the actual payload and the
single fade segment uses typed dynamic ownership, without a fabricated native
pointer. Updates run before ANM on visible callbacks, skipping the first,
inactive and expired cases. Equality with the fade duration remains active,
including a zero delta, until a later positive delta passes it. The existing
single-segment kernel previously retired at equality and is corrected.
These current parameters do not acknowledge vertex-buffer use or make the whole
explosion available. General ledger preview/surface/ground paths keep refusing
their unbound renderer needs; multiple fade segments remain outside this owner.

The original direct callback comparison covers176 frames across both source
explosions, steady deltas and exact-duration/zero-delta boundaries. Twelve selected
current words per frame agree with the canonical Session body: control flags,
age, UV, fade time/values/active state, scale, transparency and response/render
flags. Four real retirement requests deliberately reach a failing required
consumer; the canonical owners retain failure, remain unmarked and refuse replay.
Complete null-object preparation now admits these two source frames and preserves
their observed RNG result without preparing immediate children early. These are
supplied-resource/clock CPU controls, not the world schedule, vertex consumer,
actual deletion, reciprocal combat or graphical acceptance of an explosion.

`Timed::mixed_root_object` now constructs each acquired positional root prefix
from its actual source offset and whole-program identity. It supports the fixed
height/size root, ranged-size gravity roots, the scroll/fade body and the hidden
carrier prefix. Hidden ownership is explicit: the source render mark is retained
through the first callback, rather than making ordinary visible bodies accept
that mark. It creates neither the carrier's required immediate children nor a
wait acknowledgement, and does not consume the shared context.

Twelve root snapshots across both explosions, including the actual delayed
class814, match708 selected PC words: flags, pose/history, affine, size/lifetime,
fade configuration and physical storage. The per-root RNG inputs are observed
at native constructor boundaries after preceding children; those children are
not emulated by this fixture. All42 initial native constructors preserve their
incoming RNG; the configured ranged-size samples produce the observed suffixes.
The original context wrapper also confirms suspension on zero/half steps and
resumption at the exact wait deadline, creating class814 and returning its real
context to the free list. A previous harness incorrectly assumed a strict
deadline and reused an already retired context; that failed attempt is retained.
The existing construction cursor's deadline rule already agrees and is unchanged.
Their twelve first direct callbacks also match another708 selected words,
including the hidden carrier's retained mark and the visible roots' ordinary
first-frame transition. Physical, gravity and retirement services deliberately
refuse calls in that first-frame fixture; none is acknowledged as available.
This is root construction/context evidence, not whole canonical execution,
ordinary physical callbacks, child reference placement, rendering or damage.

The NPC lane stages a separate declaration-bound Ripper component in ordinary
mission loading. The Wife has five canonical mission declarations: four class73
and one class230, all state23. The component retains their exact source placement
and Session identity; it does not create a replacement population. Binding
refuses missing, duplicate, substituted, foreign-ticket or retired owners.
Other maps are selected by their declarations rather than a world-name switch.
An installed census control binds each of the first five Story2 scenarios;
their class73/class230 counts are respectively 7/4, 4/1, 7/1, 2/0 and 2/1.
These are authored declaration counts, not active or rendered populations.

Normal loading now also prepares each bound Ripper's actual affine, source
geometry/terrain environment and cold finalization/motion ownership against the
selected mission world. This starts no history, callback or animation clock.
Initialization records failures and refuses a whole-component retry after a
partial attempt. The Wife installed control preserves all five authored poses
through this preparation and verifies idle owners over two fresh tickets.

This binding alone admits no callbacks, completed pose, contact or gameplay.
The first installed control covers constructor identities and two-ticket
retirement/re-entry, without rendering or Windows input. The class230 owned
primary now also has its mode62/63/64 continuations: current-channel completion
drives the transition to63, whose actual class radii select the next mode.
Target loss retains the required cue and failure latch. These use their own
steering precision without class73 speed restoration or dirt retirement.
A read-only exact-PC control agrees on 120 consecutive mode62/63 callback outputs
(2,040 words before common motion), including the selection of mode65. Terrain,
peer eligibility/placement, clocks and absent audio registry are supplied in
that diagnostic; it is not an ordinary encounter.

A second control supplies a peer position change after actual target acquisition
and compares 123 selected mode62/63/64 outputs (2,091 words, including three
mode64 callbacks). Direct pursuit retains the source destination and personality,
uses the acquired angle helper's narrowed return, and enters65 at the actual
fire radius. This is a diagnostic peer input, not forced Ripper motion or a
substitute for ordinary player interaction.

The class230 mode65 primary now selects the actual firing clock crossing,
target-centre height and class-specific launch/decision parameters. Its owned
Session caller reads target dimensions from current identities and uses the
shared state6 projectile factory, effect preparation, positional audio request
and current animated muzzle reference. The exact-PC sequence agrees on 180
selected non-firing callbacks / 3,420 words (including sixty mode65 callbacks).
One actual native firing callback is excluded from that numerical comparison;
the installed launch control separately checks real class232/state6 construction,
prepared named FX, reference-derived launch, source ownership and retained late
sound failure without another creation or RNG replay. Its mode65/channel and
flat world are explicit diagnostic inputs and audio is suppressed. Prepared FX
does not establish visible projectile effects or impact damage. Mode66/71 and ordinary firing/flight are now connected below; later attack
continuations, impact reactions and reciprocal combat remain prerequisites.

Actual declared Rippers retain recipient flag400. Exact-PC attached preparation
maps that flag into context flag10000 while preserving the extended caller's
existing flag20000, recipient and tag. The shared context pool now retains this
metadata instead of refusing these declarations. Native controls of the same
dirt request with and without flag400 produce context flags30004 and20004;
the inherited bit introduces no extra random draw. This prepares a context only:
effect activation, emitted objects, particles and their renderer remain separate
services. The prototype PS2 `fx_create` correspondence is unknown.

The next delivery is the normal menu-to-The Wife encounter, reusing the shared
Mecc/Session clock, RNG, terrain, render, audio and retirement contracts. Missing
class230 continuations, contacts, reciprocal
combat and source-specific death services remain prerequisites. No Vimp ghost
or meat behavior is copied. Main retains mission progression and publication.
The prototype PS2 `ripper_move` attribution is unknown; see
[symbol navigation](SYMBOL_NAVIGATION.md) and the central reconstruction method.

### Ordinary Ripper frame and pose consumer

The NPC branch now connects a bounded Ripper component to the ordinary partial
Mecc controller, independently of herd availability. It uses canonical declared
identities, current published Mecc motion and source camera, Session health,
shared mission clock/slow areas and the existing mission RNG. Cohort position
histories precede callbacks in current common-list order. Primary commands feed
ordinary motion, secondary selection, ANM references/events, terrain/object
finalization and current environment geometry. No supplied velocity or behavior
mode replaces the primary. Its animation stamp is component-scoped; complete
native world ordering and random-stream interleaving remain unverified.

The common NPC renderer copies model-zero vertices only after successful world
iteration and finalization, retaining the actual animation epoch and affine.
The RGB body route now accepts class73/230 after resource validation. Cold or
failed frames supply no draw. Late ANM failures retain consumed clocks/events
and cannot replay or finalize as successful. A Ripper component/draw failure is
reported and quarantined independently of Vimps. Prior audio transport requests
survive a late callback refusal; queue submission does not establish a voice.

The admitted ordinary primary subset is class73 modes0/31/32/33/34/44 and
37/38/39, plus class230 modes0/61/62/63/64/65/66/67/68/71. Mode66 consumes actual object
local time and its source retreat decision. Mode71 follows the current animation
and marks only the actual class219 child when required; its common child frame
and retirement batch still own release and unlinking. The acquired secondary
selector also admits mode68/69 selected by the acquired mode67 primary; the
mode68 kind3 hurt consumer and mode69 primary continuation remain required.

The persistent projectile factory retains actual mission resources/templates
across births. Ordinary launch uses real attached-FX preparation, positional
sound submission and the current animated muzzle for class90/232. A shot receives
real initial radius/terrain preparation and advances against the current scene.
Its animation/draw, actual contact reactions and damage remain required. Contacts
and expiry stay pending instead of receiving a default successful reaction.
Earlier creations, preparations and sound requests remain consumed on refusal;
component quarantine prevents replay. Other primary modes still refuse.
Perception currently admits the actual controlled Mecc subset; other families
are not represented by invented current providers. Dirt/event contexts can be
prepared but have no active effect/particle scheduling here. Reciprocal combat,
source death and cleanup remain required before claiming a playable encounter.

An installed-data control runs 180 supplied stationary-player frames on each
of two fresh The Wife tickets. All five actors complete their frames and supply
five nonempty class73/230 poses to the common CPU draw consumer. Repeated clock
samples consume no additional RNG and repeated ANM consumption is refused.
Source terrain and the controller's actual rigid colliders are used. This is a
Rust composition control, without original execution, GPU or Windows input.
Thirty further frames exercise the common headless Bevy asset consumer, changed
animated vertices, buffer reuse, camera hiding/restoration and cleanup. The
acquired untextured diffuse parts use the same unit texture/source-RGB path as
cinematic bodies; a missing named image is still refused. The first Windows
attempt on source `5fefde0` exposed this missing null-texture binding and did not
establish Ripper rendering or encounter acceptance.
The prototype PS2 `ripper_animselect` candidate for the acquired PC secondary
callback remains **unknown**, separately from this implementation result.

The ordinary controller now also merges completed Ripper contact submissions
with Vimps in actual Session common-list order. The shared `actor_contacts`
packet retains `vimp_contacts::Snapshot` as an alias. Dedicated collision-model
selection, current ANM vertex buffer, affine and acquired animation bounds feed
the existing player ellipsoid and projectile segment consumers; render LOD or
constructor meshes are not substitutes. Cold, foreign, zero-number, unfinished
or failed frames refuse. Markers never enter this body-contact population.
Stable scene keys retain separate typed actor identity and source class, and
family quarantine clears that family's stale shapes while retaining the other.
Ripper admission follows actual declarations without a map-name switch.

The exact PC common collision-event dispatcher has absent entries for states2,
21 and23. Three native calls independently return zero; this supports the
unchanged common event response for these admitted families, not other NPCs or
whole-world parity. The installed four The Wife controls cover both classes,
source-face segment intersections, shared scene registration, incomplete-history
refusal and two-ticket lifecycle. The existing Vimp contact/re-entry regression
passes after extracting the shared collision geometry consumer.

The named ignored diagnostic `diagnostic_wife_ripper_supplied_approach_reaches_contact`
drives the actual Mecc controller/terrain/collision path toward a current Ripper.
It supplies player commands only, without assigning an NPC pose, mode, target,
health or readiness. The first run advanced the player approximately 1,502 units
before class230 selected mode65 and the old ordinary dispatcher refused it.
With the launch consumer, the same approach creates class232 at step2618 after
approximately1,517 units. The current-scene flight consumer now advances the
shot with actual history, rates and lifetime. With mode66 and mode71 connected,
the approach reaches step2755 after approximately1,593 units of player movement.
After137 flying samples, its next query returns a real ground contact. The
then-missing impact/damage/retirement continuation refused and remained pending.
That historical frontier is now superseded by the ordinary missed-shot host and
physical-contact control above. The longer encounter's current frontier is
the class230 mode68 kind3 hurt request after mode67, rather than this earlier terrain contact; injury, death and GPU fidelity
remain unverified.

The ordinary controller now publishes a Mecc dedicated-contact packet through
`Session::controlled_mecc_contact_snapshot` before the Ripper component, using
its actual selected clip/tree and clock. The Session
validates canonical class2 ownership and the contiguous current physics step,
checks the prepared descriptor, runs its real affine writer, deforms collision
model6 (`mc_l6`) through its own binding and computes source animation bounds.
It does not change native ANM channels, health or environment readiness. Resource
ownership, foreign/stale steps, mismatched descriptors/trees and nonfinite time
refuse; failed publication clears the previous packet. Installed controls also
cross a current dedicated-model face with the shared segment consumer. The
flight consumer requires this actual packet rather than a render model0, box,
invented global vertex epoch or ready result. The ordinary scene now combines
actual prepared rigid declaration geometry, the current Mecc packet and current
completed Vimp/Ripper packets. Static source keys remain distinct from Session
actor identities. Query keys retain the controller registry; admitted NPCs retain
the common list order within the partial cohort. This is not complete native
population, interleaving, live static-object simulation or damage availability.

`Session::advance_enemy_projectile_singleplayer_scene` shares state6 arithmetic,
terrain/object queries, history and pending-contact publication with the existing
owned-list route. It validates actual resident flags/parents before publishing
motion. Declaration hits retain a typed declaration target; neither a fabricated
actor serial nor a global vertex epoch is required. Ground/object contacts and
expiry remain pending for their required reactions. There is no placeholder
impact provider, acknowledged clear world, damage or retirement success. The
legacy owned-list API retains its immediate real ground-service/countdown order.
A failed peer geometry producer is retained as an error until an actual shot
needs it; it cannot become an empty scene or suppress unrelated Ripper frames.

The next impact dependency is now independently acquired as `radial_damage`.
It retains the actual ordered list traversal, context gate, four special-state
predicates, active/health filters, stored 3D distance and radius arithmetic,
strict Vimp-alert range and kind2 hit requests with the original point/tag. The
eligible `other` identity ends the entire traversal, including outside hit range;
an ineligible `other` does not. A zero slope returns before any world access.
Callbacks read subsequent actors live, and callback failure retains its prefix
and forbids replay. No default list, health, alert or hit provider is supplied.

Private exact-PC traversal controls cover1,692 cases and689 outgoing requests
with no differences, including radius boundaries, special states, contexts,
health, list early return, 3D separation and zero slope. The four predicates
were separately executed for all state values0..255. Actor headers, list/world
inputs and outgoing capture-return sinks are explicit fixtures. These results
validate requests and arithmetic, not applied damage or ordinary world coverage.
Literal prototype PS2 search routes `hurt_object_do_multi` and `hurt_object`
remain unknown correspondences; raw symbols and old PC aliases stay in private
identity-bound records. No platform ABI/layout is transferred.

`Session::record_vimp_radial_signal` supplies the separately acquired recipient
writer: it copies a canonical source actor's actual current origin and the
already-produced world elapsed time into the Vimp signal payload. The native
writer changes only four words across64 controlled cases; an installed control
checks real constructors, zero time, other-state preservation and foreign/retired
source refusal. A zero clock stays zero; no invented timeout or forced movement
substitutes for the later living response. The writer changes no mode, channels,
health, RNG or readiness. Its prototype relationship is still unassigned.

The current normal Mecc contact packet additionally carries the actual radial
impulse center: the selected ANM auxiliary box's midpoint transformed by the
current actor affine. A missing auxiliary descriptor uses the actor origin,
not a primary/render-mesh box or the center of a world AABB. Original PC impulse
and transform executions match512 supplied box/cache/impact cases and3,072 center/
velocity words. These are geometry/arithmetic controls, not applied health.

The PC Mecc callback entry is now established by an actual indirect dispatch
trace. `mecc_hit_source::singleplayer` reproduces its nonnegative-hit source gate,
including one owner dereference, state2/44 context filtering and exact self-source
exceptions. Private original-callback controls match1,536 cases /3,072 words under
explicit inactive gates/equipment/voice fixtures. A denied health change still
requires the callback's remaining flags, impulse and feedback; never drop a hit
because this arithmetic gate is false. Multiplayer policy is not admitted here.
`Session::enemy_projectile_mecc_source` validates an actual class90/232 shot and
its current canonical Ripper owner against a real Mecc recipient. The two-ticket
Wife flight control checks that relation and rejects foreign or non-Mecc IDs.
It does not apply damage or publish associated-player retaliation. The literal
prototype key `mecc_shot` remains an unknown correspondence; old PC aliases and
raw-symbol uncertainty are retained privately.

These dependencies do not yet provide genuine active hits. The ordinary missed
shot now composes named FX, actual radial no-effect proof, sound transport and
retirement; a possible alert or hit still requires canonical list/health and
attributed Mecc/Vimp/Ripper consumers.
The existing source-absent/nonlethal Mecc service cannot stand in for an attributed,
directional kind2 blast, and partial-pistol serials cannot become canonical native
projectiles merely to attach a damage source. GPU reciprocal combat/death remain
required. The main lane's mission objective and publication flow are unchanged.

After a successful flight, `Session::advance_enemy_projectile_presentation`
consumes its actual object-local clock and launch-owned rates, advances source
ANM channels with the composing component's persistent stamp and runs the shared
current-affine writer. A per-flight revision rejects duplicate or skipped passes;
a late cache failure retains the clock prefix and latches replay. Draws validate
that revision and affine, then copy source-skinned vertices immediately. They
use the common transparent/additive auxiliary material program, not an opaque
body substitute. The actual class90/232 specifications have no animation events
or references; changed specifications requiring those services are refused.
This scoped contract does not claim generic actor-world finalization, effects,
impact readiness, damage, retirement or complete native scheduling.

An independent exact-PC control matches the shared clock/affine kernels on99
non-retiring source-created projectile frames:3,564 channel words and1,683 affine
words with no difference. Its native flat-world and peer services are supplied;
it is not graphical or ordinary encounter acceptance. Prototype PS2 animation
and location search records are retained with their separate attribution scope;
no new function relationship or platform signature is confirmed.

The installed `source_wife_ripper_launch_flies_against_current_scene` control
uses the same ordinary supplied-input approach on two fresh tickets, stopping
after a real class232 flight sample changes position/history and decreases
lifetime. It also checks source ANM/affine draw submission, duplicate-clock
refusal, actual mesh movement on the next runtime step and renderer asset cleanup. Its fixture admits four actual rigid declarations, the controlled Mecc
and five Rippers; it does not stage the Vimp component. This CPU control neither
proves a graphical shot nor expands admission to the whole native world.
The separate full-contact diagnostic continues beyond that first sample and
now consumes the actual missed ground impact and reaches physical actor contact.
That earlier longer encounter stopped at class230 mode67. Neither control establishes
reciprocal damage or source death.

Class230 mode66 preserves strict expiry before the rounded timer store,
current-target range boundaries, the cue-before-retreat-delay order and the
shared game RNG. A private exact-PC control matches485 selected callback
snapshots (9,215 state words plus24 sound-request words), including no-target,
zero-clock and boundary fixtures. Mode71 matches86 snapshots (1,634 state words,
86 marker-mode words and8 sound-request words): the strict animation threshold,
absent/different-class marker, actual marker mark and animation completion.
These controls use supplied peer/clock/flat-world inputs and captured sound,
not a live native world or audio-fidelity proof. No prototype PS2 attribution
is promoted by these PC comparisons.

The installed `source_variant_retreat_marks_owned_child_without_acknowledging_cleanup`
control uses actual class230/219 constructors with explicit mode71/clock inputs.
It checks the real reciprocal relation, marking without channel/retirement
advancement, non-marker/absent relations, late sound failure and no replay.
It does not establish natural marker retirement during an ordinary encounter.
Current source passes1000 Rust tests /180 ignored, five installed `source_wife_`
controls, the additional owned-retreat and radial-signal controls,45 Python tests and strict
Clippy/format checks. The full-contact diagnostic's pending-ground failure remains
separate; no rendered combat availability follows from these counts.
Ripper projectile models (`ripspit` / `ripspitbig`) also exist; preparation alone
does not establish their draw, movement or impacts. Source naming remains separate:
no new prototype PS2 attribution is confirmed by this input diagnostic.

A Ripper selected by the ordinary partial pistol is identified and reports the
missing damage consumer without dispatching to the Vimp backend or inventing a
health change. Physical geometry and target selection do not deliver injury,
knockback, blood, death or kill credit. Those consumers, GPU projectile acceptance, remaining attack modes
and a rendered reciprocal encounter remain required; `ripper_shot` remains an
unknown prototype PS2 search candidate for the separately acquired damage work.

The surfacing consumer now uses the real popup preparation, owned motion reset
and persistent class219/state48 factory, including the class230 marker birth.
The factory preserves one immutable specification/event identity across births;
the child event namespace must match its actual context pool. Its new owned
world consumer retains copied newborn history, then separate subsequent history,
primary, secondary, ANM references/events, finalization and current affine. Late
failures prohibit replay or interleaving isolated marker updates. The common
renderer consumes completed marker poses through the acquired RGB body route.
The component runs its Ripper cohort then its owned marker cohort; complete
native world interleaving and newborn admission remain unverified.

A private exact-PC control executes 69 actual marker primary/secondary/full ANM
and finalization iterations through retirement marking. Rust agrees on 3,519
selected mode/timer/flag/channel and position/history words. The parent birth
pose, flat world, clock, enabled blending and one parent motion command are
supplied. This is not a whole-header, reference-matrix, renderer or mission
comparison. Installed controls additionally cover disabled/enabled blending,
current animated geometry, late-finalization failure and replay refusal.
An explicit two-child constructor fixture in The Wife checks shared event/model
ownership and seven common headless Bevy visual owners across two tickets.
It does not prove natural awakening, OS interaction or a complete encounter.
The ordinary partial scene retains its disabled-blending policy.

Completed, marked state48 children now have bounded individual retirement.
The acquired PC reference pass clears the owner's callback link before object
data/registration release. The owned consumer first validates the whole selected
cohort and clears all those references in actual common-list order, then invokes
the real recipient-context/audio release services, revalidates and removes each
child's three registrations, state/specification and resident. It also clears
the Rust primary machine's alias; the native retired pool may retain old bytes.
No source actor death is implied. An external-release failure retains consumed
reference changes, quarantines surviving owners and prohibits replay. The common
renderer removes retired child assets while preserving the parents and unrelated
entities; shared resources remain ticket-owned. Only the acquired eventless,
payloadless state48 child and its exact reciprocal relation are admitted, with
no global death cascade or copied Vimp ghost/meat behavior.

The installed ordinary-component control uses two explicit children and supplied
parent common-motion/finalization, then 500 further real component frames on each
of two tickets. It observes both reference clears before the first test release,
two residents/six registrations removed, seven headless visual owners reduced to
five, retired mesh/image/material removal and an unrelated entity preserved.
The isolated world control separately checks external-release failure and no
replay. Neither control establishes natural awakening or actual device/GPU input.
Workspace validation: 987 Rust tests passed / 176 ignored, 45 Python tests passed,
strict all-target Clippy and formatting passed.

Active popup/dirt effects, reciprocal damage and source death remain
prerequisites; the ordinary firing/flight path is connected above. The marker's prototype PS2 attribution remains
unassigned: literal `mound_move`, `mound_animselect` and `mound_shot` are additional
search candidates, not confirmations. Generic cleanup names also remain unknown;
see [SYMBOL_NAVIGATION.md](SYMBOL_NAVIGATION.md).

The animated-projectile local GNU build is source
`83aade9bb5db5b0f5d5dfe5ccd444cb491a669fc`, executable SHA256
`ad3b4d88384dcc85c623df72b0cad98dd94486e32a527e42d26f065d2dd5e488`.
The receipt and executable digest were independently checked. It includes the
flight-owned ANM/affine and source-skinned auxiliary draw consumer described above.
The five installed The Wife controls pass, including two-ticket first-flight
mesh/material update and renderer cleanup. On that earlier executable the supplied
full-contact diagnostic failed at the pending terrain contact at step2755 after137
flying samples. The newer missed-shot host above supersedes that CPU frontier;
clocks and drawing alone do not substitute for impact services. There is no
new GPU projectile/reciprocal encounter acceptance or published preview.
Use **Load Single Player Game → Mecc Story 2 → The Wife** with the ordinary
controls; neither AI nor projectile presentation needs a diagnostic flag.

The cooldown/retreat local GNU build is source
`857b59168c0664c109e299a919eec622fa02c049`, executable SHA256
`8c0d5cad92cfec682e8f3ab723a4c2542e6d650c1138871ca26fa68168968ba5`.
Its receipt and executable digest were independently checked. It includes the
current-scene flight consumer and class230 modes66/71. Use the same ordinary
menu path below. It has no new GPU encounter verification: the supplied-input
full-contact diagnostic stops at a real pending terrain contact after137 flying
samples at step2755. Projectile animation/draw and genuine impact, reciprocal
damage and death consumers remain prerequisites. The earlier flight-only build
was source `225f6fa3ece4b2545745410e41a3d8a103c6e80f`, executable SHA256
`b0f1d23f2929b0615137a8016f0c6920ea9fc71b765cfb97da931d8b22041381`.
Neither local build is a published preview or complete encounter acceptance.

The latest GPU-checked local build is source `8e5d72b004f7da6c7e92e215d8907b4be0c1cd8b`,
executable SHA256
`865d6ad15293710bc14cc80d798aff819cb26104a7f04f1d77f6c84eef5a5c78`.
This build predates the current-scene flight consumer. The GNU receipt was
independently checked. Its ordinary launch and normal-Mecc
contact producer are compiled; the exact-name supplied-input diagnostic still
fails at the ordered static/NPC scene/flight boundary. At step2618 it uses the
actual moving controller clip2/tick2839 and current dedicated geometry. No
projectile flight, reciprocal combat or death is accepted by this diagnostic.
Standard validation passes 987 Rust tests with 177 ignored, four installed
`source_wife_` controls, 45 Python tests and strict Clippy/formatting; the failing
approach diagnostic is separate from those passing results.

Actual Windows input on that executable enters The Wife through Load Single
Player Game / Mecc Story2 / The Wife, skips the first opening with Escape,
sends a movement tap and brief right-click, returns to the menu and enters
The Wife again. The second four-director opening reaches its natural source end
before the ordinary Mecc is observed. Tickets1/5 each submit nineteen current
body/contact packets (fourteen Vimps and five Rippers), without a Ripper/contact
refusal. An intermediate selection also opens The Road Home on ticket3 with
twenty-two packets (fourteen/eight); this is admission, not mission completion.
All three Sessions release to zero entities; F10 closes the owned test process.
Audio is muted. These are menu/render/re-entry checks, not a close-up Ripper
encounter, held flight, visible projectile, reciprocal damage or death test.
No new screenshot is published and no competing preview is released.

Earlier source-only deformation checks cover model6 across sixteen normal Mecc
clips at eighty supplied times. The new ordinary contact producer uses the same
prepared bindings with the actual controller sample; it does not acknowledge a
complete native state2 callback or whole-world vertex scheduling.
Both projectile models are skinned with an animation descriptor; static or bind
vertices cannot substitute for their animation. Some current rigid-scene entries
have declaration identities without constructed Session actors. Preserve those
distinct source keys rather than inventing canonical actor IDs for flight hits.

The earlier contact-only local GNU executable,
is built from source `c6e363c5a478bb89097970a38d28626d50e1b688`, SHA256
`803f9ac4a956f69af3b4fc8eb226d1fca85821d2d0946bdfdd8e014fa002f3d8`.
Its private build receipt was independently hash-checked. Use the ordinary
**Load Single Player Game → Mecc Story 2 → The Wife** path, then let the film
finish or use Escape to skip; Escape in gameplay returns to the mission menu.
No script or Ripper activation variable is required. This is a local development
build, with the incomplete continuations above, not a public preview.

Actual Windows clicks/Escape entered The Wife twice (fresh tickets1/3), displayed
ordinary Mecc gameplay, returned to the mission menu and re-entered. The first
opening was skipped; the second rendered all four directors and resumed Mecc
naturally. A real movement tap was supplied; F10 exited and process/window closure
was checked. Each entry submitted five completed source Rippers plus fourteen
Vimps both to the GPU body scene and to the current contact registry, without a
Ripper geometry/contact refusal. This verifies the connected registry, not a
close-up Ripper view, physical encounter, reciprocal attack/damage or death.
No natural marker closing/individual retirement was observed on GPU. Sound was
muted, and active FX consumers remain missing. Initial Ripper sound reservations
cancelled before playback, separately from the unknown FX scheduling fidelity.

The earlier marker-retirement build `3a3b3ab` / SHA256
`0fb0d8bcc44f3bf71915e9465cef77d336ff625bb2f7359a23a234b45cc856e4`
passed its separate ordinary menu/re-entry and body-submission check. It did not
include the new contact consumer or establish a complete Ripper encounter.

The first executable (`5fefde0`) reached ordinary Mecc gameplay with actual
Windows clicks/Escape and five completed Ripper actors, but its common renderer
refused the null-texture part. That failure is retained as a failed graphical
check. The earlier corrected executable (`0273154`) passed the headless asset consumer; its next
Windows check was stopped at the menu because another computer-use operation
was active. That failed attempt does not validate Ripper visibility, reciprocal encounter
or death. The later `230826c` surfacing build received no Windows/GPU encounter
check. The current menu/re-entry result above belongs only to the identified
`c6e363c` executable and retains its encounter limits. No new gallery capture or
preview was published. The previous loading baseline below remains
separate evidence.

### Intermediate local loading check

GNU release source `364c3036672de870500d018f23f58a662e0b2fc7` produced executable
SHA256 `7767186b1ce7b8d12a98ed152b1a5c1693233b2f0f371fc34c48dd30d45fd587`.
Targeted Windows clicks selected **Load Single Player Game → Mecc Story 2 →
The Wife** on that executable, with an independent test profile. Two separate
entries each prepared four class73 and one class230 canonical owners. The first
opening was skipped with Windows Escape; the second reached its actual source
terminal and resumed the rendered ordinary Mecc scene. Escape returned each
entry to the mission menu with zero remaining Bevy entities; F10 closed the game.
Audio was muted and no replay/probe controller drove this check.

This validates loading and cancellation with the new cold owners. Ripper
callbacks and rendering remained explicitly inactive. No reciprocal encounter,
Ripper death, held movement/input acceptance or audible fidelity is established.
The local build/interaction receipts and logs remain private; no preview was
published. The common integration lane still owns mission progression/publication.

## Ordinary Vimp collision, injury and death

Preview .11.3 extends the ordinary Story2 component with completed
collision-slot geometry, replacing only its dynamic contact tail while keeping
the declared static geometry. Player responses and partial-pistol queries share
stable prepared-scene keys, resolved separately to typed Session identities.
Queries retain terrain occlusion and the acquired distinction between selected
nearest distance and the shared scratch impact point. No guessed hit spheres,
teleported poses or canonical projectile identities are introduced.

Source-bearing nonlethal damage can now apply the shared impulse to the actual
physical velocity, select hurt mode and attach a wound at the impact. Attached
wounds retain their selected current animation joint and local point, owner
clock, pulse phase and lifetime. Their separate capacity is 50; generated blood
uses the existing 500-particle pool. Missing pose/animation providers fail rather
than silently emitting actor-centred blood. Regeneration resumes through the
existing living-primary scalar contract after hurt recovery.

Controlled installed and original-PC evidence, remaining native-world prerequisites and
delivery acceptance are tracked in BACKLOG.md. The immutable .11.2 package keeps
its earlier scope. Preview .11.3 connects ordinary hits, injury/death sound, blood,
death FX, ghost, meat and cleanup with exact-build graphical acceptance. Actual
Windows gameplay and complete native fidelity remain unverified; see PREVIEW_NOTES.md.

The 120 Hz approach/hunt control additionally covers a hit while a danger
response is active. The native post-switch active block also follows hurt modes;
the ordinary recovery caller consumes that existing block through the current
primary scope and actual response callbacks before physics/ANM. Response and gait
words are preserved. The isolated recovery API still refuses active responses
without this caller/provider, and no mode1 command generation or threat selection
is inserted into the hurt dispatch. The original-PC primary lookup is reused;
the prototype PS2 `vimp_logic` correspondence remains unknown.

Attached Vimp cues use the canonical completed actor position, not a missing
presentation-position mirror. Their source descriptor bit0 requests a loop;
bit0x20 reuses a live same-sample/same-owner voice before allocation or a new
audio RNG draw. Forty original owned-start controls cover empty/matching/stopped/
other-owner/other-sample lists, loop combinations and two supplied audio seeds.
Allocator, position upcall and device returns remain supplied in those controls.
The Bevy transport consumes those two bits for attached voices, follows their
actual owner and retains exact-owner stop/retirement. Other descriptor flags and
hardware behavior remain outside this acquired profile. Muted real-sink tests
are separate from native timing, mix and audible-fidelity validation.
The current source also owns the retained ghost actor/ANM, its state84 meat
presence lifecycle and loose-meat expiry. Those installed dependency cycles
still supply birth/admission/time and external-release boundaries; they do not
by themselves establish a rendered ordinary lethal interaction. The later
connected pistol control exercises the combined source consumer; the matching
preview adds separate graphical acceptance recorded in PREVIEW_NOTES.md.

The common scene consumes the actual loose-meat affine and retained ghost ANM,
with separate shared unlit materials, source transparency and per-owner buffer
cleanup. Ghost RGB addition and additive blending retain the original checked
renderer state program. Source asset controls and the matching preview
graphical lethal passage pass; native fidelity and OS gameplay remain unverified. The NPC component owns attached wounds and blood
particles; its renderer reads copied quads only. Injury/recovery sound requests
use the existing actor-owned transport, with playback acknowledged only by its
real sink consumer. The installed nonlethal bridge supplies a hit explicitly.

The damage API provides the actual mutable Session to its required death service.
The ordinary adapter owns its FX graph under separate typed visual keys, so those
keys cannot be mistaken for Session actor IDs or native pointers. It consumes
actual source programs/classes/ANM/textures, actual full/lite selection inputs
and shared visual, reference, movement, gravity and terrain kernels. Host-frame
FX scheduling, terrain-only contact and splat retention for the requesting
source object's lifetime remain explicit preview policies. This does not admit
or acknowledge a full native state78/world/decal callback. Prepared full/lite
and nested skip frames alone still do not acknowledge active execution.
The positional wrapper's inherited angle basis lives in its own payload field,
distinct from the translated birth affine and from the state78 SIMD Euler path.
Immediate child references remain the actual fixed own/parent pair. Emission
windows and bounded impact-child counts have real consumers; no death request
is acknowledged by a recording-only provider. The connected installed control
uses actual ordinary pistol amounts/trajectories and supplied navigation/time,
with three kills per entry, real source ghost/meat cycles and complete owner
retirement. Its external release boundary and copied render packets are not
GPU, audible or OS-input proof. See BACKLOG.md for final build acceptance.

The common recovery invocation preserves the current threat population and
control words: native hurt modes2..4 do not run mode1's threat traversal. The
isolated API retains its stricter admission. Actor-owned audio retirement stops
real started sinks, cancels unstarted reservations without playback acknowledgement
and removes stale queued starts after their actual actor owner disappears.

## Story2 living baseline and retained reconstruction evidence

The active Mecc-loop delivery now composes actual Vimp threat consideration,
response, route selection, movement and heading within the existing owned common
primary. A private invocation scope pins the actor, herd parent, context, frame
and helper counters. Existing isolated entry points retain their refusal and
replay rules. History, physical placement, ANM, references and event preparation
remain single consumers; the world-running flag is never switched off to bypass
those guards.

Noise response enters mode7. The new bounded selector requests the source
forward/backward startle clip, waits for the actual channel completion and
single-player predicate, then resets the source noise/priority/timer fields on
return to mode1. Higher-priority returns, linked targets, feeding and other
unsupported branches still refuse. An original-PC instruction control compares
768 finite mode7 cases and 6,912 selected words/requests. The numeric selection
wrapper is an intercepted request boundary; this is not a full-primary or visual
comparison. Prototype PS2 attribution remains unknown.

An installed supplied-clock cohort completes 50,400 common callbacks, 252 startle
entries and 246 returns, consuming 2,253 actual sound-only event contexts with no
pending contexts. Its flat terrain, noise emissions, session and absent sound
bank are declared control boundaries. A separate actual-Story2 terrain probe
also completes 3,600 supplied steps with repeated noise. These controls alone
establish no ordinary playability.

The .11.2 baseline connects a first-interaction component to the ordinary
Mecc controller on Story2. It uses the existing mission Session, authored herd
and marker identities, controlled player position, sampled host clock, owned
slow-area namespace, actual pistol noise and shared partial-runtime RNG. The
existing renderer consumes completed canonical poses. Source sound-only contexts
feed the same queued positional transport as other ordinary cues; only observed
AudioSink playback acknowledges a voice. Failures latch without replay.

This explicit component population does not certify a native common-list/world
update. Other staged AI families remain inactive and receive no successful
default callbacks. Original full populations, admission and several native global
writers remain unverified. Preserve the roster/admission limitations below;
do not report the earlier detached CPU cohorts as ordinary mission evidence.
The connected installed control uses actual Story2 terrain and declared rigid
geometry, actual player motion/pistol emissions, and supplied timestamps/commands.
On each of two tickets it completes 6,000 component frames with 14 Vimps, 259
startle entries and 253 returns, queues 3,700 sound requests and clears the mission
on cancellation. Repeating a sampled frame consumes no RNG or duplicate sound.
This is CPU ownership/reaction evidence, with no device or OS-input acknowledgement.
Published .11.2 graphical runtime `0da06b4` (executable SHA256
`891b16220b9609e062bf48ae0ec6eb8d0dba234c87e39440450dacaf19dcfc56`)
passes actual Story2 admission, supplied normal Mecc navigation/fire, canonical
14-Vimp rendering, 14 startle entries and 7 returns before capture, and actual
muted positional sinks. It reaches 44 source units from the real herd without
forcing actor positions or readiness. The first source cue is prepared on demand
through the existing sound registry. Supplied commands do not establish ordinary
OS-held interaction or native complete-world scheduling. The same runtime is delivered as preview .11.2 after common-runtime/lifecycle
acceptance. Real OS-held interaction remains the next acceptance gap.



Select and resume work under the shared [gameplay-first
objective](GOAL_GAMEPLAY_FIRST.md). Prioritize the ordinary player-visible path and
its first missing caller/consumer before rare variants. Dated controls and
next-acquisition notes below retain their evidence scope; they do not override the
current delivery priority in `BACKLOG.md`. A handoff must identify the exact
testable build and normal controls, or state "intermediate dependency only" and the
next runtime blocker.

The NPC lane resumes the existing source-backed services rather than starting a
parallel AI framework. `BACKLOG.md` remains the detailed backlog. This inventory
identifies implementation boundaries; it is not a claim that these inhabitants
already run in normal missions. Character-player controllers remain in their own
Mecc, Delphi and Kabuto lanes, and cinematic direction remains in the Flick lane.

## Where can this be tested?

The next Mecc/living-interaction source checkpoint adds real ordinary-pistol
noise publication and a ticket-bound clock sampled from Bevy's monotonic real
frame snapshot. Reset and first-frame timing preserve their distinct observation
boundaries, zero timestamp behavior, smoothing reset and retained counters.
The shared noise kernel preserves the owner, clock and position words; its App
producer reads the current controlled actor, not an invented muzzle/noise target.
One thousand twenty-four original-PC reset/first-frame/noise controls agree with
the Rust kernels; only the OS clock import is supplied in that comparison.
Prototype PS2 naming remains unknown. The placement after partial pistol firing
is an App integration boundary, not full native primary/caller-order validation.

This source is not in published .11.1 and enables no live NPC yet. It does not
provide admission, slow-area/control/heading populations, complete callbacks or
reaction services. A diagnostic actual Story2 birth cohort advances its fourteen
children for 3,600 supplied 60 Hz steps on the source terrain without a quiet-wander
refusal; this remains a supplied CPU probe, not ordinary gameplay evidence.

The owned slow-area registry now preserves newest-first priority, copied centers,
current owner-context lookup, countdown expiry and retirement/clear order. Its
384 original-PC scenarios agree on 42,864 selected words through real creation,
query, update and clear routines; allocation/free remain bookkeeping boundaries.
The ordinary App advances the registry before actor work. A future utility/event
caller must create real entries: current input exposes none. This is not proof
that an original mission has no areas or that all native birth callers are bound.

The main consolidation consumes the typed parented state0 callbacks in ordinary
Mecc jet presentation. Right-mouse thrust and Space nitro use the same three
constructor-bound children, canonical ANM/reference owners and shared position
buffers; the renderer has no copied child channel clock. The installed
1,600-step combined control covers thrust, nitro, release, descent and landing
with each child's callback count checked. Preview .11.1 additionally passes
combined supplied-controller GPU cycles and an actual OS right-mouse pulse,
menu return/re-entry and normal exit on the exact executable. This does not admit Vimp AI or certify the
native common-world schedule.

| Entry | Current availability |
| --- | --- |
| Ordinary public preview maps | No active Vimp herd/AI is admitted yet. Authored herd/marker construction does not populate a live herd. |
| Story2 (`w_Story2.bin`) | Reference resources for installed CPU controls and first intended ordinary-mission integration; not a playable AI test yet. |

Source/CPU controls below do not identify a map on which their behavior can
already be exercised interactively. Ordinary admission still requires actual
world services and validation.

## Eventless state0 actors in the shared callback path — source dependency

No ordinary Vimp AI map is available. The common-node adapter now also accepts
constructed, unparented nonphysical state0 actors whose actual source has no
animated channels and requires no external orientation. Reference-bearing state0
actors use the acquired absent-primary guard described below. Their
primary/secondary state dispatch entries are acquired absence; ANM/time and
finalization still execute. Unsupported state0 profiles receive no successful
callback by default. Existing herd/marker behavior and older API names remain;
`passive_frame` and the Session `passive_world_*` entries name the shared profile.

The actual constructor creates this owned frame only for the supported source
profile. The existing clock, ANM epoch, finalization, environment and footprint
contracts remain required. A failed or consumed iteration cannot replay, and
retired, untyped-parented and changed profiles refuse. This adds no independent simulation or
new random stream, and enables no callbacks from mission loading by itself.

A focused original-PC control constructs two neutral state0 classes after poisoning
prior pool and template transient fields. The real constructor clears payload/
physics, rates, commands and contact storage. Their actual common-node callback
order matches the existing state0 marker path. Rust agrees on 156 selected numeric
class/pose/history/scale/angle/flag/channel/rate/affine words. Control presence,
clock, placement/history, slow-area population and admission remain supplied;
resource pointers and UID are excluded. Prototype PS2 lookup remains unknown.

The initial empty-reference checkpoint admitted 129 actors. Current Story2
declarations admit 133 empty-channel neutral actors to this shared profile.
Three supplied callback steps retain their positions, publish current affines and
preserve RNG across 399 updates; replay and retirement refuse. Terrain/platform/
footprint providers are required but not entered by these stationary actors.
The additional animated state0 declaration is supported by the bounded eventless
single-channel path below. This is a selected-cohort CPU
control: other actor families are not acknowledged as updated, and empty supplied collision geometry is not a claim of
an empty mission. Parent completion/attachment publication, wider state0/AI
callbacks and the actual shared world admission/clock/control/environment/effect
producers still block
ordinary whole-world scheduling before the live Vimp render consumer can be used.

## Active eventless state0 animation - intermediate source dependency

The actual Story2 building now advances its declared channel through the shared
channel loop, stores the shared epoch, samples its fifteen owned references and
then finalizes its affine. Eligibility requires a real positive-duration primary
descriptor, empty backup channels, an explicitly empty event track and no reference
corrections. State0 has no supplemental state2 event dispatch. Generic parent
relationships, external orientation and eventful profiles remain outside this
adapter; the typed Mecc children below have a separate ownership guard. Earlier
herd/passive identifiers and empty-channel guards remain available.

A focused original-PC control observes five callbacks: initial forced zero time,
positive time, a loop crossing, another forced zero and unforced zero. Three
source/native projections agree on 999 selected words, including channels, rates,
affine and all references. Pointers, UID and numeric epoch identities are excluded;
clock/control/history/placement/admission remain supplied. Prototype PS2 lookup
remains unknown; no signature, ABI or memory layout is imported from it.

The installed selected cohort now runs 134 actors /402 callbacks, consumes the
animated actor through the actual shared position buffer and retains the four
empty-channel reference owners as unsampled. An explicit environment-gate fault
checks that a late failure retains ANM clocks, epoch and references and refuses
replay. It is a supplied fault, not an acquired ordinary flag writer. These checks
do not update other families or supply the whole-world scheduler. No new ordinary
Vimp map, GPU interaction or preview is established.

## Typed parented state0 callbacks - intermediate source dependency

The acquired Mecc special constructor now binds the shared nonphysical frame to
its three supported children. Their channels and retained header mirror agree at
construction. Admission requires the actual typed constructor group, exact parent
identity, binding words/context and supported eventless state0 resources. A generic
presentation relationship cannot qualify, nor can a detached or replaced mirror
silently turn a typed child into an ordinary unparented actor. Unsupported resource
variants still construct without receiving an acknowledged callback by default.

Common history, primary/secondary dispatch, object time and ANM now execute for
these children through the existing owner. Parented finalization retains its real
early return: positions/history and the dirty flag publish, environment/affine
production do not. No identity matrix, parent pose or completed parent turn is
invented. The existing attachment consumer still needs its actual current parent
pose; the normal presentation path remains separate from world completion.
Before enabling the world caller, Mecc/main must switch normal jet presentation
from its constructor-snapshot clock to completed Session child channels/references.
Its current guard intentionally refuses mutations outside that presentation owner.
Keep one animation producer and preserve that guard until the consumer handoff.

An original-PC special constructor plus twelve common callbacks match 800 selected
Rust words across initial forced zero time, positive time, loop crossing and
unforced zero. Fields, all channels/rates and actual child references agree;
parent-pointer retention and unchanged native affine storage are checked
separately. Two installed tickets run 24 local callbacks with actual position-buffer
consumption, unchanged RNG/parent channels, typed binding retention, replay/foreign/
retirement refusals and no fabricated world affine. Callback admission, control,
clock and prepared history are supplied; parent completion and whole-world
scheduling are not established. Prototype PS2 attribution remains unknown.

## Retained references behind an absent primary channel

The four additional Story2 declarations retain actual reference resources despite
having no active animation channel. The native reference routine returns before
sampling, correction or matrix writes when the primary descriptor is absent.
The common ANM adapter now calls the existing owned reference pass after a channel
loop was entered, preserving that guard and storage state. It does not supply an
identity/bind matrix or acknowledge a current reference pose. Zero time without
a force flag skips that pass; positive time or an explicit force entry executes
it after the real ANM epoch store.

Six original-PC common-node callbacks over two actual classes cover zero time,
positive time and forced zero-time entry. A deliberately nonzero reference-cache
fixture remains intact with zero observed writes. Static instruction inspection
and retained decompiler evidence independently locate the guard. Two positive-time
node projections match Rust on 156 selected pose/flag/channel/rate/affine words;
reference-cache fixture values, epochs, resource pointers and UID are excluded
from that pair. Separate source guard controls preserve the same nonzero supplied
matrix and refuse current-reference consumption. The 133-actor installed cohort
checks four real reference owners, retained constructor storage, force-flag clear
and stale/unsampled refusal across its 399 supplied callbacks. Admission, controls,
clocks and the force-flag input remain supplied; no ordinary writer is inferred.
Prototype PS2 attribution remains unknown. Later sections acquire the animated
building and local typed child callbacks; parent completion/attachment publication,
other AI families and real shared world producers still block ordinary Vimp play.

## Vimp pose consumer in the common renderer — source only

No ordinary mission or public preview can yet exercise live Vimp AI. The render
consumer is now connected to the common application; actual world/birth admission
is still missing. No normal controls, GPU observation or playable map is claimed.

`Session::completed_vimp_frame` publishes a local completed-callback count and
its actual animation epoch. It refuses cold/failed/stopped callbacks, unfinished
history or another running actor phase, and inconsistent channel ownership. The
mission consumer additionally checks the exact factory/specification identity and
current affine. A previous pose after next-frame history is not a current frame.
The shared position evaluator uses those actual channels and epoch; each actor's
result is copied before another Vimp reuses the shared model buffer.

The normal mission renderer consumes these packets through the existing RGB body
shader, source geometry and textures. It uses primary model slot zero as an
explicit presentation policy; native visibility/LOD, mirrored culling and wider
transparent/additive profiles remain incomplete. Unsupported profiles refuse;
no replacement body, bind pose, missing affine, effect, random draw or animation
clock is supplied. Cold births produce no body before their first completed
callback. Geometry is cached by the mission resource owner; render entities and
mesh/material/image handles are retained by the actual actor Id and ticket.
Camera absence hides the submissions; retirement removes them. A required asset
failure releases partial ownership and quarantines this ticket's renderer.

The installed CPU control enters canonical Story2 staging twice, retaining its
other actor families. With explicit offline birth/callback admission, quiet
signals, supplied clocks/epochs and actual terrain queries, each ticket produces
fourteen owned Vimps. Their completed poses populate and update Bevy mesh assets,
remain separate across shared-buffer reuse, hide/resume with the camera and
release all owned meshes/materials/images. The second ticket checks missing-mesh
cleanup and quarantine while preserving an unrelated entity. Fresh history,
foreign factory resources and retired binding refuse. Repeated render sampling
consumes no gameplay RNG. These are CPU Bevy asset checks, not GPU pixels,
mission-wide scheduling, ordinary controls, AI behavior or native visual fidelity.

The next missing producer is the real shared world scheduler and its admission,
history/control/clock/RNG/environment/effect services. It must preserve all active
families and same-step birth order, then exercise this consumer with ordinary
menu/input/exit/re-entry. Do not run the offline control from loading or advertise
its fourteen-body result as a playable herd.

## Owned Vimp contact producer — 10 November 2026

Intermediate dependency only: Story2 remains unavailable as an interactive Vimp
AI test. At this contact checkpoint, shared world admission and the render
consumer were still missing; the later pose consumer above closes that render
boundary, with live admission and interaction still open. No normal controls or
downloadable preview change follows from the contact checkpoint.

The exact PC constructor clears the initial contact word independently of its
previous pool and class-template contents. A focused two-construction control
poisons those inputs separately and records the actual clearing write; static
inspection independently locates the constructor's clearing phase. This resolves
the initial contact prerequisite, not later collision population or AI readiness.
Prototype PS2 naming was searched before acquisition; attribution remains unknown.

The Vimp owner now retains that word through construction, history and physical
updates. Physics consumes the retained value, publishes its acquired clear before
an attempted ground query and retains the result across later errors. The bounded
ordinary wander route refuses an unsupported nonzero contact before consuming
random draws. Movement/active response can read the owned word directly; isolated
controls retain an explicitly labelled supplied predicate instead. The physical
object-walk gate and other acquired profile limits have not been broadened.

Installed controls exercise the constructor value, the owned movement branch,
unsupported-contact refusal, retained clear after a required-ground failure and
replay quarantine, plus newborn and locomotion continuation. Nonzero contact
fixtures are supplied ownership checks, not observed player collisions. The
response arrival timer remains separately unknown until its own producer runs.

The actual Story2 staging roster has 166 common-list actors: the controlled Mecc,
three parented Mecc children, one herd, sixteen markers, 134 other state0 actors
and eleven state23 actors. The children precede the herd. A scheduler cannot
acknowledge those active actors with default-success callbacks, assume an empty
threat population, or run a detached herd-only walk as ordinary mission proof.
Reuse the existing Session and coordinate its real family callbacks, clocks,
random stream and history before consuming same-step births in the renderer.

## Ordinary mission resource binding — 10 November 2026

Intermediate dependency only: no normal controls or public build exercise live
Vimp AI yet. The first missing runtime consumer is common world callback admission,
followed by actual live population and interactive render validation. A later
source consumer above now accepts completed poses. NPC owns these actor resources;
main integration owns the shared schedule and combined interactive delivery.

The ordinary mission worker now prepares the Vimp factory from the actual plan's
registered specification and effect namespace, plus the installation's birth
constants. Canonical mission staging binds the existing herd/marker constructor
IDs and source declaration keys to the mission ticket, then creates a deferred
animation-effect pool from the same captured image. No filename-selected image,
constructor contact zero, clock or RNG seed is substituted. Missing/replaced
namespaces, failed construction, foreign/retired owners and repeated binding
remain explicit failures. Worlds without authored herd parents skip preparation.

The Story2 source entry control binds one herd and sixteen markers on each of two
mission tickets, retaining zero Vimps/effect contexts and untouched herd clocks.
It uses actual menu data with supplied unlocked progress and the canonical spawn
path, checks ticket/rebind/retirement guards, removes the owned entity tree and
preserves an unrelated entity across re-entry. This is a silent CPU entry/binding
control; ordinary input and GPU behavior remain unverified. A separate resource
control checks the captured image and missing/replaced namespace refusals.

Preparation now reaches the mission owner; it still executes no population or
world callback and makes no rendering/readiness acknowledgement. Next connect
the real world admission/clock/control/terrain/effect providers and exercise the
connected pose consumer with actual admitted births/animation before advertising
a testable map.

## Class230 awakening through the owned primary - source dependency

The initial Story2/Starving record stages seven class73 and four class230 actors
in state23; The Wife's separate declarations are documented at the top. This
section retains the earlier awakening acquisition evidence. The existing
primary machine now also admits class230 mode0/61 through its actual constructor,
shared entry/perception and motion phases. The older Ripper identifiers remain
aliases. This adds no whole-world callback admission or playable AI map.

The class-specific entry cue and parameter handling remain distinct. This branch
reads camera range from actual global resources and re-queries object-local time
after entry may change the dispatch context; the earlier prefix delta cannot be
reused. An expired timer and current selected target request the awakening cue,
store mode62, clear the acquired visibility/personality fields, construct the real
class219/state48 marker and publish its reciprocal owner before stopping the old
cue. The marker birth is unconditional on height. The debug history-tail hook is
empty; the destination-copy helper writes thirteen destination words and no source
words. This route does not import the class73 surfacing motion reset or speed
restoration. Shared footstep dispatch retains the class230 cue.

The Session caller uses the captured source resources and existing marker factory,
retaining exact owner, marker and target identities. Callback errors with an
acquired failure latch publish their owned prefixes, including a marker already
created before a later audio error; they refuse replay. Pure preflight errors
still acknowledge no callback. An installed control covers actual construction,
conditional birth, prefix and late failures, copied pose/scale, retirement and a
separate post-entry slow-area clock. Audio requests are explicitly captured and
suppressed by the test, not treated as a live sound backend or native absence.

Six native diagnostic sequences acquire sixteen steps each with actual callbacks,
resource resolution and a supplied peer/flat world. They do not establish six
playable missions or complete behavior. Three focused class230 source/native first
primary-plus-motion projections match 90 selected words. Native sound-registry
absence and source diagnostic suppression are declared boundaries; the equality
establishes no audible result or DLL/audio RNG ownership. A focused copy trace
observes thirteen destination writes and zero source writes, agreeing with the
independent PC decompilation. Prototype PS2 lookups remain unknown, with no new
confirmed attribution or imported signatures/layouts.

At that checkpoint, mode62 and subsequent continuations, the spawned state48 callback schedule,
secondary/ANM/finalization, real player pose/vitals and complete ordered perception,
actual voice/effect/RNG/clock producers and menu-to-mission interaction remained
required. Normal Mecc presentation is not a substitute for an up-to-date owned
player view. Keep those refusals until main connects and verifies the real owners.

## Existing reconstruction

| Family | Existing Rust services | Established boundary | Missing ordinary gameplay connection |
| --- | --- | --- | --- |
| Herd population | `herd_generator`, `herd_markers`, `declared_residents` | Source herd/marker construction, authored properties, ordered marker allocation, offline clock and Vimp births | Mission scheduler, native live clock/RNG ownership, complete population and retirement |
| Living Vimps | `vimp_actor`, `vimp_wander`, `vimp_physical`, `vimp_animation`, `vimp_frame`, `vimp_threat`, `vimp_response`, `vimp_vector_response`, `vimp_active_response`, `vimp_movement` | Owned birth/initialization/growth and admitted ordinary wander/physical/ANM composition; isolated owned threat/response/vector/active composition and persistent arrival timer | Required hint/action bodies and live session/caller ownership, feeding, full-world order, live render and audio ownership |
| Vimp injury and remains | `vimp_damage`, `vimp_hurt`, `vimp_recovery`, `vimp_living_scalars`, `vimp_chunks`, `vimp_death_proxy` | Bounded damage, hurt/recovery and owned chunk/proxy services | Feeding/economy, objective producers, eligible mana and complete native-world admission; bounded hit/effect/cleanup and pickup consumers are connected |
| Rippers | `ripper_perception`, `ripper_steering`, `ripper_pursuit`, `ripper_primary_turn`, `ripper_attack`, `ripper_awakening`, `ripper_emergence`, `ripper_turn_completion`, `ripper_ground`, `ripper_dirt` | Ordered target selection, supplied-view behavior kernels and persistent Session adapters for acquired branches | Complete actor/world primary scheduling, required firing/effect/audio providers, remaining modes and ordinary mission admission |
| Shared enemy response | `state23_entry`, `state23_damage`, `state23_hurt`, `state23_animation`, `enemy_projectile`, `enemy_ballistic`, `enemy_flight` | Acquired state23 entry/damage/animation, enemy projectile birth/launch and isolated ordinary flight arithmetic | Complete enemy projectile world/contact/damage composition and all enemy classes/bosses |
| Smartie handling | `smartie_carry`, `smartie_drop`, `carried_actor` and Session pickup/release services | Acquired Mecc carry/detach and owned relationship services | Complete Smartie initialization, independent AI, rescue/follow/work/fear, speech and worker economy |
| Other interactive inhabitants | Shared actor, targeting, physics and damage services | Reusable infrastructure only | Other fauna, allied squad decisions/orders, faction behavior, buildings/shops/vehicles and scripted inhabitants require separate acquisition |

`mission_runtime` constructs and binds supported authored declarations, including
herd generators and markers, and now prepares their registered Vimp resources.
That construction does not execute their primary
callbacks or populate a live herd. `vimp_frame` explicitly requires world/history
admission and required services. It must not be enabled by fabricating sound-stop,
feeding or threat callbacks. Ripper perception uses ordered current views and
validated health/ownership; a synthetic candidate array does not prove that the
complete live contact population is available.

## Evidence and reuse

Read the previous private research archive in place. Its source controls for
herd birth/markers, Vimp lifecycle, Ripper perception/primary sequences and
Smartie carry/drop are investigation aids, not public fixtures or an alternative
runtime branch. Inspect individual services and their evidence before adapting
anything. Retain supplied boundaries, rejected cases and previous identifiers.
Keep raw listings, control transcripts, source snapshots and native binaries out
of Git; publish only reviewed independent code and minimal validation summaries.

Follow [RECONSTRUCTION_METHOD.md](RECONSTRUCTION_METHOD.md) before revisiting a
native function: pin the actual PC binary, search prototype PS2 names, retain raw
symbols separately from ambiguous demangling and keep attribution confidence
separate from behavior/runtime validation. Initial herd-primary and Vimp-primary
lookups remain unassigned; matching vocabulary alone establishes no attribution.

## Threat consideration — 10 November 2026

`vimp_threat::consider` now preserves the observed signed-priority gates,
ordered/lazy nearby and peer reads, persistent signal flags, linked-presence
condition and remembered noise. Distance tests are strict and use XY; the noise
age limit includes exactly one second. The returned boolean reports new evidence
in that invocation, independently of existing flags. The noise level is stored
as a float before comparing the still-wide sum against its threshold.

Original-PC helper/distance execution matches 1,505 controlled cases /9,030
output/read-count words, including exact boundaries and a rounded-store threshold
case. The inputs are synthetic actor/payload fields, ordered lists, parameters
and noise under Double53. This is a helper comparison, not a complete primary,
source-world or live player-response comparison.

`Session::consider_vimp_threats_offline` resolves actual owned list order for an
initialized Vimp. It retains completed flags/noise writes on a late error and
quarantines that owner against replay. Two installed cohort controls check
once-per-noise memory, foreign/wrong invocation refusal, unsupported links and
late finite-profile failure. The latter tests Rust failure handling, not native
exception equivalence. Nonzero linked state still requires its owned producer.

The ordinary primary is not switched to this isolated entry. The response
dispatch below now preserves bounded selection. Its vector/route service below still requires hint/action
bodies, linked/physical/session input binding, required audio/ANM services, caller timing
and actual world noise production remain open. No fleeing, AI mission admission or
visual availability is claimed. Prototype PS2 attributions remain unassigned.

## Response dispatch and session predicates — 10 November 2026

`vimp_response::respond` now preserves response priority, pending-flag consumption,
physical/channel entry gates, nearby/peer transitions, linked/noise target writes
and orientation. Required providers observe preceding stores. Each session query
supplies its current snapshot; the kernel executes `actor_session_predicates`
rather than accepting a supplied decision. The predicates preserve first-match
selection, signed count-minus-one fallback and low-byte actor-word comparison.
They do not produce live session or actor ownership. Projected state is not
automatic Session publication: an owned adapter must publish each prefix before
provider callbacks and retain callback writes/retirement rather than restore a
stale working copy after the call.

Original-PC predicate wrappers and their actual callees match 2,840 contexts /
5,680 output words. Response controls match 1,024 contexts /28,672 projected
state/request/RNG/query words: 927 run to the helper return, 75 stop before the
required vector/route callee and 22 before the required action callee. Those
97 stopped entries are prefix evidence, not completed responses. No callee return
is substituted. Byte-only target requests remain distinct from nearby float/word
requests. The random interval retains a wide subtraction, and equal bounds still
consume a draw. Source TLS seeds and session/actor/physics/channel/affine/target/
noise fields are supplied; this is not a live ownership or full-primary proof.

Rust controls retain preceding priority/target/delay/RNG stores after required
provider failures. No successful default is installed. The isolated owned entry
below now binds inline physical flags, channels and the vector route. The ordinary
primary remains refused pending caller timing, actual session/linked/hint owners,
mode7, required action transport, sound and ANM. The physical flag word already
belongs to the inline Vimp payload; its omission from the physical view does not
mean it is unavailable or justify supplying zero. Retained source controls show
1,344 Vimp snapshots with the physical pointer equal to the payload base. A future
adapter must read the current owned word, respecting phase and owner lifetime.
No ordinary fleeing, rendering or gameplay download follows from this milestone.
Prototype PS2 attributions remain unknown.

## Vector response and marker retargeting - 10 November 2026

`vimp_vector_response` implements bounded direction selection, sparse-route marker
advance and retargeting. It preserves inclusive XY marker distance, wrapping visit
counts, conditional parent-selection movement and the unconditional parent-position
copy. Retargeting consumes three independent draws and preserves the target Z.
The direction uses one turn draw and a single angle correction; exactly 360 stays
360. Owned session snapshots are queried again at each native predicate point.
Retarget actions clamp XY to the acquired range before packing doubled signed
coordinates; heading requests retain their separate typed shape.

The original PC debug-reference-1.1 vector helper at `0x00519590` and retarget helper
at `0x00519e80` were compared against Rust in 676 contexts /22,308 state, route,
request, RNG and query words. There are 393 complete helper returns, 248 stops
before required action execution and 35 before the terrain/hint service. No
successful callee result is substituted. Inputs include synthetic actor fields,
a sparse four-slot route, session snapshots and TLS seeds. Some contexts supply
a session-mode transition after the actual retarget return to check the outer
predicate reread. Fields outside the acquired write projection remain unchanged.
These are bounded helper controls, not a complete response-primary comparison.

Required hint and action providers receive mutable projections; late callback
writes and errors survive, optional hint identity is reread after retarget, and
the common response tail is reached only after successful required providers.
The isolated owned adapter below now publishes each prefix into Session and
rereads callbacks. The standalone kernel itself owns no replay guard. Five Rust
controls check route wrap/selection, exact angles, predicate skips, RNG phase,
hint identity rereads and late failure retention. Complete action transport,
hint terrain/affine work, live session/linked ownership, mode7 and exact primary
scheduling remain open. This source-only dependency adds no normal gameplay
availability or capture.

## Owned vector route publication - 10 November 2026

`Session::advance_vimp_vector_offline` binds the vector helper to an initialized
source herd child, its real parent and ordered owned marker table. Actor UID,
position, heading, current affine columns, source marker radii, payload counters,
retarget offsets and response fields come from their actual owners. The vector
projection now contains only the fields it consumes/writes; unrelated response
links and physical flags remain in their owned payload rather than needing
fabricated values. Entry requires an explicitly prepared current affine, no
pending history/physical/ANM/world phase and a fresh invocation number.

Every predicate sees preceding actor and route writes in Session. Required action
callbacks receive the current owned actors. After their return, including an
error, the adapter rereads actor fields, offsets, mode, response payload and route.
It preserves callback changes and refuses lost/mismatched owners without restoring
a stale projection. A successful common tail may still overwrite fields at the
native later point. Failed invocations quarantine the surviving actor; replay and
reentry refuse. A Session cancellation never recreates dropped actors/resources.
These counters count isolated helper invocations, not completed primary frames.

Six installed-data Rust controls cover a fourteen-child source cohort, route
publication before the predicate, changed heading/route and unrelated payload
writes, required-action errors, injected individual owner loss, actual Session
cancellation, foreign/reentrant/stale-affine refusals and unsupported hint state.
The native 676-context helper comparison also remains passing after the projection
and predicate-interface changes. This is not a new native comparison of the
Session composition. Test session snapshots, affine entry phase, origin and RNG
phase are supplied; callback changes/failures are declared control scenarios.
The original child UID is retained, including its low-byte signed-count fallback
in the supplied action-admitting snapshot. No live session producer is inferred.

Nonzero hint state refuses before invocation until its actual typed owner and
terrain/affine producer exist. Required action execution has no successful default.
The stricter owned-entry preflight requires the available route/affine even for a
predicate-skipped helper; it is not a native equivalence claim for missing owners.
The isolated response entry below now composes this vector adapter. The ordinary
primary's threat/response refusals remain. Acquire actual caller phase, required
hint/action/ANM/audio and mode7, and establish actual session/global/world
scheduling before normal gameplay admission.

## Owned response dispatch - 10 November 2026

`Session::respond_vimp_offline` now reads the actual inline physical word, mirrored
primary-channel flags, actor UID, control/binding words, response payload, cached
forward vector, current position and delay. A signal response publishes priority,
remaining signal and cleared flags before executing the actual owned vector entry
with the same RNG. Nearby responses retain their source delay interval. Noise
responses retain the actual target coordinates and update only the backwards
byte; its three opaque neighbouring bytes are preserved.

Session snapshots are still current supplied inputs, not supplied predicate
results. Each query sees preceding response stores. Terminal action callbacks
observe the published prefix and may change or cancel actors. Their resulting
Session state is retained on success or error; the old kernel outcome is never
restored afterward. The nested vector path rereads the full acquired response
projection, including callback changes outside the smaller vector projection.
Fresh invocation counters, pending-phase guards and failed-owner quarantine remain
separate from completed world/primary frames.

Six installed-data Rust controls check signal-to-owned-vector composition,
physical/channel gates, noise targets and opaque bytes, nearby action prefix
visibility, terminal callback writes on success/error, nested vector failure,
actual Session cancellation and foreign/reentrant/stale-affine/link refusals.
They use real source actors with declared isolated priority/signal/channel entries;
setting the test channel-ready bit does not acquire its native producer. Session
snapshots, noise, initial affine phase and global RNG phase are supplied. Required
action callbacks in the tests are controlled mutations/failures, not transport
implementations. No new native comparison of this Session composition is claimed.

A separate original-PC caller control retains real construction, births, common
scheduling and primary execution, injecting only the warm signal flag. It observes
29 actual response entries: 28 with animation not ready, then one complete signal/
vector response after readiness appears. Physical and channel flags are not forced
in that native control. A Rust response/vector comparison from those captured
helper entries matches 957 projected state/RNG/route words. Captured entry fields
and source route are inputs to this comparison; it is not Rust whole-primary or
Session-composition proof. Execution stops immediately after the helper return,
before the caller's subsequent binding write and tail. Remaining caller/active
response work must retain that boundary and the actual readiness producer.

Nonzero raw link or target state refuses before entry until it has a typed owned
producer; it is never silently cleared. This strict preflight requires the available
parent/affine even when the helper's predicate would skip a later read. It does
not prove native equivalence for absent owners. Actual native caller timing, live
session/linked/hint identities, required action/hint bodies, active response/mode7
continuation, sound/ANM and world admission remain open. The ordinary primary
keeps its refusals; no normal gameplay, new capture or preview follows from this
source-only composition. Prototype PS2 attributions remain unknown.

## Active response cycle - 10 November 2026

`vimp_active_response` acquires the four-stage block after the primary's movement
producer. It turns toward the retained heading, arms and consumes a turn wait,
selects the target bearing, turns again, arms the final delay, then clears response
kind/active/priority on expiry. The stage3 heading producer executes before its
session gate. Required action requests retain distinct heading/finished shapes;
errors preserve preceding stores and callback writes. Exact zero expires a timer,
but a positive wide remainder that rounds to zero does not. The original pending
heading remains distinct from the angle helper's normalized local copy.

A separate native cycle control injects one signal into one actual warm source
child. Native construction, common-list callbacks and real readiness remain;
there is no forced physical/channel gate. It reaches all four stages and the
primary return after response completion without native-controller diagnostics.
The 115 active-block entries comprise one stage0, 64 stage1, 30 stage2 and 20
stage3 visits. Rust matches 1,495 projected state/RNG/control words. A second
comparison retains the Rust response state across all 115 visits, refreshes only
caller position/target/controls and the acquired angular-history copy, and advances
one retained RNG using the supplied external draw schedule. Active outputs are not
replaced with the next native sample. Remaining header/payload fields are checked
against the acquired native write projection.

The comparison begins at a captured active-block entry. Earlier native motion,
clock/control snapshots and external RNG draws remain supplied caller inputs;
this is not Rust whole-primary, physical/ANM or live-world equivalence. Existing
commands can be nonzero at this entry and are retained. The first handled reaction
skips the ordinary movement producer, while later active visits can consume its
commands. A future driver must preserve that distinction and the native phase
lease instead of bypassing the current offline/pending-phase guards.

The 10,000-gap angular helper is shared with the existing Delphi body-alignment
caller without changing that caller's behavior. It also matches 1,117 separate
native scalar controls, including wrapping and negative steps; this is a bounded
angle comparison, not a complete active response. Existing heading/orientation
and bearing services are reused. Prototype PS2 attributions for the primary and
touched dependencies remain unknown; no signatures or layouts are imported.
Five portable controls cover arrival/normalization distinctions, equal-bound RNG,
wide timer decisions, required-service ordering and retained callback errors.

The Session adapter below now retains the stage-produced timer. Next acquire the
remaining ordinary movement producers and compose this block at
its actual callback point before common look, physics and ANM. Session/world phase,
clock/RNG ownership, hint/action/audio services and mode7 remain prerequisites.
No normal gameplay availability, new capture or preview is added by this kernel.

### Mode1 locomotion animation and growth ordering — 10 November 2026

`vimp_animation::mode1_request_with_locomotion` now consumes the actual gait and
inline physical flags. Moving water-contact selection takes precedence over
damage, then the two explicit gait branches and native fallback; each uses its own
scaled reference-speed parameter. Idle turning retains its resource-aware default
wrapper. The numeric moving selector still publishes its request when the
descriptor is absent: the following resource consumer owns that failure.
Already-selected clips and an unfinished entry clip retain the native skip.
The former dry gait0 API delegates to this selector without changing its inputs.

Rust numeric channel composition matches 1,088 original-PC contexts /39,168 words
with blending enabled and disabled, descriptor visibility controls, signed rates,
damage/contact precedence and raw gait fallback. The native source-loaded class
has no idle-turn descriptor; positive numeric-only coverage explicitly aliases
an existing source descriptor into that slot. This synthetic alias establishes
neither an original idle-turn asset nor pose/clock/visual fidelity. Other header
and payload fields remain unchanged in the native controls.

One installed Session control exercises three actual gait/contact animation
resources after an owned physical phase and checks channel mirroring, consumption
and replay refusal. Gait/contact flags are declared post-physical inputs; this
does not acquire a water-contact or response movement producer. ANM consumes only
its own phase: wander stays pending and the complete-frame count stays unchanged.
The current ordinary movement admission still rejects active/water profiles.

Reinspection also identifies the helper before the ordinary movement block as
the already reconstructed growth service, using its target-size transitions,
health parameters, growth timer and scale/reference updates. The first handled
response skips both this growth call and ordinary movement; later visits can run
them. Preserve that branch when composing the primary rather than advancing
growth unconditionally. This is PC role evidence, not a confirmed prototype PS2
name. All touched attributions remain unknown; earlier aliases and the rejected
movement-helper hypothesis remain in the private record. Complete caller phase
ownership, actual movement/contact producers, required services and
complete response/physics/ANM/world composition remain the next work.

### Owned active-response continuation — 10 November 2026

`Session::advance_vimp_active_response_offline` now projects the actual initialized
herd child's heading, angular history, commands, response fields and born delay.
The arrival-produced turn timer persists in `vimp_actor::Cold` as an optional
value. Constructors supply no invented zero; stage1 requires its producer only
after the first predicate admits the read. Unknown is separate from zero expiry.

Prefix stores publish before required session/clock/heading/action services. The
heading callback is followed by a fresh projection even after an error; retired
or changed owners are not restored. Both action shapes are terminal, so their
Session writes and cancellation effects retain ownership without an old final
projection overwriting them. Invocation numbers, reentry and pending phase guards
refuse replay; late errors retain stores/draws and quarantine the owner. ANM and
world frame counters do not advance in this isolated block.

Six installed controls cover persistent arrival/wait state, unknown-timer refusal,
callback mutations/errors, both terminal actions, cancellation and foreign/pending/
reentrant admission. The separate optional captured-cycle test runs 115 native
entries on an actual source child: all 1,495 state/RNG/control words agree while
Session outputs and one RNG persist. Only the first active entry is supplied;
later position, target, commands, angular-history copy, object/binding/context
flags, clock/control inputs and external RNG draw counts remain caller inputs.
The existing acquired heading helper consumes actual source parameters; a private
control supplies the external schedule. This strengthens ownership evidence, not
whole-primary/world equivalence or ordinary mission admission.

The numeric captured and persistent kernel comparisons also still agree. Prototype
PS2 attributions remain unknown. The movement/active composition below now shares
one lease; earlier growth/route decisions and later look/physics/ANM still require
the complete caller. Preserve the first handled response's growth and
movement skip. Actual caller/global clocks and RNG ownership, movement/contact,
hint/action/audio bodies, mode7 and live world/render/re-entry remain required.

### Movement and active response in one lease — 10 November 2026

`vimp_movement` acquires the mode1 scalar/command prefix before active response.
It reuses the existing parameter arithmetic and steering helpers. Parameters
publish before the actor clock, speed updates before all eight command bytes
clear, and the earlier caller's gate decides whether commands are produced.
The obstruction retry path uses a separate global clock and strictly negative
expiry, unlike the active wait's nonpositive expiry. A successful retry stores
the unsigned owner remainder plus two, clears the actor delay and draws a new
interval. Failure retains preceding parameters, turn byte, stores and draws.

Native controls on an actual source-constructed actor match 1,024 contexts /9,216
words across both producer gates, obstruction states, finite gait/damage/scale,
poses, timers and TLS seeds. Original helpers execute; nonprojected header and
payload fields are checked unchanged. A separate source cycle with one signal
injection records 117 prefixes /1,053 words: 116 produce commands, while the
first handled reaction clears commands without producing them. That source
control observes zero obstruction headers; it does not establish their producer.

`Session::advance_vimp_movement_response_offline` now composes this prefix and the
active block under one invocation lease and one RNG. Actual Session fields and
prefixes publish before both movement clocks. The existing active callback,
cancellation, error and replay protections remain. The joint source comparison
matches 115 visits /2,070 projected words with commands, scalar/retry state, active
outputs and RNG retained. Later caller position/target, scale, history/flags,
clock/control, producer gate, obstruction predicate and external draw schedule
remain supplied; original first-entry state is also supplied. This is not a
complete primary or live-world comparison. The earlier active-only control stays
separate and still agrees.

Three portable controls distinguish suppressed production, separate clocks,
strict expiry and retained clock-error prefixes. Two installed joint controls
check first-handled suppression and quarantine without entering/replaying the
active block after a movement-clock failure. The generic configuration initializer
does not establish the missing contact-header constructor value; no implicit
zero obstruction or automatic ordinary admission is added. Touched prototype PS2
attributions remain unknown.

Next connect the earlier growth/route/producer decisions and later look/physics/
ANM in actual Story2 world ownership, then validate visible runtime entry, player
interaction, cancellation and re-entry. Keep required world population, clock/RNG,
obstruction/contact, action/hint/audio and remaining mode providers explicit.

The PC SHA256 is
`690bad39847b0f578163e547e6a6de10ffe9213635a069bd1ce96a75dfd39ead`;
prototype PS2 lookup uses SHA256
`be17fa4b1b5c5120dd685891894c93e65509e55bed59756110f39b0ea48258ef`.
Vector, retarget, marker advance, trigonometry, hint terrain and affine dependency
lookups retain their earlier PC aliases privately. Their prototype PS2 attributions
remain unknown: no raw symbol or demangled signature is assigned, and no ABI or
layout is imported. Follow the central reconstruction method on continuation.

## Source-resource regression

### First callback after a herd birth — 10 November 2026

`Session::advance_vimp_newborn_world` now consumes a real owned herd birth's
first common-list callback. It retains the constructor's position histories,
initializes the actor and continues into living movement/physics/ANM inside
that same callback and shared RNG stream. A separate initialization call or an
extra world-history copy would change that composition. Warm actors continue
through the separately prepared or combined routes below.

This entry requires a cold, active, unparented class8 herd child at local frame
zero. The caller must establish that this birth followed the current world-history
prepass; cold metadata alone does not establish that ordering.
Parentless allocations, already initialized actors, wrong/foreign frames
and route replay refuse. Initialization and completed physical work survive a
later required effect-namespace failure; the failed world token forbids replay.
The entry does not fabricate sound/effect providers or bypass outer world gates.

Original-PC constructor/birth/first-primary controls match 32 scenarios and 96
snapshots / 824,168 normalized words, including 448 first callbacks. The declared
Story2 cohort uses authored markers and original/supplied kind1 parent selectors,
16 TLS seeds each, positive supplied cadence, quiet noise, flat terrain and a
supplied nonnull control reference. Normalized headers, payloads, references,
ordered lists, prepared effect contexts and RNG are compared. Later two callbacks
use the existing per-node history boundary. This is neither a complete world
traversal nor live process-seed, sound transport or rendered fidelity evidence.

Two installed-source controls additionally cover a 14-child cohort through its
first callback and two separately prepared continuations, constructor-history
preservation, cold/initialized route separation, inactive/foreign/parentless
refusal and retained late failure without repeated draws or services. These
controlled source checks do not establish complete world admission or native
error handling. All detailed controls remain private; prototype PS2
correspondence is unassigned.

The herd/marker Rust common-node dispatcher, outer world admission, clock/RNG
ownership and active FX/audio still need binding before ordinary mission AI.
Remaining tasks stay in `BACKLOG.md`.

### Owned herd/marker common callbacks — 10 November 2026

`Session::prepare_herd_world_history` and
`Session::advance_herd_world_after_history` now compose acquired class49/state17
herds and class679/state0 markers with the common node policy. Source construction
owns their nonphysical motion/finalization state. The entry retains primary
rate history, the acquired absent state0 primary and state0/state17 secondary
callbacks, object-local time, empty-channel ANM epoch and finalization. The
herd's birth clock counts primary calls separately from common frame numbers.
Required environment/footprint providers are explicit; no terrain is fabricated.
Empty channels and references are a checked profile, not a general ANM adapter.

A composed Rust `Session::walk_common` control now matches the original mutable
walk across 32 declared Story2 scenarios /96 snapshots /824,168 normalized words.
It includes 2,880 node visits, 2,432 prior history preparations and 448 newborn
first callbacks in their birth steps. It compares owned headers, payloads,
references, ordered lists, birth-height requests, prepared contexts and RNG.
The global ANM epoch is shared with Vimps, not restarted per marker. Separate
original ANM controls match 128 cases /4,864 words, covering signed zero,
positive time, force masks and epoch wrap with four empty channels.

Five installed controls additionally cover a three-step 30-node traversal,
constructor histories, current affine publication, cancellation, wrong/foreign
entry and invalid-clock preflight. A supplied sleep/wake gate additionally
keeps skipped primaries separate from common frame counts. A late environment
failure retains the fourteen births, preceding ANM epoch and partial environment write; actor and
list tokens prevent replay. Loss of the controlled actor stops after birth
without acknowledging fallback; callers must stop the whole walk on that
outcome. These fault controls establish owned failure
handling, not native exception equivalence. The added coupled environment
control uses the real terrain/mesh composition and fails at its required platform
provider: floor/plane writes, births and the ANM epoch survive into the failed
owned frame, whose token prevents replay.

Cohort/control identity, clock, quiet noise, flat terrain, empty collision and
platform cohorts, TLS seed and absent sound bank remain supplied. This binds
bounded Rust composition, not outer world/global services, native live RNG
ownership, ordinary mission AI, active effects/audio or rendering. The next work
is the actual admission/phase producers and required behavioral services in
`BACKLOG.md`. Prototype PS2 counterparts remain unassigned.

### Mutable common-list acquisition — 10 November 2026

A separate original-PC-only control now executes the actual mutable callback
walk, after a supplied active-node history pass. Across 32 Story2 scenarios and
three steps each, it records 2,880 node visits and 2,432 history preparations.
All 448 newborn Vimps are visited in their birth step without an earlier history
copy. The successor is read after the callback; a snapshot of the pre-birth
population would omit these visits.

Herd parents and basic markers also execute their common secondary dispatch,
local time, ANM and finalization paths. The acquired PC state0 primary and
secondary entries are absent; state17 has the acquired herd primary and no
secondary entry. That establishes these bounded dispatch absences, not permission
to skip the common fallback. Source-marker controls still write their floor,
animation epoch and affine cache. Herd finalization additionally calls the real
environment service. The bounded Rust binding above now retains these dependencies; ordinary
Ripper-only finalization is not an interchangeable adapter.

This acquisition retains the declared cohort, nonnull controlled herd reference,
TLS seed, cadence, quiet noise, flat world and absent sound bank. It excludes the
intervening global services, outer admission, actual player and rendering. Its
96 recorded snapshots are native observations, **not a Rust comparison**. Keep
them separate from the validated newborn comparisons above. Prototype PS2
wrapper/dispatch lookups remain unassigned; no ABI or types are imported.

### Separate Vimp world-history entry — 10 November 2026

The normal PC world path prepares active actors' position histories before its
primary callback traversal. The common traversal reads each successor after
the callback. An actor born later in that traversal must not be assumed to have
received the earlier history pass. Outer world gates and intervening global
services remain required; recovering this ordering does not implement them.

`Session::prepare_vimp_world_history` now seals an initialized, active owned
Vimp's history for its current local frame. It advances no clock, AI, physics,
animation or audio. `advance_vimp_world_after_history` consumes that preparation
without copying the history again. Wrong/foreign frame identities, inactive
preparation, repeated preparation and mixing the split and combined routes are
refused. A missing preflight service keeps the pending history; an attempted
callback consumes it and retains its failed iteration against replay. Existing
`advance_vimp_world_offline` remains the combined inspection route.

The pure position-history operation is shared with ordinary actor finalization.
Actual original-PC execution of the helper and its real return tail matches
256 supplied header cases / 2,304 position/history words in Rust, including
signed zero, infinities and NaN payloads. The native control separately checks
49,408 header words for preservation outside the written history fields; that
is not a complete-header Rust comparison. These are instruction-level controls,
without world startup, live clock/RNG or callbacks.

Three installed-resource controls cover two separately prepared actors, a
position change between phases, a late terrain failure with retained history,
replay/foreign/inactive refusal and a missing required recovery-sound provider.
An eight-frame source-backed prefix matches the existing combined route for
position histories, channels, configuration, payload, RNG and ANM epoch using
supplied flat terrain, time and seed. It is a Rust composition regression, not
an original-PC comparison of eight complete frames. The controlled actor
identity is supplied; no complete world or ordinary mission is admitted.

The newborn route above supplies the separately acquired first-callback
composition. Actual world admission, clock/RNG ownership, the complete cohort
and required threat/feeding/effect/audio services remain unbound. The next tasks
are retained in `BACKLOG.md`. Prototype PS2 attribution remains unassigned.

### Authored herd births

The optional `source_npc_herd_births_keep_ownership_and_quarantine_failures` test
loads authored world herd/marker placements from a user-supplied installation.
It keeps their values and order, composes source Vimp construction and first
initialization, checks parent/session identity, repeated-initialization refusal,
terrain-failure quarantine, non-replayed RNG/service calls and Session cleanup.
The flat floor, timestep and random seed are supplied test inputs. It executes
no original program and proves neither full living AI nor visual fidelity.
See [BUILD.md](BUILD.md) for its command and required local configuration.

The first installed-data run examines 28 authored parents, initializes 313
children and checks 26 injected terrain failures. Two parents become inactive
without eligible markers in this world-only fixture; they must not be scheduled
again as active herds in that fixture. Mission markers are not loaded here.
No other configured-parent profile was excluded in this run. Two source world
files are refused by the current parser and remain outside this control's
coverage. These counts depend on the installation and do not certify all worlds,
native whole-frame behavior or the original executable's current RNG stream.

Runtime integration must retain real player-target identities, common physical
and ANM ordering, actual input/contact/damage producers, resource ownership and
cancellation/re-entry. A rendered diagnostic animal is insufficient to establish
interactive NPC behavior. Future handoffs must retain these boundaries and the
next required native-name lookup.
