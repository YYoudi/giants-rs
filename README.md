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

Current Windows download: [preview-2026.11.12.2](https://github.com/YYoudi/giants-rs/releases/tag/preview-2026.11.12.2).
The common build completes **The Wife**: reach its authored rescue destination,
let the real six-director completion film finish, then continue into **The Road
Home** with the nearby declared wife picked up and carried by the ordinary Mecc.
The preceding **Starving!** twenty-meat hunt and return also remain supported.
Open **Preview tools -> Browse all missions -> Mecc Story 2 -> The Wife**.
WASD/mouse move and aim, hold right mouse for the jet, Space adds nitro, left
mouse fires the partial pistol, R changes view and Escape returns to the menu
(or explicitly skips a film). F8 toggles objective hints. No replay flag enables
these actions, the rescue, pickup or continuation.

The exact executable passes both rendered mission chains, real muted audio,
source film terminals, cleanup and common GPU regressions with supplied commands.
Separate actual Windows checks cover The Wife entry/skip/view/fire and two
RoadHome pickups with clean returns. They do not establish a full OS-played
rescue or sustained held flight. Other mission completion/AI, animation blending,
full native inventory/world continuity, visual/audio fidelity and Delphi/Kabuto
gameplay remain incomplete. See [release notes](docs/PREVIEW_NOTES.md), the central
[mission method](docs/MISSION_PROGRESSION.md) and [BACKLOG.md](BACKLOG.md).

Engine contributors can use the reviewed [original symbol navigation](docs/SYMBOL_NAVIGATION.md)
to find related Rust code from prototype PS2 names. Confirmed, probable and unknown
attributions are labeled separately.

The [main integration process](docs/INTEGRATION.md) connects the cinematic and
character work to the shared application and validated Windows previews. CPU
services, diagnostic captures and ordinary gameplay availability are tracked
separately; a source merge alone does not make a feature playable.

Development now follows a [gameplay-first objective](docs/GOAL_GAMEPLAY_FIRST.md):
ordinary menu/input, movement, camera and basic actions first; mission interactions
and campaign coverage next; specialized fidelity cases afterward. Each delivery
must identify what can actually be tested, its controls and the exact build.
Necessary CPU dependencies keep their evidence scope until the interaction is
connected and verified. The goal remains the complete game.

The [roadmap dashboard](https://yyoudi.github.io/giants-rs/) is generated from this
repository's `PROGRESS.json` and updates through GitHub Pages on main-branch pushes.
It separates the [published Windows build snapshot](https://yyoudi.github.io/giants-rs/#current-preview)
from searchable source-status tables and the owner-authorized historical comparison
gallery. Captures with matching source, tag and executable receipts are labeled as
current-download captures; older diagnostics remain explicitly archived. Neither
label certifies native fidelity; game artwork remains outside the project MIT license.
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

- [ ] **Complete Mecc campaign** — partial; Runtime integration. Preview .12.1 completes Starving. Preview .12.2 validates the ordinary The Wife rescue, natural six-director completion film and following RoadHome entry with the declared wife picked up. The remaining Mecc missions, encounters and complete campaign remain partial. See MISSION_PROGRESSION.md and BACKLOG.md.
- [ ] **Complete Delphi campaign** — todo; Runtime integration. All missions and character-specific objectives and encounters.
- [ ] **Complete Kabuto campaign** — todo; Runtime integration. All missions, growth stages, encounters and ending.
- [ ] **Mission objectives and scripts** — partial; Runtime integration. Preview .12.1 connects source collection/stationary proximity producers, quota/counter/notices and next-ticket/source-film continuation. Preview .12.2 validates The Wife 1/1 rescue and its separate completion-film owner. Death/moving/equipment and other predicate/record types remain unbound; no general mission VM completion claim.
- [ ] **Chapter and mission continuity** — partial; Runtime integration. Preview .12.2 validates The Wife rescue -> natural source completion film -> ordinary RoadHome player, with that record's own declared selector3 wife, real proximity pickup, carrying load, audio and notice consumers. Separate Windows entry/pickup/re-entry checks pass. Native equipment, persistent-world, score/census and full save-game continuity remain incomplete.
- [ ] **Complete cinematic playback** — partial; Runtime integration. Preview .12.2 validates the six The Wife completion directors through their natural source end, with drained real muted voices, canonical Session effects, source support poses/contact children and actual declared collision inputs. Reached source requests are acknowledged by director/offset; future declarations do not extend the animation. Native clock/RNG interleaving, raster platform footprints, animation blending and complete visual/audio fidelity remain unverified. See BACKLOG.md and MISSION_PROGRESSION.md.
- [ ] **Ending and credits flow** — todo; Runtime integration. Final sequence, credits presentation and return to menus.

### Meccs

- [ ] **Mecc movement and jetpack** — partial; Runtime integration. Preview .11.4 exposes normal WASD/mouse, right-mouse thrust, Space nitro and R view through the shared partial controller. Pending fire/nitro cancel on focus/menu loss. Three supplied-command GPU entry/flight/release/return cycles and actual terrain landing/dust/audio pass. Real OS-held flight on this exact executable, slide, carried/special profiles, native primary/damage and complete world scheduling remain unverified or missing. See BACKLOG.md.
- [ ] **Mecc squad and orders** — todo; Runtime integration. Following, moving to a location, attacking, jet use, incapacitation, recovery and repair.
- [ ] **Complete Mecc weapon arsenal** — partial; Runtime integration. Preview .11.4 connects ordinary partial-pistol hits to actual Vimp collision meshes, damage/recoil, blood, hurt/recovery and death owners. Supplied-command GPU hunt and real muted injury/death sinks pass. The complete arsenal, canonical burst/weapon primary, other damage families and OS gameplay remain unverified or unavailable.
- [ ] **Mecc inventory and ammunition** — partial; Runtime integration. Carry limits, weapon selection, ammunition, reload/cadence, pickups, drops and resupply.
- [ ] **Mecc packs and upgrades** — partial; Runtime integration. Jetpack upgrade, repair, shield and bush camouflage, with their energy costs and restrictions.
- [ ] **Mecc items and deployables** — todo; Runtime integration. Mines, grenades, health syringes, flares, deployable turrets and pop-up bombs.
- [ ] **Gyrocopter** — todo; Runtime integration. Access, piloting, passengers, machine gun, bombs, landing, repair, resupply and destruction.
- [ ] **Mecc carrying and interaction rules** — partial; Runtime integration. Carry/rescue Smarties, collect meat and use shops, platforms, vehicles and mission objects.

### Sea Reapers / Delphi

- [ ] **Delphi movement and water** — todo; Runtime integration. Complete Delphi movement, water, turbo, weapons and character interactions remain unimplemented in the ordinary application. Download .11.1 offers Preview tools -> Browse all missions -> Delphi first world -> free inspection -> F7 initial-body focus, R reset and Escape return; two actual OS entries pass. Ordered projection, shared look, evaluated body caches and generic class0/state7 shell are acquired dependencies. App must reuse that same shell identity through retyping and persistent camera update, then bind actual target/physical/held-copy/scene/water/input producers. Body inspection is not playable admission.
- [ ] **Sword combat and combinations** — todo; Runtime integration. Melee attacks, combinations, targeting, contact and damage.
- [ ] **Complete Sea Reaper bow arsenal** — todo; Runtime integration. Regular, Lightning, Hellfire, Screamer and Power-Up bows; charging, lock-on and multiple targets.
- [ ] **Offensive spells** — todo; Runtime integration. Cluster, Cluster Mine, Firewall, Fire Circle, Hail and Tornado, including their targeting and duration rules.
- [ ] **Time and target-control spells** — todo; Runtime integration. Slow Time, Shrink and Follow; area effects, exclusions and restoration.
- [ ] **Utility spells and spell inventory** — todo; Runtime integration. Smartie Grab/Bubble, Cloak, Teleport and Repair; spell slots, charges and replenishment.
- [ ] **Reaper Ski** — todo; Runtime integration. Mounting, water/land handling, turbo and missile pickups, vehicle health and destruction.

### Kabuto

- [ ] **Kabuto movement and growth** — todo; Runtime integration. Complete Kabuto movement, growth, combat, feeding and size-dependent interactions remain open. Download .11.1 exposes static initial-body inspection through Browse all missions -> Kabuto -> F7, using actual owned look/size and a compatible external camera profile. Two OS entries, R reset and clean return pass. ProjectionReset, AnimationCache, ProceduralMaterial and GeometryRegistration remain pending; startup/warm order and live primary/input are missing. No playable Kabuto claim follows.
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
- [ ] **Enemy and boss behaviour** — partial; Runtime integration. Complete enemy perception, navigation, attacks and mission integration remain open. The public preview retains bounded ordinary Vimp interaction. The NPC development branch connects source-declared Ripper frames, body/contact submissions, the normal Mecc dedicated collision producer, owned launch/current-scene flight arithmetic, class230 cooldown/retreat, animated source projectile submissions and earlier local menu/re-entry checks. Projectile GPU encounter acceptance, impacts, reciprocal Ripper combat and death remain open. Complete native-world admission and scheduling remain unverified. See BACKLOG.md.
- [ ] **Wildlife and non-player inhabitants** — partial; Runtime integration. Preview .12.1 retains ordinary Vimp contacts/hurt/death/blood/ghosts/meat and connects twenty-meat Starving collection/return. Authored herd admission replaces the Story2 guard; Story2 and Story5 actual GPU populations are checked. General economy, other AI, non-Mecc gameplay and full native-world/FX fidelity remain open.
- [ ] **Complete projectile and damage rules** — partial; Runtime integration. Complete native damage, hit reactions, death, targeting and weapon interactions remain open. Download .11.1 has the ordinary partial Mecc pistol, source projectile presentation and acquired landing feedback. Short OS fire presses work; this does not establish native impact/damage behavior. Ordered burst and immediate-child services cannot run live until their canonical construction/command/contact/audio consumers exist.
- [ ] **Injury, death and respawning** — partial; Runtime integration. Reactions, incapacitation, corpses/drops, recovery, player death and mode-specific respawn rules.

### Interface, worlds and audio

- [ ] **Complete game menus and profiles** — partial; Runtime integration. Start, mission selection, loading, pause, inventory/shop screens, errors and returns to menus. Desktop entry detects adjacent game data, with a folder picker fallback, a Preview tools overlay and default independent unlocked mission catalog.
- [ ] **Controls and settings** — partial; Runtime integration. Keyboard/mouse bindings, sensitivity, display and audio settings, persistence and focus handling.
- [ ] **All gameplay camera modes** — partial; Runtime integration. Complete source-driven player/cinematic cameras and projection across characters remain the scope. Download .11.1 retains the ordinary Mecc R view toggle and offers F7 initial Delphi/Kabuto body inspection from the normal mission browser. Kabuto reads actual owned look/size with a compatible external profile; free navigation/R reset retain inspection projection. Two OS entries per character pass. Native projection reset, same-identity player entry, startup gates and other playable controllers remain unfinished.
- [ ] **Character HUDs, maps and objectives** — partial; Runtime integration. Health, fuel, magic, adrenaline, weapons, items, resources, targeting, map, notifications and score.
- [ ] **Complete world and actor rendering** — partial; Runtime integration. Owned mission inspection now uploads camera-selected body models from the Session shared pose bank. Native controls match 1,280 selections and 256 complete model snapshots; six recorded Bevy endpoints cover automatic, forced and absent models. Full world visibility, part/material admission, subjective drawing, lighting, scenery and fidelity remain incomplete. See docs/OWNED_MODEL_DRAW.md.
- [ ] **Water and environmental presentation** — partial; Runtime integration. Waves, reflections, sky, fog, vegetation and environment interactions.
- [ ] **Animations and active effects** — partial; Runtime integration. Preview .11.4 retains normal Mecc jet/projectile/landing feedback and adds actual Vimp hurt, blood, death FX, animated ghost and meat to the common renderer. Exact-build supplied-command GPU passages pass with real muted sinks. Death FX retain explicit host-frame/terrain/splat preview policies. Other AI, full native material/timing fidelity and cinematic world producers remain open.
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
- [ ] **Playable Windows build stability** — partial; Runtime integration. The exact common .11.1 executable passes combined GPU/runtime/cinematic cleanup and actual OS menu/re-entry/exit: six manual mission tickets release with zero entities. 960 portable Rust controls, 60 installed controls, 44 Python checks, formatting and strict Clippy pass; 159 optional controls remain ignored in the portable run. Receipt-bound package notices/checksums and independent downloads are reviewed separately. These bounded controls do not establish whole-game stability, security or fidelity.
- [ ] **Complete gameplay and presentation fidelity** — partial; Fidelity. Compare complete scenes and mission outcomes, timing and behaviour across all three characters and game modes.

### Technical foundations

- [x] **Reproducible Windows MinGW build** — done; Build. 
- [x] **Test-build menus and mission selection** — done; Runtime. 
- [x] **Controlled normal Mecc cycle, orientation and complete payload** — done; CPU only. 
- [ ] **Controlled Kabuto construction, growth and physical subphases** — partial; CPU phases + body inspection; gameplay pending. Acquired cold owner, shared profile-camera kernel, current body references and size/look providers are retained. Download .11.1 exposes normal F7 static inspection and R reset, verified across two actual OS mission entries. Four native startup requirements remain pending; live primary/input and growth/gameplay are not admitted. CPU construction is an intermediate dependency. See OWNED_KABUTO.md and BACKLOG.md for preserved evidence and actual missing contracts.
- [x] **Controlled herd and Vimp kernels** — done; CPU only. 
- [x] **Controlled actor callback policy and acquired owned adapters** — done; CPU only. 
- [x] **Approved reference and Bevy comparison gallery** — done; Historical public gallery. Owner-authorized original / Bevy comparisons are available in the dashboard; historical captures do not certify the current build or full-game fidelity.
- [x] **Controlled offline normal Mecc header and primary contract** — done; CPU only. 

### Research and validation

- [ ] **Reviewed prototype PS2 function attribution** — partial; Research method; separate from runtime and fidelity. Mandatory name lookup with pinned PC/prototype identities, literal symbols, uncertain demangling, retained aliases and independent evidence. Initial read-only audit verifies 4,769 function symbols; six reviewed services include three confirmed identities, two probable candidates and one unassigned. Seven current primary/subjective lookups remain explicitly unassigned, with retained PC aliases and owned Rust relationships. Broader mapping and type recovery remain incomplete. Method: docs/RECONSTRUCTION_METHOD.md.
<!-- progress:end -->
