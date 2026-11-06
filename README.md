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
- [ ] **Complete cinematic playback** — partial; Runtime integration. 545 sources / 16 worlds; zero complete scenes verified. The main film integrates a bounded persistent attached-FX queue, shared support reuse and source-marked cleanup: 23 sources / 25 requests, with CPU creation/re-entry/release controls over 50 object lifecycles. Native source-actor phase controls match 100 support samples / 1,700 words; complete VM/world cadence is unverified. Preview 2026.11.06.1 has exact-build natural/re-entry and live-object cancellation controls, plus Rstory1 mission 3 UI checks: Escape after birth, natural re-entry and double-click before birth, returning through partial free camera with tracked cleanup. Delphi gameplay remains refused. The inventory retains 59 gap sources, including 56 general FX queue prerequisites, and 282 separate strict-profile reviews. Non-opening triggers, effect-object channel cadence, world scheduling/admission/capacity and complete visual/audio/subtitle parity remain open.
- [ ] **Ending and credits flow** — todo; Runtime integration. Final sequence, credits presentation and return to menus.

### Meccs

- [ ] **Mecc movement and jetpack** — partial; Runtime integration. Walking, running, jumping, jet fuel/recharge, nitro, carried weight, terrain contact and water hazards.
- [ ] **Mecc squad and orders** — todo; Runtime integration. Following, moving to a location, attacking, jet use, incapacitation, recovery and repair.
- [ ] **Complete Mecc weapon arsenal** — partial; Runtime integration. Handgun, RPG, machine gun, proximity missile, sniper gun, missile launcher, homing missile and Millennium Mortar need complete aiming/firing rules. Normal-pistol CPU services compose the owned primary, current held cache, subjective animation, targeting, projectile creation and effects. The controlled subjective post now produces its inner camera before placement and deferred firing. The main mission driver still has no integrated owned firing. Main-driver post/drawing binding, posed vertices, complete target contexts and positional audio remain open. Evidence and boundaries: docs/OWNED_LOCAL_FIRE.md.
- [ ] **Mecc inventory and ammunition** — partial; Runtime integration. Carry limits, weapon selection, ammunition, reload/cadence, pickups, drops and resupply.
- [ ] **Mecc packs and upgrades** — partial; Runtime integration. Jetpack upgrade, repair, shield and bush camouflage, with their energy costs and restrictions.
- [ ] **Mecc items and deployables** — todo; Runtime integration. Mines, grenades, health syringes, flares, deployable turrets and pop-up bombs.
- [ ] **Gyrocopter** — todo; Runtime integration. Access, piloting, passengers, machine gun, bombs, landing, repair, resupply and destruction.
- [ ] **Mecc carrying and interaction rules** — partial; Runtime integration. Carry/rescue Smarties, collect meat and use shops, platforms, vehicles and mission objects.

### Sea Reapers / Delphi

- [ ] **Delphi movement and water** — todo; Runtime integration. Owned cold construction, physical initialization and jump services are CPU-validated. Dry sword physical/history -> secondary selection -> zero-time ANM now matches six 256-step native/Rust cases / 12,243,456 normalized words over 22 actors, reference buffers, payload and ordered lists, plus shared RNG endpoints. Installation clip definitions now load read-only from two acquired PE data layouts and are checked against the owned class; no original table or executable is bundled. Full primary/camera/target producers, auxiliary callbacks, nonordinary effects, water/turbo, combat and live admission remain unimplemented. Separate native-only full selected-owner acquisition covers 768 steps; it does not enlarge the Rust comparison. CPU composition is not gameplay readiness or verified fidelity.
- [ ] **Sword combat and combinations** — todo; Runtime integration. Melee attacks, combinations, targeting, contact and damage.
- [ ] **Complete Sea Reaper bow arsenal** — todo; Runtime integration. Regular, Lightning, Hellfire, Screamer and Power-Up bows; charging, lock-on and multiple targets.
- [ ] **Offensive spells** — todo; Runtime integration. Cluster, Cluster Mine, Firewall, Fire Circle, Hail and Tornado, including their targeting and duration rules.
- [ ] **Time and target-control spells** — todo; Runtime integration. Slow Time, Shrink and Follow; area effects, exclusions and restoration.
- [ ] **Utility spells and spell inventory** — todo; Runtime integration. Smartie Grab/Bubble, Cloak, Teleport and Repair; spell slots, charges and replenishment.
- [ ] **Reaper Ski** — todo; Runtime integration. Mounting, water/land handling, turbo and missile pickups, vehicle health and destruction.

### Kabuto

- [ ] **Kabuto movement and growth** — todo; Runtime integration. Movement, jumping, collision at each size and growth triggers remain unimplemented. CPU controls cover cold group fields, animation-cache lifecycle, generated texture/shared RNG, ordered geometry/material retirement (32 scenarios, 144 removals) and registered custom cleanup (8 contexts, 36,624 words, 88 marks). The normal geometry-entry CPU packet also matches 64 native controls / 24,876 words; its state producer and real backend remain open. Marked actors stay owned. Common actor transition/retirement, world dispatch, camera acknowledgement, GPU material/geometry rendering, full primary and live admission remain pending. CPU parity is not playable readiness or fidelity.
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
- [ ] **Enemy and boss behaviour** — partial; Runtime integration. Detection, navigation, pursuit, attacks, reactions, faction relationships and scripted boss encounters.
- [ ] **Wildlife and non-player inhabitants** — partial; Runtime integration. Vimps, herds and other creatures: movement, feeding, fleeing, interactions and mission roles.
- [ ] **Complete projectile and damage rules** — partial; Runtime integration. Direct/explosive/guided attacks, impacts, hit ownership, shields, friendly fire and environmental reactions. On 5 November 2026, the preview pistol object query was bounded by its terrain stop, matching the existing owned-flight ordering. Synthetic regressions distinguish objects before and behind terrain. Silent installed-data development replays pass declared-obstacle hits, an empty-obstacle control and synthetic-target damage/reset; owned actor damage and native trajectory fidelity remain unverified.
- [ ] **Injury, death and respawning** — partial; Runtime integration. Reactions, incapacitation, corpses/drops, recovery, player death and mode-specific respawn rules.

### Interface, worlds and audio

- [ ] **Complete game menus and profiles** — partial; Runtime integration. Start, mission selection, loading, pause, inventory/shop screens, errors and returns to menus. Desktop entry detects adjacent game data, with a folder picker fallback, a Preview tools overlay and default independent unlocked mission catalog.
- [ ] **Controls and settings** — partial; Runtime integration. Keyboard/mouse bindings, sensitivity, display and audio settings, persistence and focus handling.
- [ ] **All gameplay camera modes** — partial; Runtime integration. First/third person, aiming/zoom, vehicles, base building and special attacks remain partial. Owned mission inspection consumes a Session-bound outer view with terrain adjustment and prepared-mesh clipping. Controlled subjective post now produces the inner camera before placement/fire, with camera invocation counts separate from world frames. Four controls match 320 frames/12 shots and both camera caches; another 64-frame control checks inner view matrices. Full world/list admission, smoothing, special modes and main-driver subjective drawing/fire remain open. See docs/OWNED_CAMERA.md.
- [ ] **Character HUDs, maps and objectives** — partial; Runtime integration. Health, fuel, magic, adrenaline, weapons, items, resources, targeting, map, notifications and score.
- [ ] **Complete world and actor rendering** — partial; Runtime integration. All required maps, terrain layers, buildings, scenery, characters, materials, lighting and shadows.
- [ ] **Water and environmental presentation** — partial; Runtime integration. Waves, reflections, sky, fog, vegetation and environment interactions.
- [ ] **Animations and active effects** — partial; Runtime integration. Character/equipment animation, attachments, particles, trails, explosions, magic and effect lifecycles.
- [ ] **Music, voices and gameplay audio** — partial; Runtime integration. Music, dialogue/subtitles, footsteps, weapons, spells, ambience, spatial mixing and stopping/releasing sounds.
- [ ] **Saving and restoring a game** — todo; Runtime integration. Restore progression and the required mission/player state reliably, including restart and profile selection.

### Multiplayer

- [ ] **Networked game simulation** — todo; Runtime integration. Hosting/joining, state replication, ownership, latency, disconnects and game transitions.
- [ ] **Lobby, teams and communication** — todo; Runtime integration. Map/race/team choice, match options, readiness, chat, scores and server controls.
- [ ] **Deathmatch and faction matchups** — todo; Runtime integration. Mecc/Reaper/Kabuto combinations, with or without full bases, scoring, friendly fire and respawning.
- [ ] **Capture the Smartie** — todo; Runtime integration. Capture, carrying, stealing, returning, scoring and victory, including the base-building variant.
- [ ] **Multiplayer base-building rules** — todo; Runtime integration. Resource competition, faction construction, victory options and timed protection/dÃ©tente rules.

### Whole-game validation

- [ ] **Complete content coverage** — partial; Runtime integration. Audit every required mission, actor, weapon, spell, building, vehicle and presentation variant.
- [ ] **Text and language coverage** — todo; Runtime integration. Menus, objectives, subtitles, fonts and available dialogue/audio variants.
- [ ] **Playable Windows build stability** — partial; Runtime integration. Full-session play, repeated loading/restart, resource cleanup, performance and supported rendering paths. Silent development-session launch/return cycles observed on 4 November 2026; campaign loading, manual input and complete memory reclamation remain unverified.
- [ ] **Complete gameplay and presentation fidelity** — partial; Fidelity. Compare complete scenes and mission outcomes, timing and behaviour across all three characters and game modes.

### Technical foundations

- [x] **Reproducible Windows MinGW build** — done; Build. 
- [x] **Test-build menus and mission selection** — done; Runtime. 
- [x] **Controlled normal Mecc cycle, orientation and complete payload** — done; CPU only. 
- [x] **Controlled herd and Vimp kernels** — done; CPU only. 
- [x] **Controlled actor callback policy and acquired owned adapters** — done; CPU only. 
- [x] **Approved reference and Bevy comparison gallery** — done; Historical public gallery. Owner-authorized original / Bevy comparisons are available in the dashboard; historical captures do not certify the current build or full-game fidelity.
- [x] **Controlled offline normal Mecc header and primary contract** — done; CPU only. 

### Research and validation

- [ ] **Reviewed prototype PS2 function attribution** — partial; Research method; separate from runtime and fidelity. Mandatory name lookup with pinned PC/prototype identities, literal symbols, uncertain demangling, retained aliases and independent evidence. Initial read-only audit verifies 4,769 function symbols; six reviewed services include three confirmed identities, two probable candidates and one unassigned. Seven current primary/subjective lookups remain explicitly unassigned, with retained PC aliases and owned Rust relationships. Broader mapping and type recovery remain incomplete. Method: docs/RECONSTRUCTION_METHOD.md.
<!-- progress:end -->
