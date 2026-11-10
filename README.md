# giants-rs

Experimental independent Rust + Bevy reconstruction of Giants: Citizen Kabuto.
The goal is the complete game. The current menu-first Windows preview is incomplete.

The owner estimates approximately **99% of the current project content is AI-generated**.
This is an estimate, not a measured authorship statistic or a completion percentage.
There has been no comprehensive human review. Automated tests do not establish
gameplay fidelity, security, authorship or clearance of third-party rights.

Unofficial project, unaffiliated with the original developers or rights holders.
Supply your own legally obtained PC game installation. No original game assets,
binaries or extracted game data are bundled. The historical screenshot gallery
described below is published under the owner's explicit authorization.

[Quick start](docs/QUICK_START.md) · [Build](docs/BUILD.md) · [Release policy](docs/RELEASES.md)

The [main integration process](docs/INTEGRATION.md) connects the cinematic and
character work to the shared application and validated Windows previews. CPU
services, diagnostic captures and ordinary gameplay availability are tracked
separately; a source merge alone does not make a feature playable.

The [roadmap dashboard](https://yyoudi.github.io/giants-rs/) is generated from this
repository's `PROGRESS.json` and updates through GitHub Pages on main-branch pushes.
It separates the [published Windows build snapshot](https://yyoudi.github.io/giants-rs/#current-preview)
from searchable source-status tables and the owner-authorized historical comparison
gallery. Captures are labeled development evidence, not certification
of the current public build; game artwork remains outside the project MIT license.
[Automatic Windows build checks](https://github.com/YYoudi/giants-rs/actions) are
active. See [automation and activation](docs/AUTOMATION.md)
and the [publication review status](docs/PUBLICATION_REVIEW.md).

Native-function research now systematically checks names from the **prototype PS2**
before analysis or implementation. The [reconstruction method](docs/RECONSTRUCTION_METHOD.md)
separates literal ELF symbols, uncertain demangling, proposed PC correspondences
and independently reviewed attributions. Private metadata stays outside the
repository. Recovered names do not measure implementation or gameplay fidelity.

Project source is under the [MIT license](LICENSE), within the scope described in
[NOTICE.md](NOTICE.md). Public source publication follows the owner's approval and
an AI-assisted technical review; it is not legal clearance or comprehensive human
review. Download the experimental [Windows preview](https://github.com/YYoudi/giants-rs/releases)
and follow the [quick start](docs/QUICK_START.md). The ZIP includes the executable,
launcher and notices. Place `giants-rs.exe` beside the original `Giants.exe` and
double-click it to open the menu; no command line is required. From another folder,
it offers an installation picker. The default Preview All profile
unlocks mission selection. Your own game installation is required. Linux and macOS
downloads are planned for later. See the [distribution review](docs/DISTRIBUTION_REVIEW.md).

## Whole-game roadmap

![Whole-game feature-family status](docs/roadmap.svg)

Generated from `PROGRESS.json` by `python -m tools.update_roadmap`.
CPU acquisition, runtime integration and fidelity are separate. Partial does not
mean a complete playable feature. No global completion percentage is claimed.

<!-- progress:start -->

### Campaigns and progression

- [ ] **Complete Mecc campaign** — partial; Runtime integration. All missions, allies, scripted sequences and chapter completion.
- [ ] **Complete Delphi campaign** — todo; Runtime integration. All missions and character-specific objectives and encounters.
- [ ] **Complete Kabuto campaign** — todo; Runtime integration. All missions, growth stages, encounters and ending.
- [ ] **Mission objectives and scripts** — partial; Runtime integration. Triggers, dialogue events, timers, escort/defence tasks, victory, failure and restart.
- [ ] **Chapter and mission continuity** — partial; Runtime integration. Unlocks, entry/exit transitions and transfer of the required player state.
- [ ] **Complete cinematic playback** — partial; Runtime integration. 545 sources /16 worlds recounted on preview-2026.11.10.2: 59 reported gaps, 56 general-FX refusals, 282 strict-profile reviews; zero complete scenes verified. Main film renders bounded persistent attachments across 23 sources /25 requests. Attached/detached timed actors share Session fields, payload, channels and references, retaining completed writes after late failures. Retained ownership controls match 576 states. New CPU comparison: common finalization/environment matches 1,152 states, including 640 detached, and 902 ground queries across all sixteen static source terrains. Entry physical/header/history/affine and caller-slot inputs remain supplied; mesh/platform population is empty. The published build passes silent cancellation/re-entry, muted voice/text cleanup and package verification. Manual first-opening natural end, menu return, second entry, double-click skip and partial Mecc entry pass on one profile. Actual timed physical/header ownership, mission population, stock, native RNG/clock/world scheduling and main-film admission remain open. Complete visual/audio/timing fidelity, native gameplay and broad manual coverage remain unverified.
- [ ] **Ending and credits flow** — todo; Runtime integration. Final sequence, credits presentation and return to menus.

### Meccs

- [ ] **Mecc movement and jetpack** — partial; Runtime integration. Walking, running, jumping, jet fuel/recharge, nitro, carried weight, terrain contact and water hazards. The living owned selector now admits negative absolute heights using the acquired floor-relative gap. Native/Rust selector controls pass 144 cases; the full low-floor primary comparison matches 40 of 128 frames before refusing the next object-flag profile. The unequipped jet now continues through zero fuel after 288 native branch controls and 1,536 jet-prefix comparisons. The actual muted-sink inspection reaches step 441 before refusing an unacquired impact reaction; equipped depletion services, long-cycle validation and complete environment/health behavior remain open. Post-impact/directional-air selector continuation matches 1,680 native contexts / 560,640 selected words, with 816 comparisons ending before the weapon-commit helper. A further 48 idle-draw cases explicitly require the missing animation callback. Damage, flat floor, positive idle timer and disabled blending are supplied; wounded full-world admission remains unverified. Full-primary discovery exposes required damage-smoke preparation. Its separate policy matches 2,080 contexts / 18,780 words and real prepared-context controls match 256 contexts / 16,640 words. The bounded primary now owns persistent smoke with a source/frame-bound display snapshot: 384 native/Rust frames / 124,800 normalized header/payload/VM words agree. A real owned impact also continues into the next wounded frame without replaying damage or supplied cues. Prepared cancellation and failed-frame quarantine are checked. App gate production, active smoke and full-world admission remain open.
- [ ] **Mecc squad and orders** — todo; Runtime integration. Following, moving to a location, attacking, jet use, incapacitation, recovery and repair.
- [ ] **Complete Mecc weapon arsenal** — partial; Runtime integration. Handgun, RPG, machine gun, proximity missile, sniper gun, missile launcher, homing missile and Millennium Mortar need complete aiming/firing rules. Normal-pistol CPU services compose owned primary, held cache, subjective animation/camera, targeting, projectiles and effects. Completed subjective positions now use the shared model bank; two 32-frame controls also validate backward-air continuation. Main mission owned firing remains disabled. Live slot/part/material and subjective drawing admission, complete target contexts and positional audio remain open. See docs/OWNED_LOCAL_FIRE.md.
- [ ] **Mecc inventory and ammunition** — partial; Runtime integration. Carry limits, weapon selection, ammunition, reload/cadence, pickups, drops and resupply.
- [ ] **Mecc packs and upgrades** — partial; Runtime integration. Jetpack upgrade, repair, shield and bush camouflage, with their energy costs and restrictions.
- [ ] **Mecc items and deployables** — todo; Runtime integration. Mines, grenades, health syringes, flares, deployable turrets and pop-up bombs.
- [ ] **Gyrocopter** — todo; Runtime integration. Access, piloting, passengers, machine gun, bombs, landing, repair, resupply and destruction.
- [ ] **Mecc carrying and interaction rules** — partial; Runtime integration. Carry/rescue Smarties, collect meat and use shops, platforms, vehicles and mission objects.

### Sea Reapers / Delphi

- [ ] **Delphi movement and water** — todo; Runtime integration. Live Delphi remains unavailable. Owned CPU services cover construction, dry movement/jump, camera, reserve/items, held caches, terrain/target geometry and linked readiness. The Delphi writer feeding bounds-centre selection now matches 513 original-PC contexts. A separate control over 128 supplied four-actor schedules matches 640 production/query output words, with Rust calculating the scalar rather than replaying native results. Current mode/link/context gates, source interval and precision boundaries are preserved. Physical support now feeds the writer through a required header resolver: 320 original-PC/Rust schedules match 3,840 exact control/position/scalar words, with floor/plane error below 0.00000012. Unknown nonzero scene keys fail. A further 512-context control executes the original list walk/radius/bounds helpers, matching 4,608 control/radius/scalar words and 910 ordered contacts. List membership and source geometry/headers remain supplied. The support-class packed attribute now matches 512 original dispatch/copy programs /31,744 projected words. An explicit cold factory now owns the support class in the common Session: eight original constructors match 1,584 normalized header/payload words and 32 registrations; Rust repeats with three source mission catalogs. Private payloads, shared models, failed-construction rollback and clear/re-entry pass. Ordinary construction and current geometry remain refused. First-update initialization, full native retirement, scene-key ownership and phase-correct header publication remain open. Traversal, prepared geometry/model/part eligibility, headers, positions, viewport and schedule remain supplied; collision-then-scalar does not prove same-frame phase order. Earlier selector/guard/range and readiness controls retain their declared scopes. Next: actual scene-key ownership/lifetime and phase-correct header publication, other actor writers, target/proxy lifetime and primary/world/child-ANM timing. Camera/input/inventory transitions, active physics/audio, menu/renderer/HUD/cleanup, water/swim/turbo and combat remain open. CPU agreement does not establish gameplay or fidelity; older executable/package receipts retain their original sources. Details: docs/OWNED_DELPHI.md and BACKLOG.md.
- [ ] **Sword combat and combinations** — todo; Runtime integration. Melee attacks, combinations, targeting, contact and damage.
- [ ] **Complete Sea Reaper bow arsenal** — todo; Runtime integration. Regular, Lightning, Hellfire, Screamer and Power-Up bows; charging, lock-on and multiple targets.
- [ ] **Offensive spells** — todo; Runtime integration. Cluster, Cluster Mine, Firewall, Fire Circle, Hail and Tornado, including their targeting and duration rules.
- [ ] **Time and target-control spells** — todo; Runtime integration. Slow Time, Shrink and Follow; area effects, exclusions and restoration.
- [ ] **Utility spells and spell inventory** — todo; Runtime integration. Smartie Grab/Bubble, Cloak, Teleport and Repair; spell slots, charges and replenishment.
- [ ] **Reaper Ski** — todo; Runtime integration. Mounting, water/land handling, turbo and missile pickups, vehicle health and destruction.

### Kabuto

- [ ] **Kabuto movement and growth** — todo; Runtime integration. Continuous live movement, jumping, feeding, growth-threshold production and size-dependent live scheduling remain pending. Factory/Session CPU phases now continue from post-ANM/gravity forces through the acquired guard and resource/charge/next-speed-latch producer; 512 source-backed contexts match 1,317,724 words including clock and service-prefix publication. Prior resource/latch state, commands and world/service entry contexts are supplied; full primary/action/lifecycle completion and actual world/ANM/camera/GPU cadence still block live admission. Earlier cold/growth/motion/contact receipts retain their separate scopes in OWNED_KABUTO.md. No playable or visual-fidelity claim is added.
- [ ] **Grabbing, storing and eating creatures** — todo; Runtime integration. Grab creatures, store them on horns, eat them and restore health.
- [ ] **Kabuto attacks and adrenaline** — todo; Runtime integration. Hand and foot attacks, adrenaline levels, combinations and area damage.
- [ ] **Throwing and environmental attacks** — todo; Runtime integration. Throw creatures and boulders; collisions, fragments, damage and replenishing rock columns.
- [ ] **Kabuto offspring** — todo; Runtime integration. Lay eggs, hatch and grow offspring; command them to defend, attack, eat or fetch creatures.
- [ ] **Kabuto weak spot and gemstone attack** — todo; Runtime integration. Weak-spot exposure, lethal hits and the controllable gemstone beam.

### Bases, Smarties and resources

- [ ] **Base-building interface** — todo; Runtime integration. Access platforms, overhead camera, placement, build choices, prerequisites, upgrades and indicators.
- [ ] **Smarties and worker behaviour** — partial; Runtime integration. Capture/rescue, faction ownership, transport, work orders, hunger, rest, departure and hostile variants.
- [ ] **Resource economy** — partial; Runtime integration. Vimp meat and souls, expiry, carrying limits, deposits, consumption and worker upkeep.
- [ ] **Mecc base development** — todo; Runtime integration. Pit/pub, build platform, workshop/generator, wood/stone walls and command centre.
- [ ] **Mecc shops and advanced projects** — todo; Runtime integration. Gift-shop tiers, party house, research/access requirements, healing and equipment supply.
- [ ] **Sea Reaper base development** — todo; Runtime integration. Persuader, build platform, spell school/shield generator, spell shops, force fields, command centre and party house.
- [ ] **Base defence, repair and destruction** — todo; Runtime integration. Bulldog and Hellfire turrets, SAM and Screamer SAM; targeting, player control, shields, building damage, repairs and loss of services.
- [ ] **Remote structures and faction projects** — todo; Runtime integration. Tower and cloaked tower, mini-shop and mini-spell shop, mine shaft and portal, gyrocopter pad and pier; access, travel and services.
- [ ] **Sea Monster attack** — todo; Runtime integration. Summoning cinematic, countdown, targeted fireball and resulting damage.

### Actors and shared combat

- [ ] **Complete mission actor population** — partial; Runtime integration. Spawn, activation, update order, attachments, lifecycle and removal for the required actors.
- [ ] **Enemy and boss behaviour** — partial; Runtime integration. Detection, navigation, pursuit, attacks, reactions, faction relationships and scripted boss encounters. Existing Ripper perception, primary/attack and awakening/emergence services plus shared enemy response/projectile kernels are inventoried in docs/OWNED_NPCS.md. Complete live actor/world scheduling, real firing/effect/audio providers and remaining enemy/boss profiles remain open; this inventory enables no new ordinary gameplay.
- [ ] **Wildlife and non-player inhabitants** — partial; Runtime integration. Vimps, herds and other creatures: movement, feeding, fleeing, interactions and mission roles. Ordinary mission AI remains unavailable. Owned services cover herd births, separate warm history and newborn initialization plus living/physical/ANM continuation in one callback, preserving constructor history and late failure. Original-PC controls match 32 scenarios / 96 snapshots / 824,168 normalized words, including 448 first callbacks, within supplied Story2 cohort, parent-selector, time/terrain/control/RNG boundaries. Full herd/marker dispatch, outer world and clock/RNG ownership, required threat/feeding/active effects/audio, rendering and re-entry remain open. See docs/OWNED_NPCS.md. A separate original-PC-only mutable walk records 2,880 callbacks and 448 birth-step first callbacks; herd/marker fallback and population traversal still need Rust binding.
- [ ] **Complete projectile and damage rules** — partial; Runtime integration. Direct/explosive/guided attacks, impacts, hit ownership, shields, friendly fire and environmental reactions. On 5 November 2026, the preview pistol object query was bounded by its terrain stop, matching the existing owned-flight ordering. Synthetic regressions distinguish objects before and behind terrain. Silent installed-data development replays pass declared-obstacle hits, an empty-obstacle control and synthetic-target damage/reset; owned actor damage and native trajectory fidelity remain unverified. The separate normal Mecc impact CPU path now applies actual Session health before contact precedence and positional preparation. Native controls match 628 complete selectors / 209,615 words and 64 pain-entry prefixes / 21,056 words. A supplied falling-entry primary retains health at the required missing voice service without RNG replay. The acquired voice tail now matches 136 native contexts / 136 cue entries / 1,224 selected words, including health-ratio and cue-choice boundaries. An optional primary provider uses the actual Session countdown: supplied cues complete one falling-entry frame; either cue failure retains damage and prevents replay. Local transport now independently matches 960 native/Rust caller cases / 16,968 selected words. Four actual muted damage-cue starts and one jet start preserve separate RNG owners; two unique-reuse calls draw nothing. Typed root/refusal, late sink loss and asset cleanup are checked. The App still supplies no impact/voice provider. The bounded primary now continues supplied wounds and the frame after an owned impact with required display snapshots. App mission gates, broader routing/lifecycle and full trajectory remain open.
- [ ] **Injury, death and respawning** — partial; Runtime integration. Reactions, incapacitation, corpses/drops, recovery, player death and mode-specific respawn rules.

### Interface, worlds and audio

- [ ] **Complete game menus and profiles** — partial; Runtime integration. Start, mission selection, loading, pause, inventory/shop screens, errors and returns to menus. Desktop entry detects adjacent game data, with a folder picker fallback, a Preview tools overlay and default independent unlocked mission catalog.
- [ ] **Controls and settings** — partial; Runtime integration. Keyboard/mouse bindings, sensitivity, display and audio settings, persistence and focus handling.
- [ ] **All gameplay camera modes** — partial; Runtime integration. First/third person, aiming/zoom, vehicles, base building and special attacks remain partial. Owned mission inspection consumes a Session-bound outer view with terrain adjustment and prepared-mesh clipping. Controlled subjective post now produces the inner camera before placement/fire, with camera invocation counts separate from world frames. Four controls match 320 frames/12 shots and both camera caches; another 64-frame control checks inner view matrices. Full world/list admission, smoothing, special modes and main-driver subjective drawing/fire remain open. Native owner selection has 162 prefix cases. Shared Rust mode commands match 1,280 original contexts; eligibility matches 864 pre-world vectors with supplied current globals/resources. Session pre-world preparation now consumes that policy and returns actual display activity. Camera startup/override producers, world ordering and App binding remain open. See docs/OWNED_CAMERA.md.
- [ ] **Character HUDs, maps and objectives** — partial; Runtime integration. Health, fuel, magic, adrenaline, weapons, items, resources, targeting, map, notifications and score.
- [ ] **Complete world and actor rendering** — partial; Runtime integration. Owned mission inspection now uploads camera-selected body models from the Session shared pose bank. Native controls match 1,280 selections and 256 complete model snapshots; six recorded Bevy endpoints cover automatic, forced and absent models. Full world visibility, part/material admission, subjective drawing, lighting, scenery and fidelity remain incomplete. See docs/OWNED_MODEL_DRAW.md.
- [ ] **Water and environmental presentation** — partial; Runtime integration. Waves, reflections, sky, fog, vegetation and environment interactions.
- [ ] **Animations and active effects** — partial; Runtime integration. Character/equipment animation, attachments, particles, trails, explosions, magic and effect lifecycles. Owned positional-impact birth and timed callbacks have complete normalized native state controls. Actor-local core retirement follows required tracked release, with 192 native retirements/reuses preserving a live peer: 53,184 release-entry state words and 2,880 ownership-protocol words match. Late release/reference failures retain ownership without replay. Those controls disable native audio and supply local time. A separate 20-sequence native TLS/helper control supports distinct effect-VM and audio-module RNG owners; frequency variation must not be redirected into the VM stream. Actual muted Bevy impact transport now starts ready positional sinks with separate audio RNG and owner-specific stop/release. Native parameter controls match 32 cases / 160 words and two actual starts / ten words; source controls retain late sink failures and reject queued-only playback without audio draws. Inspection reservation is wired, but main impact reaction binding, enclosing cleanup prepass, other profiles, audible fidelity and full world scheduling remain open.
- [ ] **Music, voices and gameplay audio** — partial; Runtime integration. Music, dialogue/subtitles, footsteps, weapons, spells, ambience, spatial mixing and stopping/releasing sounds.
- [ ] **Saving and restoring a game** — todo; Runtime integration. Restore progression and the required mission/player state reliably, including restart and profile selection.

### Multiplayer

- [ ] **Networked game simulation** — todo; Runtime integration. Hosting/joining, state replication, ownership, latency, disconnects and game transitions.
- [ ] **Lobby, teams and communication** — todo; Runtime integration. Map/race/team choice, match options, readiness, chat, scores and server controls.
- [ ] **Deathmatch and faction matchups** — todo; Runtime integration. Mecc/Reaper/Kabuto combinations, with or without full bases, scoring, friendly fire and respawning.
- [ ] **Capture the Smartie** — todo; Runtime integration. Capture, carrying, stealing, returning, scoring and victory, including the base-building variant.
- [ ] **Multiplayer base-building rules** — todo; Runtime integration. Resource competition, faction construction, victory options and timed protection/detente rules.

### Whole-game validation

- [ ] **Complete content coverage** — partial; Runtime integration. Audit every required mission, actor, weapon, spell, building, vehicle and presentation variant.
- [ ] **Text and language coverage** — todo; Runtime integration. Menus, objectives, subtitles, fonts and available dialogue/audio variants.
- [ ] **Playable Windows build stability** — partial; Runtime integration. Long sessions, repeated loading/restart, complete resource reclamation, performance and supported rendering paths remain incomplete. Published preview-2026.11.10.1 makes per-emitter ambient step diagnostics opt-in while retaining source/voice lifecycle and error logs. Its exact executable passes six offscreen development launches and two selected cinematic end/re-entry cycles. One local muted manual profile covers the menu, opening skip, ordinary Mecc entry, return and second entry; held native jet/descent/landing and broad campaign/fidelity checks remain open. The dashboard pins the download separately from later source-only services.
- [ ] **Complete gameplay and presentation fidelity** — partial; Fidelity. Compare complete scenes and mission outcomes, timing and behaviour across all three characters and game modes.

### Technical foundations

- [x] **Reproducible Windows MinGW build** — done; Build. 
- [x] **Test-build menus and mission selection** — done; Runtime. 
- [x] **Controlled normal Mecc cycle, orientation and complete payload** — done; CPU only. 
- [ ] **Controlled Kabuto construction, growth and physical subphases** — partial; CPU only; live admission pending. Owned cold construction, growth preparation and numbered motion/contact/model/ANM/gravity/control phases reach the Factory/Session resource stage (512 contexts / 1,317,724 words). Pre-query reference release now matches 512 / 1,331,545 normalized header/payload/service words, including 128 original state18 route/reset compositions under a declared animation service. Query/filter/retention independently matches 512 / 13,248 words. Common attachment requests match 512 / 389,023 words with a read-only external rule catalog; reset control/response/behavior roles are separated. Current/held-reference handoffs match 512 / 1,076,651 words under required entry providers; target detachment is shared. Shared matrix, normalization, clock, impulse and ownership services are reused. Session parent removal now retains supplied binding indices and behavior; unknown constructor words are not fabricated. Actual Session current/held-reference ownership, caller policy, physical/global ownership, constructor/animation/transport backends and complete lifetimes remain unbound. All initializer/history prerequisites remain; ordinary menu-to-mission Kabuto, movement, feeding, growth, world/camera/GPU scheduling and fidelity are not validated. Detailed controls and boundaries remain in OWNED_KABUTO.md.
- [x] **Controlled herd and Vimp kernels** — done; CPU only. 
- [x] **Controlled actor callback policy and acquired owned adapters** — done; CPU only. 
- [x] **Approved reference and Bevy comparison gallery** — done; Historical public gallery. Owner-authorized original / Bevy comparisons are available in the dashboard; historical captures do not certify the current build or full-game fidelity.
- [x] **Controlled offline normal Mecc header and primary contract** — done; CPU only. 

### Research and validation

- [ ] **Reviewed prototype PS2 function attribution** — partial; Research method; separate from runtime and fidelity. Mandatory name lookup with pinned PC/prototype identities, literal symbols, uncertain demangling, retained aliases and independent evidence. Initial read-only audit verifies 4,769 function symbols; six reviewed services include three confirmed identities, two probable candidates and one unassigned. Seven current primary/subjective lookups remain explicitly unassigned, with retained PC aliases and owned Rust relationships. Broader mapping and type recovery remain incomplete. Method: docs/RECONSTRUCTION_METHOD.md.
<!-- progress:end -->
