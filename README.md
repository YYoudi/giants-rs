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
It opens with featured engine screenshots and includes searchable status tables
and 302 owner-authorized historical screenshot
comparisons. Gallery captures are labeled development evidence, not certification
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
- [ ] **Complete cinematic playback** — partial; Runtime integration. 545 sources /16 worlds inventoried; zero complete scenes verified. The last full census (preview 6) retains 59 gap sources, 56 general-FX queue refusals and 282 strict-profile reviews. Main film: bounded persistent attached FX across 23 sources /25 requests, current source-driven channel/matrix, camera/range and both draw routes. Preview tools expose four openings and 19 completion sequences. Published preview-2026.11.09.11 (source cf12894) passes both silent offscreen development endpoints, three active-effect cancellations and two source-terminal/re-entry cycles with voice/text cleanup; downloaded bundle verified. Bounded timed parent/gravity-child ownership matches 576 original-PC callbacks /100,224 selected words, including zero deltas, one-time context consumption and late-failure quarantine. Its new timed owner is not yet admitted by the main film: ordinary preparation, physical/context/stock providers and world RNG/cadence remain supplied boundaries. Full visual/audio/timing fidelity, gameplay transitions and broad manual verification remain open.
- [ ] **Ending and credits flow** — todo; Runtime integration. Final sequence, credits presentation and return to menus.

### Meccs

- [ ] **Mecc movement and jetpack** — partial; Runtime integration. Walking, running, jumping, jet fuel/recharge, nitro, carried weight, terrain contact and water hazards. The living owned selector now admits negative absolute heights using the acquired floor-relative gap. Native/Rust selector controls pass 144 cases; the full low-floor primary comparison matches 40 of 128 frames before refusing the next object-flag profile. The unequipped jet now continues through zero fuel after 288 native branch controls and 1,536 jet-prefix comparisons. The actual muted-sink inspection reaches step 441 before refusing an unacquired impact reaction; equipped depletion services, long-cycle validation and complete environment/health behavior remain open.
- [ ] **Mecc squad and orders** — todo; Runtime integration. Following, moving to a location, attacking, jet use, incapacitation, recovery and repair.
- [ ] **Complete Mecc weapon arsenal** — partial; Runtime integration. Handgun, RPG, machine gun, proximity missile, sniper gun, missile launcher, homing missile and Millennium Mortar need complete aiming/firing rules. Normal-pistol CPU services compose owned primary, held cache, subjective animation/camera, targeting, projectiles and effects. Completed subjective positions now use the shared model bank; two 32-frame controls also validate backward-air continuation. Main mission owned firing remains disabled. Live slot/part/material and subjective drawing admission, complete target contexts and positional audio remain open. See docs/OWNED_LOCAL_FIRE.md.
- [ ] **Mecc inventory and ammunition** — partial; Runtime integration. Carry limits, weapon selection, ammunition, reload/cadence, pickups, drops and resupply.
- [ ] **Mecc packs and upgrades** — partial; Runtime integration. Jetpack upgrade, repair, shield and bush camouflage, with their energy costs and restrictions.
- [ ] **Mecc items and deployables** — todo; Runtime integration. Mines, grenades, health syringes, flares, deployable turrets and pop-up bombs.
- [ ] **Gyrocopter** — todo; Runtime integration. Access, piloting, passengers, machine gun, bombs, landing, repair, resupply and destruction.
- [ ] **Mecc carrying and interaction rules** — partial; Runtime integration. Carry/rescue Smarties, collect meat and use shops, platforms, vehicles and mission objects.

### Sea Reapers / Delphi

- [ ] **Delphi movement and water** — todo; Runtime integration. Live Delphi remains unavailable. Owned CPU services cover construction, dry movement/jump, camera, reserve/items, held caches, terrain/target geometry and linked readiness. The separate viewport/current-bounds selector now matches 768 complete original-PC searches, preserving perpendicular ranking, first-tie ordering, strict limits and wide/stored precision boundaries. Its range-mode guard matches 1,024 calls; both caller parameter prefixes match 130 executions with actual source globals and finite variants. Candidate headers, ordered population, current bounds and viewport remain supplied. The same search feeds two distinct caller relationships; their publication must remain separate. Next: acquire current-header production and bind owned pending/active target lifetime, proxy updates/retirement and primary/world/child-ANM order. Camera/viewport and inventory transitions, active physics/audio, menu/renderer/HUD/cleanup, water/swim/turbo and combat remain open. CPU agreement does not establish gameplay or fidelity; older executable/package receipts retain their original sources. Detailed controls and remaining prerequisites: docs/OWNED_DELPHI.md and BACKLOG.md.
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
- [ ] **Wildlife and non-player inhabitants** — partial; Runtime integration. Vimps, herds and other creatures: movement, feeding, fleeing, interactions and mission roles. Existing source-backed herd/Vimp services now have a retained authored-resource ownership regression: 28 parents, 313 initialized children and 26 injected terrain failures on the tested installation. Two parents become inactive without eligible world-only markers; mission markers are outside this fixture and two world files are parser-refused. Flat floor, clock and RNG are supplied. Ordinary live AI, player threat/feeding responses, world scheduling, rendering and audio remain open. See docs/OWNED_NPCS.md.
- [ ] **Complete projectile and damage rules** — partial; Runtime integration. Direct/explosive/guided attacks, impacts, hit ownership, shields, friendly fire and environmental reactions. On 5 November 2026, the preview pistol object query was bounded by its terrain stop, matching the existing owned-flight ordering. Synthetic regressions distinguish objects before and behind terrain. Silent installed-data development replays pass declared-obstacle hits, an empty-obstacle control and synthetic-target damage/reset; owned actor damage and native trajectory fidelity remain unverified. The separate normal Mecc impact CPU path now applies actual Session health before contact precedence and positional preparation. Native controls match 628 complete selectors / 209,615 words and 64 pain-entry prefixes / 21,056 words. A supplied falling-entry primary retains health at the required missing voice service without RNG replay. Main impact binding, pain/ambient transport, wounded continuation and full trajectory remain open.
- [ ] **Injury, death and respawning** — partial; Runtime integration. Reactions, incapacitation, corpses/drops, recovery, player death and mode-specific respawn rules.

### Interface, worlds and audio

- [ ] **Complete game menus and profiles** — partial; Runtime integration. Start, mission selection, loading, pause, inventory/shop screens, errors and returns to menus. Desktop entry detects adjacent game data, with a folder picker fallback, a Preview tools overlay and default independent unlocked mission catalog.
- [ ] **Controls and settings** — partial; Runtime integration. Keyboard/mouse bindings, sensitivity, display and audio settings, persistence and focus handling.
- [ ] **All gameplay camera modes** — partial; Runtime integration. First/third person, aiming/zoom, vehicles, base building and special attacks remain partial. Owned mission inspection consumes a Session-bound outer view with terrain adjustment and prepared-mesh clipping. Controlled subjective post now produces the inner camera before placement/fire, with camera invocation counts separate from world frames. Four controls match 320 frames/12 shots and both camera caches; another 64-frame control checks inner view matrices. Full world/list admission, smoothing, special modes and main-driver subjective drawing/fire remain open. See docs/OWNED_CAMERA.md.
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
- [ ] **Multiplayer base-building rules** — todo; Runtime integration. Resource competition, faction construction, victory options and timed protection/dÃƒÆ’Ã‚Â©tente rules.

### Whole-game validation

- [ ] **Complete content coverage** — partial; Runtime integration. Audit every required mission, actor, weapon, spell, building, vehicle and presentation variant.
- [ ] **Text and language coverage** — todo; Runtime integration. Menus, objectives, subtitles, fonts and available dialogue/audio variants.
- [ ] **Playable Windows build stability** — partial; Runtime integration. Full-session play, repeated loading/restart, resource cleanup, performance and supported rendering paths. Silent development-session launch/return cycles observed on 4 November 2026; campaign loading, manual input and complete memory reclamation remain unverified. Owned descent/contact selection and real deferred slide-effect preparation now pass the former step-158 boundary. The latest exact muted-sink Mecc movement replay passes the former height and unequipped fuel-depletion refusals, then stops explicitly at step 441 on an unacquired impact reaction. Hard-landing preparation now has bounded CPU controls; complete impact/health integration and active world effects remain open. See docs/OWNED_MECC_CONTACT.md.
- [ ] **Complete gameplay and presentation fidelity** — partial; Fidelity. Compare complete scenes and mission outcomes, timing and behaviour across all three characters and game modes.

### Technical foundations

- [x] **Reproducible Windows MinGW build** — done; Build. 
- [x] **Test-build menus and mission selection** — done; Runtime. 
- [x] **Controlled normal Mecc cycle, orientation and complete payload** — done; CPU only. 
- [ ] **Controlled Kabuto construction, growth and physical subphases** — partial; CPU only; live admission pending. Owned cold groups, growth decisions/size pursuit and numbered history, motion, classification, additional contacts, model rates, ANM/gravity and control forces now lead into a Factory/Session post-control guard and resource/charge/speed-latch phase. Actual owner damage/class limit, current descriptor/channels, model/commands and repeated shared object-clock queries drive it. Its combined original interval matches 512 source-backed contexts / 1,317,724 final/scratch/clock/service-prefix words, with 2,389 clocks, 170 sound requests and 170 notifications. Services observe the published real Session; header/payload mutations are reread, while failures retain prior effects and prevent replay. Earlier isolated guard and timer controls retain 512 / 256,743 and 512 / 141,758, with separate scopes in OWNED_KABUTO.md. Commands, prior resource/latch state, sine tables, raw network context, flat terrain/empty prepared scene and service entry providers remain declared boundaries. Reinitialization internals, required global tail, actual sound/network backends, following action/animation, full primary and world/camera/GPU cadence remain open. All initializer/history prerequisites remain; CPU composition is not playable readiness or visual fidelity. The following reference consumer now has an acquired shared velocity-addition dependency (512 / 208,572), with source-owner notification gating and target-owner packet identity; actual typed target dispatch and caller composition remain open.
- [x] **Controlled herd and Vimp kernels** — done; CPU only. 
- [x] **Controlled actor callback policy and acquired owned adapters** — done; CPU only. 
- [x] **Approved reference and Bevy comparison gallery** — done; Historical public gallery. Owner-authorized original / Bevy comparisons are available in the dashboard; historical captures do not certify the current build or full-game fidelity.
- [x] **Controlled offline normal Mecc header and primary contract** — done; CPU only. 

### Research and validation

- [ ] **Reviewed prototype PS2 function attribution** — partial; Research method; separate from runtime and fidelity. Mandatory name lookup with pinned PC/prototype identities, literal symbols, uncertain demangling, retained aliases and independent evidence. Initial read-only audit verifies 4,769 function symbols; six reviewed services include three confirmed identities, two probable candidates and one unassigned. Seven current primary/subjective lookups remain explicitly unassigned, with retained PC aliases and owned Rust relationships. Broader mapping and type recovery remain incomplete. Method: docs/RECONSTRUCTION_METHOD.md.
<!-- progress:end -->
