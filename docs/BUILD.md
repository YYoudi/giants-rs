# Build on Windows with MinGW

For source no-shot surface checks, the existing muted offscreen
building input fixture accepts `GIANTS_MISSION_BUILDING_CAPTURE=unchanged`.
It selects an already admitted source state0 mesh, supplies ordinary contact,
retreat, look and pistol inputs, then requires three actual distinct projectile
hits and unchanged player/projectile geometry. No health, collider, effect or
readiness is written to reach that endpoint. This does not enable an action or
make an unsupported callback indestructible; require real rendered acceptance
on its own receipt and follow [BUILDINGS.md](BUILDINGS.md).

For the initial static-shadow dependency, run `cargo test --locked -p
giants-game actor_static_shadow --target x86_64-pc-windows-gnu`. Its optional
`installed_entrance_retains_registered_shadow_and_shared_copy_identity` control
uses `GIANTS_BUILDING_ROOT` and `-- --ignored`. The app's
`installed_entrance_damage_updates_player_shape_without_inventing_environment_owner`
control uses the same variable. These check initial fields/copies and supplied
damage/geometry ownership, not GPU shadow drawing, Windows input or generic
Raster3 admission; keep the current public preview receipt separate.

The mixed guard effect owner has the optional installed control
`cargo test --locked -p giants-app installed_guard_graph --target
x86_64-pc-windows-gnu -- --ignored`, using `GIANTS_BUILDING_ROOT`. It checks real
source graph resources, lit/unlit draw submissions, contact/lifetime descendants,
clock idempotence, context cleanup and ticket isolation under supplied emission
and time. It does not itself prove GPU/ordinary-input acceptance; preview .13.2
records the separately delivered guard damage/replacement path. Follow
[BUILDINGS.md](BUILDINGS.md).

The barracks effect continuation uses the same installation variable:

```powershell
cargo test --locked -p giants-game installed_barracks_inherited --target x86_64-pc-windows-gnu -- --ignored
cargo test --locked -p giants-app installed_barracks_graphs --target x86_64-pc-windows-gnu -- --ignored
cargo test --locked -p giants-app installed_rigid_material_owners --target x86_64-pc-windows-gnu -- --ignored
```

These check prepared contexts/contact variants, complete descendant graphs with
fading draw submissions, and quantized shader uniforms/blend-mode updates and
buffer release. Requests, selection inputs and clocks are supplied. They do not
enable ordinary barracks damage, render GPU pixels or validate Windows input.

The ordinary house owner also has an installed app control:
`cargo test --locked -p giants-app installed_house_damage_updates_all_contact_owners
--target x86_64-pc-windows-gnu -- --ignored`, using `GIANTS_BUILDING_ROOT`.
It supplies hits and time and does not establish GPU or ordinary controls.
The explicit muted offscreen `showcase-mecc` Story2 replay can additionally use
`GIANTS_MISSION_BUILDING_PROBE=1`; `GIANTS_MISSION_BUILDING_CAPTURE` selects
`damaged`, `destroyed` (default), or `settled`. It supplies only normal movement,
look and fire commands and observes actual contact/health/retirement. It neither
writes poses/health/resources nor enables the ordinary building feature. The
settled phase also requires all actual burst bodies to expire. The optional
`GIANTS_MISSION_BUILDING_CLASS` selects a positive authored class already admitted
by the ordinary owner; e.g. 515 selects a small state101 tower, without changing
its health, pose or collision. The default selects a house. Keep this evidence
separate from real Windows inputs and source fidelity.
`GIANTS_MISSION_BUILDING_CYCLES=2` additionally requests two ordinary cancellation/
fresh-entry cycles after the selected live-effect stage; `settled` is refused.
The observer requires real live particles at cancellation and waits for the
normal runtime/environment/owned-entity release before the next request. It does
not force destruction, cleanup, readiness or a new source terminal.

The active building work follows [BUILDINGS.md](BUILDINGS.md). Dependency checks:
`cargo test --locked -p giants-game building_damage --target x86_64-pc-windows-gnu`.
With a user-supplied installation configured in `GIANTS_BUILDING_ROOT`, run
`cargo test --locked -p giants-game installed_houses_share_health_pose --target
x86_64-pc-windows-gnu -- --ignored`. This checks source health deformation and
consistent initial player/projectile geometry; it does not test ordinary damage,
effects/audio, mission credit, retirement, GPU response or actual Windows input.
Those remain required on the final identified menu-first executable.

For meat collection, follow [MEAT_COLLECTION.md](MEAT_COLLECTION.md) and the
identity-bound checks in [PREVIEW_NOTES.md](PREVIEW_NOTES.md). In normal Story2 /
Starving!, hunt a Vimp, wait for settled meat, approach it and observe the carried
count, pickup sound and disappearance. Return to the menu and enter a fresh
mission. Full carried inventory must leave remaining pieces uncollected. The
normal action needs no replay flags. For admitted source-owned mission objectives,
follow [MISSION_PROGRESSION.md](MISSION_PROGRESSION.md) and the current exact-build
release notes; preview .11.4 remains the earlier uncredited collection build.

The internal `GIANTS_MISSION_MEAT_PROBE=1` adds only supplied hunt/approach commands
to the existing `GIANTS_MISSION_VIMP_PROBE=1` Story2 `showcase-mecc` replay. It
requires the existing offscreen silent/muted transport and user-supplied data
folder. It must observe actual births, settlement, source collection, carried
state, real pickup sinks and collected-owner release, with no NPC/collection/audio
refusal. Add the existing `cycle-cancel` subjective probe to check cleanup after
collection. These diagnostics neither enable pickup nor prove OS-held input.
Preserve source/tag/EXE receipts before publishing their captures. New native
dependencies still begin with the central method's prototype PS2 name lookup.

For the current reliable-Mecc/first-living-interaction delivery, first exercise
the shared `mecc_input` edge controls and `mission_runtime::signals` ownership/
reset control, then the `frame_clock` and `world_noise` kernel tests. The clock
and noise original-PC comparison is retained privately, with actual OS clock
values supplied and no game helper replaced. These are dependencies: keep their
source identity and supplied-input scope explicit. Preview .11.2 passes connected
Story2 reaction/render/audio and common GPU/menu lifecycle checks; real OS-held
flight and near-herd interaction on that executable remain unverified.

For the Starving campaign input fixture, `GIANTS_MISSION_CAMPAIGN_PROBE=1` extends
the existing muted offscreen Story2 hunt/collection probe through the authored
return destination, normal next-ticket loading and the following source film's
actual end. It supplies commands only, with no resource, position, readiness or
film-terminal writes. Require twenty actually collected meat, both source
producers, real old-ticket release, saved reached-entry bits, the next source film
end and a fresh ordinary player with cleared meat. Other missions do not inherit
this acceptance. The ignored `installed_round_trip_with_ordinary_motion` control
accepts a private installation root and waypoint file to diagnose the same
terrain-route input fixture without rendering; it never proves campaign or OS
integration. Failed graphical candidates remain private and separate from the
eventual delivery receipt.

For the .12.2 The Wife dependency controls, `cargo test --locked -p giants-game smartie_follow
--target x86_64-pc-windows-gnu` exercises the ordinary selector3 selection
dependency. This is not a pickup/gameplay test. The ignored app control
`installed_wife_completion_has_real_prepared_render_and_effect_owners` uses
`GIANTS_MISSION_OWNED_ROOT` and prepares the actual six-director completion film.
It requires canonical owners, actual root/child geometry and the prepared
collision world for all six directors. Neither this preparation nor the
portable completion-receipt tests establish graphical mission acceptance.
The ignored game control
`installed_declared_wife_selects_actual_start_player_and_attaches_once` uses the
same root and checks the next record's actual declared actor/player start,
retained scan cooldown, typed ownership, pickup services, flag preservation and
duplicate rejection. Its reference/clock/random inputs remain a CPU fixture.
The ordinary whole-chain graphical diagnostic uses
`GIANTS_MISSION_REPLAY=showcase-mecc`, `GIANTS_MISSION_OBJECTIVE_PROBE=1`,
`GIANTS_SHOWCASE_WORLD=Story2`, `GIANTS_SHOWCASE_MISSION=1` and muted offscreen
menu capture with a fresh user-data directory. It navigates through supplied
Mecc commands toward admitted authored proximity targets; it does not alter
poses, objective credit, resources or film ends. Its endpoint requires a new
ordinary player and any required declared pickup. Inspect the real completion
receipt/source terminal separately; the observer itself does not acknowledge it.
The separate ignored game control
`installed_wife_timed_parents_have_canonical_bodies_and_scoped_cleanup` uses the
same root variable. It verifies actual source shotgun-parent bodies, source
retirement-child construction and scope cancellation, with supplied clock/RNG/
epoch. It also checks that an unprovided child service remains a callback failure.
It does not establish the ordinary film's detached-child preparation or drawing;
the whole completion-preparation refusal above remains required.
`cargo test --locked -p giants-game actor_finalization --target
x86_64-pc-windows-gnu` also checks explicit selection of the prototype-derived
physical threshold with no fabricated PC argument. The historical default still
refuses a missing caller word. This dependency test does not establish PS2
whole-function execution, exact cross-platform numerics or ordinary film drawing.
The ignored game control
`installed_wife_children_use_actual_world_and_retained_platform_candidates`
uses `GIANTS_MISSION_OWNED_ROOT` and drives the real class828 descendants through
history, callback, physical/gravity and finalization stages on Story2 terrain and
declared rigid geometry. It retains the 48 source platform-flag declarations,
partitioning non-raster profiles from unresolved raster candidates. A point
admitted by an unresolved footprint remains an error. These are supplied-frame
dependency controls; graphical acceptance of the common consumer remains pending.

The common movie consumer now routes the supported timed family through the
mission's canonical Session and into the rigid renderer. Run the ignored app
control `installed_wife_timed_world_and_render_owners_prepare_together` with
`GIANTS_MISSION_OWNED_ROOT` to verify the actual render/world preparation and
whole-film asset admission and refusal without the actual world. The installed child control above now exercises
the same immutable world preparation. Neither control proves GPU drawing,
ordinary mission completion or native timing; those remain required.

The ignored game controls
`installed_source3_snapshots_real_class_zero_support_and_keeps_context_binding`
and `installed_wife_mixed_source_events_have_canonical_roots_and_children` use
the same root. They check real support/context identity and the mixed source
event walk, respectively. Supplied clocks/RNG and source data are not GPU or OS
input evidence. For a graphical completion-film/re-entry diagnostic, use
`GIANTS_MISSION_REPLAY=showcase-film-cycles`,
`GIANTS_MISSION_FILM_COMPLETION_REPLAY=1`, `GIANTS_SHOWCASE_WORLD=Story2` and
`GIANTS_SHOWCASE_MISSION=1` with the ordinary menu executable, fresh user-data,
`--offscreen --silent-capture --muted-transport`, `GIANTS_AUDIO_MUTE=1` and a
new capture path. This selects the actual declared completion film for inspection;
it does not complete the rescue condition or admit a player in that context.

The ignored game control
`installed_blood_body_uses_canonical_payload_and_requires_contact_services` uses
`GIANTS_MISSION_OWNED_ROOT` to construct the real source blood frame in a Session.
It checks separate null/active random passes, the actual inline integer budget,
single context consumption and a retained failure at missing contact services.
It does not supply a live contact, instantiate a surface child, render blood or
admit the completion film. The portable `effect_ground_emission` controls cover
the ordered contact decision; the private original-PC comparison supplies its
physical flags and intercepts factory/retirement calls rather than executing them.
The separate ignored `installed_source_budget_changes_only_after_matching_completion`
control supplies a contact word to the constructed source body, verifies its
cinematic response mark and checks budget publication/stale-completion rejection.
Its supplied contact is not a successful world query or child creation.

The ignored game controls
`installed_blood_source_pose_uses_real_callback_terrain_before_child_refusal`
and `installed_blood_contact_constructs_real_surface_child_and_retires_parent`
use the same root variable. Both evaluate a supplied pose of the actual source
film actor/reference and execute terrain queries over real Story2 data. The
second supplies the concrete source surface-child factory, observes the actual
budget decrement/deletion mark, advances the child through expiry and cancels
the complete scope. The first requires refusal without that factory. The pose,
frame clock and random stream are dependency inputs; these controls do not
execute normal FLK dispatch, draw the scene or complete The Wife.
`installed_surface_inheritance_and_late_query_rng_are_preserved` checks all eight
contact-packet inheritance combinations against the actual surface constructor
and verifies consumed null/active draws survive an injected late query failure.

For the all-five consolidation, run the shared workspace checks below and the
installed owner/normal-controller controls, including the generic player shell,
ordered Delphi projection/body phase, typed state0 children, class230 awakening,
Mecc immediate constructor/burst and Kabuto profile camera. The ignored controls
require the corresponding user-supplied source roots/executable; their supplied
contexts do not establish ordinary gameplay. On the combined graphical build,
repeat the normal menu/input/return cycle and short fire taps. Browse a Kabuto
mission, skip to free inspection and press F7 with a compatible installation;
check framing, projection, R reset, cancellation and re-entry. Repeat Delphi
initial-body F7 inspection through the same normal browser. Preview .11.1 passes
these actual OS checks (two entries each), brief keyboard/default mouse fire,
jet pulse, six zero-entity returns and normal exit; supplied-controller GPU and
full selected cinematic acceptance remain separately scoped. Kabuto's four startup
requirements must remain pending. Keep actual-input and replay evidence distinct.

Choose and report validations under [GOAL_GAMEPLAY_FIRST.md](GOAL_GAMEPLAY_FIRST.md).
The commands below remain required at their stated scope. For a playable-feature
claim, also test the ordinary menu and real keyboard/mouse path on the exact build,
with visible response, exit and re-entry. A synthetic/installed CPU control or
scripted GPU replay alone does not establish that interaction. Lead the handoff
with the actual test path and availability before numerical results.

For combined character/cinematic contributions, follow
[INTEGRATION.md](INTEGRATION.md) before recording a public preview milestone.
Keep the exact source/build identity and test scope separate from source-only
and diagnostic coverage.

Install Rust with the GNU Windows toolchain (Rust 1.98 or newer), Python 3.11+,
Git, and x86-64 MinGW-w64 GCC/binutils. Put their bin directories on PATH.
Visual Studio is not required. The lockfile pins dependency versions.

Native research follows [RECONSTRUCTION_METHOD.md](RECONSTRUCTION_METHOD.md).
The optional read-only `python -m tools.ps2_symbol_lookup --help` tool uses private,
locally configured inputs; its worksheets stay under ignored `.local`. Synthetic
CI requires neither prototype data nor an original installation.
Preserve build labels and previous identifiers with `--pc-profile` and repeated
`--alias`; the central method defines the private continuation record and the
separate, reviewed public attribution summary.

```powershell
rustup toolchain install stable-x86_64-pc-windows-gnu --profile minimal --component rustfmt,clippy
rustup target add x86_64-pc-windows-gnu --toolchain stable-x86_64-pc-windows-gnu
. ./tools/dev-env.ps1
cargo test --locked --workspace --target x86_64-pc-windows-gnu
python -m unittest discover -s tests -v
python -m tools.audit_source_tree --output .local/source-history-review.json
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --target x86_64-pc-windows-gnu -- -D warnings
./tools/build_preview.ps1 -OutputDirectory .local/build
```

The release executable uses the Windows GUI subsystem. For scripted CLI checks,
explicitly wait for it, for example with Python's `subprocess.run(..., check=True)`.
A bare PowerShell command can finish before the child and close its redirected
stdout; that is not a reliable CLI exit/output check. The Windows workflow waits
and captures the child's `--help` result explicitly.

The installed `source_camera_profile_reads_current_headers_and_checked_shared_states`
control uses `GIANTS_MECC_WORLD_ROOT`. It reads actual Mecc Session headers and
separate controlled-state inputs, then checks live variant changes, foreign/retired
identities and forced-view precedence. It is silent and does not load original
coefficient tables, publish projection or complete activation.

Default tests use synthetic inputs without installed game data. Ignored tests
requiring external data or audio devices are not part of this portable check.

The installed `source_normal_mecc_body_drives_jet_presentation_across_flight_and_release`
control also exercises short keyboard/mouse fire down/up pairs using actual Bevy
input flags and the installed pistol. It retains the press across zero-step host
updates, consumes it once, respects the pistol's cooldown and drops pending input
when disabled. These are supplied input flags; actual OS tap/re-entry checks on
the identified common build remain separate.

With compatible `GIANTS_DELPHI_ROOT`, run
`cargo test --locked -p giants-app source_delphi_menu_entry --target x86_64-pc-windows-gnu -- --ignored`
for common mission construction/role/re-entry checks. Its source menu request
uses an all-unlocked test mask; no actual UI input, campaign unlock or graphics
are claimed. `source_delphi_retained_player_attempt` in `giants-game` additionally
checks constructor identity after an explicit late child-resource failure.

`cargo test --locked -p giants-game vimp_animation --target x86_64-pc-windows-gnu`
checks locomotion/contact/damage precedence, native gait fallback, missing moving
descriptor requests, same-clip and unfinished-entry skips. With a locally set
`GIANTS_NPC_ROOT`, the ignored
`source_vimp_locomotion_animation_consumes_actual_resources_and_physical_phase`
control uses three installed animation resources after owned physics. It supplies
gait/contact flags, retains the unfinished frame and refuses ANM replay. It does
not establish live movement/contact production or ordinary mission admission.

With the same installation, the ignored `source_vimp_contact` control checks
unsupported-contact refusal and a retained physical clear after ground failure;
`source_vimp_active_movement_reads_owned_contact` checks the constructor/physical
owner consumed by movement. Nonzero contact and fault cases are explicit supplied
fixtures, not observed object collisions. These controls enable no playable map.

With `GIANTS_NPC_ROOT`, run
`cargo test --locked -p giants-app source_npc_completed --target x86_64-pc-windows-gnu -- --ignored`
for the actual common Vimp pose/Bevy asset consumer. It retains canonical Story2
staging but explicitly supplies offline birth/callback admission, quiet signals,
clocks and epochs. Two fourteen-body tickets check animated mesh updates, shared
pose copies, camera hide/resume, missing-mesh cleanup/quarantine and asset release.
This is CPU asset coverage, with no GPU or ordinary gameplay availability claim.

With the same root, run
`cargo test --locked -p giants-game source_passive_world --target x86_64-pc-windows-gnu -- --ignored`
for 134 source-backed eventless state0 actors over three supplied callback
steps, including zero/positive/forced-zero ANM entry. Four reference owners retain
unsampled storage; the animated building samples fifteen references and feeds
the actual shared position buffer. The selected-cohort CPU control checks current
affines, unchanged RNG, replay/retirement and retained ANM state after an explicit
environment-gate fault. Whole-world admission/control, clocks and
unused collision/platform providers remain explicit supplied boundaries.
The ignored `source_absent_primary_reference_guard` control separately uses the
actual two reference-bearing class resources and a declared nonzero cache fixture;
it checks retention and unsampled/current-reference refusal, not a generated pose.


With `GIANTS_NPC_ROOT`, run
`cargo test --locked -p giants-game source_variant_awakening --target x86_64-pc-windows-gnu -- --ignored`
for actual class230 constructors, owned first primary and marker birth. Flat terrain,
peer eligibility, clocks/camera and captured/suppressed audio are supplied. It
checks conditional birth, exact copied/reciprocal ownership, post-entry clock
selection, retained prefixes/late audio failure and replay/retirement refusal.
It does not enable mode62 onward, state48 scheduling or ordinary NPC gameplay.

With `GIANTS_NPC_ROOT`, run
`cargo test --locked -p giants-game source_parented_state0 --target x86_64-pc-windows-gnu -- --ignored`
for the three acquired special children across two tickets. It supplies local
callback admission/control/time, checks owned ANM/references and position buffers,
retains typed bindings and refuses replay, changed parenting, foreign and retired
owners. Environment providers must remain unentered. A completed child callback
publishes no world affine and does not acknowledge the parent turn or live AI.

With `GIANTS_DELPHI_ROOT` and an explicit `GIANTS_DELPHI_EXECUTABLE`, run
`cargo test --locked -p giants-game source_camera_coefficients --target x86_64-pc-windows-gnu -- --ignored`
for owned Delphi selection and external camera coefficients through projection
reset. Mode/owner inputs are supplied and projection requests are captured;
no App device submission, startup or gameplay admission is acknowledged.

With `GIANTS_DELPHI_ROOT` and optional `GIANTS_DELPHI_EXECUTABLE` as below,
`cargo test --locked -p giants-game source_delphi_local_phase --target x86_64-pc-windows-gnu -- --ignored`
runs six owned-phase controls: publication/physical consumption and later
activation refusal, retained prefix/cancellation, real proxy construction with
typed reference protection, cold/previous held-cache geometry, refused cache
provenance, and held-route static terrain publication. The cache control uses
actual shared candidate geometry and placed source actors. The terrain control reads three installed terrain resources and preserves
miss-point writes and late query failures; camera/items/time and the physical
floor remain supplied. Other controls use scripted terrain/view/geometry/linked
results and sound-request sinks. These checks do not
validate a complete ordinary turn.

With `GIANTS_FLICK_ROOT` and `GIANTS_FLICK_PERSISTENT_COHORT` (private, headerless
world/sequence TSV), run `cargo test --locked --workspace source_constant_cohort_moves_actual_body --target x86_64-pc-windows-gnu -- --ignored`.
This silent control checks actual constant-effect storage transfer to Session,
channel/reference presentation, rejected foreign supports, retained late-failure
prefixes, typed-reference cancellation refusal and independent source generations.
The current cohort has 23 sources /25 requests and exercises 92 queue lifecycles.
Its source support/clock inputs and self-comparison are ownership controls,
not new original-PC comparisons, native world admission or visual fidelity.

`cargo test --locked -p giants-game delphi_local_target --target x86_64-pc-windows-gnu`
checks the local caller's separate point/link lifetimes, inline-wide versus
linked-rounded readiness, retained terrain/linked/item-target failures, inactive
pending selection and nonlocal no-effect entry. Geometry/camera/item/proxy
providers are test doubles; these controls do not validate their runtime binding.

The ignored `source_timed_positioned_preparation_uses_owned_header_and_terrain`
control additionally requires `GIANTS_FLICK_ROOT`, `GIANTS_FLICK_TIMED_TERRAINS`
(private archive/member TSV) and `GIANTS_FLICK_TIMED_PREPARATION_CONTROLS` (private
original-PC rows). It compares actual Session preparation across sixteen static
terrains without per-step state injection, then checks retained contact writes,
stale affine and replay refusal after a late platform failure. Clocks, support,
physical caller argument and empty mesh/platform populations remain supplied;
this is not live mission admission or a fidelity check. Keep its inputs private.

With `GIANTS_FLICK_ROOT`, run
`cargo test --locked -p giants-game source_timed_virtual_support_is_not_an_ordinary_parent --target x86_64-pc-windows-gnu -- --ignored`.
This silent dependency control checks canonical payload transfer, virtual
support identity, foreign-owner staging without context/RNG consumption, actual
retirement-child ownership and cut cleanup across independent source generations.
It also refuses cancellation inside a pending callback. Its clocks/RNG/support
are supplied; the menu dispatcher and live world producers are not established.

With `GIANTS_DELPHI_ROOT` set to compatible installed data,
`source_delphi_dependency_order_keeps_group_identities_and_cleanup` checks the
common dependency-order service against the actual cold Delphi/copy/held group.
It deliberately permutes list0 registrations, then checks typed owner precedence,
unchanged reverse memberships/other lists, cleanup and reconstruction. Run with
`cargo test --locked -p giants-game source_delphi_dependency_order --target x86_64-pc-windows-gnu -- --ignored`.
This silent CPU control does not execute a complete world or validate gameplay.

With `GIANTS_MECC_WORLD_ROOT` set to compatible installed data, the ignored
`source_camera_owner_selection_keeps_callback_effects_and_identity` checks the
camera-owner prefix against actual Session actors and the existing subjective
preparation consumer. It covers duplicate classes, retained callback effects,
late errors, retirement, foreign owners and reconstruction. The reference-point
body, initial words and mode/force/profile inputs remain declared boundaries.
Run it alongside `source_mecc_preworld_camera_prepares_or_retires_actual_display`;
both are silent CPU compositions, not App input or complete world-order checks.

The portable `camera_mode` tests also cover activation reset, retained failure
prefixes, quarantine across shared commands and lens evaluation against the same
transition word. They use explicit profile/projection fixtures; these are not
App backend or GPU checks.

With `GIANTS_MISSION_OWNED_ROOT` set to compatible installed data,
`cargo test --locked -p giants-app source_actor_roles --target x86_64-pc-windows-gnu -- --ignored`
checks actual player constructor identity with a second actor of the same class,
local/other audio routing, motion ownership, foreign and retired owner refusal,
and mission cleanup. The same control checks constructor-bound ordinary jet model sizes and visibility,
stale-frame quarantine and unchanged Session animation/world ownership. It also
checks source pistol projectile visual preparation and foreign-ticket refusal. These
checks do not prove GPU rendering or held input. It is silent CPU composition.
The same environment enables
`cargo test --locked -p giants-app source_normal_mecc_body_drives_jet_presentation_across_flight_and_release --target x86_64-pc-windows-gnu -- --ignored`.
This control composes the actual ordinary body, motion publication and jet child
presentation over 1,600 supplied steps, including nitro, release and descent-clip
selection. It samples the current body and preserves constructor channels and
native-world admission guards; cancellation retires its owner. Real input and
GPU appearance remain separate checks on the exact executable.
The additional diagnostic
`source_owned_mecc_runtime_terrain_and_presentation` now passes 128 prepared CPU
frames with actual terrain, camera, ANM and weapon presentation. It checks the
weapon's absence during draw and presence once actually equipped. This does not
certify a graphical run or complete ordinary gameplay. Neither control proves native input or
visual fidelity.

With `GIANTS_KABUTO_ROOT` and, where necessary, `GIANTS_KABUTO_EXECUTABLE` set as
described below, `source_kabuto_held_bank` runs four silent source-backed checks
of the current/held-reference Session composition. They reach the actual preceding
resource checkpoint, then test callback publication/rereads, retained late errors,
replay/reentry refusal, owner removal, clean re-entry and use of retained caller
input after later channel/mode writes. The captured entry reuses the actual
resource-phase sample; the lower-level entry retains supplied inputs. Capture policy/scale,
caller decisions/counters/mode and the binding entry remain supplied. These do
not validate a continuous primary, actual binding/animation/transport bodies or
live Kabuto input. Optional `GIANTS_KABUTO_HELD_BANK_RECEIPT` must be an absolute
output path under ignored local storage; Cargo runs the test in the crate folder.
For example, set it to `Join-Path (Get-Location).Path '.local/bank-receipt.txt'`
from the repository root before launching Cargo. The receipt contains only the
selected diagnostic projection; keep it private.

With the same Kabuto root/executable variables, `source_kabuto_animation_commit_`
runs three silent installed checks of the selected-animation commit, late
notification failure and owner removal. They use the actual common setter and
zero-second ANM pipeline after the real resource stage. Selection/transport/world
policy remains supplied. Optional `GIANTS_KABUTO_COMMIT_INPUTS` and
`GIANTS_KABUTO_COMMIT_RECEIPT` exports require absolute paths under ignored private
storage. Their 512 contexts are independent controls, not completed game frames.

`source_kabuto_action_selection_` runs three further silent installed controls
with the same Kabuto root and an explicit `GIANTS_KABUTO_EXECUTABLE` definition
provider. They test actual Session selection and directed exits, retained late
sound errors and owner removal. Sound/network and one PC RNG context are declared
providers, not live backend validation. Optional `GIANTS_KABUTO_ACTION_INPUTS` and
`GIANTS_KABUTO_ACTION_RECEIPT` require absolute private output paths. The 512
contexts are independent; no primary/history or pending initializer is completed.

Per-emitter ambient step traces are disabled during normal play. For a private
diagnostic run, set `GIANTS_TRACE_AMBIENT_STEPS=1` before launching the application.
The choice is captured when each mission's ambient resources are mounted. This
restores the detailed `mission_ambient_step` state/RNG/service records; it can
produce large logs. Source preparation, voice start/stop/retirement, cancellation
and errors are still logged without this option. Keep diagnostic logs private.

With `GIANTS_DELPHI_ROOT` set to the user's installation (and optional
`GIANTS_DELPHI_EXECUTABLE`), the ignored
`source_delphi_candidates_use_current_geometry_and_preserve_target_payload`
test checks actual owned Delphi/ordinary-actor construction, held-ray targeting,
current geometry/mesh access, bank-backed terrain/target scanning in both view
routes, viewport access before held publication, unsupported range and stale/
foreign refusal, payload preservation and cleanup. It is silent CPU composition
with supplied time, flat terrain and camera/viewport snapshots; it enables no live
input or complete mission/world schedule.
The animation-definition reader accepts only its acquired executable layout.
Compatible installed data does not establish that `Giants.exe` in the same folder
has that layout. When it differs, set `GIANTS_DELPHI_EXECUTABLE` explicitly to the
read-only, identity-checked reference used for the acquisition. A layout refusal
is an unsupported input profile, not permission to bypass the reader or claim a
successful composition. The portable synthetic checks need no such executable.
With the same Delphi inputs, the ignored
`source_delphi_proximity_retains_counter_failure_and_refuses_profile_replay`
control exercises the explicit physical/item/proximity/secondary sequence with
source resources, supplied clock/flat ground and a recorded sound-stop request
sink. It checks profile refusal, counter persistence, retained late failure,
quarantine and clear/re-entry. Its owned population has no state102 interaction
actor; active services, actual sound stopping and full-primary/world admission
remain unverified.

With the same `GIANTS_DELPHI_ROOT`, the ignored
`source_delphi_proxy_is_direct_zero_state_without_effect_origin` control checks
three actual source-backed direct target proxies, zero state78 payload, unchanged
class flags, common-list-only membership, refused ordinary affine access, clear
and re-entry. It is silent CPU construction, with no live target assignment,
proxy update, render submission or world scheduler.

With the same `GIANTS_DELPHI_ROOT`, the ignored
`source_delphi_support_owns_private_payload_and_retains_ordinary_refusal` control
checks the explicit cold support factory in three actual mission catalogs. It
checks private payloads, shared resources, rollback, source release, clear and
re-entry while preserving ordinary-constructor and current-geometry refusal.
This is CPU ownership coverage, without initialization, world admission or a
complete native actor-retirement claim.

With `GIANTS_FLICK_ROOT` set to a user-supplied installation, the ignored
`source_timed_effect_owns_resources_and_quarantines_late_services` control checks
bounded timed resource ownership after context/external resource release, shared
ANM epoch wrap and retained writes/quarantine after declared late physical and
retirement failures. It is silent CPU composition, with no world admission,
original-process launch, window, audio or complete cinematic-fidelity claim.

With `GIANTS_MECC_WORLD_ROOT` set to a user-supplied installation, the ignored
`source_mecc_retained_smoke_stages_without_consuming_context` test checks the
three source smoke constructors, retained resources, separate active interval
sample, profile/missing-descendant refusal and unchanged prepared pool. This is
silent constructor staging; it admits no actor, active context or renderer.

With the same `GIANTS_MECC_WORLD_ROOT`, the ignored
`source_mecc_primary_retains_and_stops_canonical_smoke` control runs sixteen
supplied wounded-primary frames with explicit canonical smoke activation,
display-owner replacement and final cancellation. Foreign/duplicate activation
and unsupported stop profiles preserve state. It emits no particles and does not
launch the game; actor destruction and world scheduling remain prerequisites.

With the same `GIANTS_MECC_WORLD_ROOT`, the ignored
`source_mecc_smoke_parent_loss_retains_context_and_refuses_missing_storage`
control checks prepared/active parent-reference invalidation, peer preservation,
foreign/missing live storage refusal, retained origin/cursor and stop after loss.
The existing wounded-primary smoke control also cancels after supplied parent
death. These checks do not run a global cleanup pass or destroy dead actors.

With the same `GIANTS_MECC_WORLD_ROOT`, the ignored
`source_mecc_stopped_smoke_destroys_only_owned_resources` control checks bounded
stopped-emitter destruction, weak resource ownership, parent/peer preservation,
counts, generation reuse and refused live/foreign/stale/wrong-phase/dependent
requests. It does not execute particles or the general world cleanup schedule.

For an optional shared cache set GIANTS_CACHE_ROOT. To add the MinGW bin directory
set GIANTS_MINGW_BIN. Otherwise normal Cargo/Rustup settings and PATH are respected.
Use a separate Cargo target directory for each active worktree. Sharing downloaded
registry/toolchain caches is fine; sharing build artifacts between concurrently
modified worktrees can produce a test inventory from the wrong source. Reject
that result and rebuild in an isolated target before recording a release receipt.

Windows CI restores only Rust dependencies using a pinned cache action; workspace
crates and installed Cargo binaries are excluded. Its key includes the Rust/Cargo
environment plus the GNU compiler binary and MSYS package inventory. Only main
saves the cache. Each runner retains its own target directory and rebuilds the
project crates, tests and receipt; a cache hit is not validation of current source.
Private `.local` research, installed game data and runtime captures are not cached.

Package a draft with `python -m tools.package_preview --help`. Run the extracted
bundle with `./Play.ps1 -DataRoot 'D:/Games/Giants'`. Fonts are loaded from Windows
or GIANTS_INSPECTION_FONT, never bundled. Game data is read-only; user state goes
outside the installation. Interactive coverage is limited to the exact preview
and mission recorded in RELEASES.md; broader gameplay and fidelity remain pending.

## Optional NPC source-resource lifecycle check

With `GIANTS_NPC_ROOT` pointing to the user-supplied installation,
`cargo test --locked -p giants-app source_npc_menu_entry --target x86_64-pc-windows-gnu -- --ignored`
checks actual Story2 menu data, canonical mission resource/declaration binding and
two entry tickets without a window or audio. It supplies unlocked progress and
does not execute world callbacks or render Vimps. The separate ignored game-crate
`source_vimp_registered_factory` check verifies the registered image and rejects
missing/replaced namespaces. These are source controls, not gameplay test paths.

For NPC source-resource ownership regression, set `GIANTS_NPC_ROOT` to the
user-supplied installation and use an isolated Cargo target:

```powershell
cargo test --locked -p giants-game source_npc_herd --target x86_64-pc-windows-gnu -- --ignored --nocapture
cargo test --locked -p giants-game source_vimp_world_history --target x86_64-pc-windows-gnu -- --ignored --nocapture
cargo test --locked -p giants-game source_vimp_newborn --target x86_64-pc-windows-gnu -- --ignored --nocapture
cargo test --locked -p giants-game source_herd_frame --target x86_64-pc-windows-gnu -- --ignored --nocapture
cargo test --locked -p giants-game source_vimp_threat --target x86_64-pc-windows-gnu -- --ignored --nocapture
cargo test --locked -p giants-game source_vimp_vector --target x86_64-pc-windows-gnu -- --ignored --nocapture
cargo test --locked -p giants-game source_vimp_response --target x86_64-pc-windows-gnu -- --ignored --nocapture
cargo test --locked -p giants-game source_vimp_active --target x86_64-pc-windows-gnu -- --ignored --nocapture
```

The active-response block's portable tests run in the default workspace suite.
Its separate private captured-cycle comparisons retain supplied caller motion,
clock and external RNG draw schedule; they do not establish ordinary primary,
physical/ANM or world integration. See [OWNED_NPCS.md](OWNED_NPCS.md).

The active Session command adds six installed ownership controls: the arrival
timer persists without a fabricated constructor value, callbacks retain their
effects, and pending/foreign/reentrant/replayed visits refuse. The separately
ignored `source_captured_vimp_active_cycle_retains_session_outputs_and_random`
requires `GIANTS_NPC_ACTIVE_CONTROL` to be an absolute path to a private reviewed
captured-entry control, in addition to `GIANTS_NPC_ROOT`. It retains Session
outputs and RNG across 115 visits with supplied caller motion/clock/flags and
external draw counts. Inputs are not distributed; this is not a normal mission
or whole-primary test. Follow the central reconstruction method for acquisition.

The default `vimp_movement` controls cover the ordered scalar/command prefix,
separate actor/global clocks, strict retry expiry and retained clock errors.
The installed `source_vimp_active` group also covers the combined movement/active
lease. The optional ignored
`source_captured_vimp_movement_active_cycle_retains_outputs_and_random` requires
an absolute `GIANTS_NPC_MOVEMENT_CONTROL` private reviewed control path as well
as `GIANTS_NPC_ROOT`. Its 115-entry composition retains commands, scalar/retry
and active outputs and one RNG, while earlier caller gates, contact predicate,
motion/growth/flags/clock inputs and external draws remain supplied. Neither this
control nor the Story2 resource fixtures enable ordinary mission AI.

The response command checks actual inline physical/channel reads, signal-to-owned
vector composition, terminal callback effects, cancellation and noise-target byte
preservation. Priority/signal/channel entries are declared test inputs; session,
noise, affine entry and RNG phase remain supplied. It acquires no readiness or
caller producer, transport, live linked/target/hint ownership or ordinary primary.

The vector command checks isolated owned route publication, callback rereads,
late-error quarantine and cancellation against source actors. Session snapshots,
affine entry, origin and RNG phase are supplied. It adds no ordinary primary,
world schedule, live action transport, hint ownership or rendered coverage.

This silent CPU check uses authored herd/marker values with a supplied flat
floor, clock and RNG. It covers births, first initialization, owner identity,
terrain-failure quarantine, replay refusal and cleanup. It does not establish
live NPC AI, player interaction, world scheduling or rendering. See
[OWNED_NPCS.md](OWNED_NPCS.md).
It loads world declarations only; mission markers are outside this fixture.
The second command checks separate Vimp history/primary entry, retained late
terrain failure, missing-service preflight and eight controlled frames against
the existing combined Rust route. Time, ground, seed and control identity are
supplied; it establishes no complete native world frame or mission admission.
The newborn control exercises 14 real source births, first-callback continuation
and two warm continuations without overwriting constructor histories. A separate
required-FX failure retains initialized/physical state and rejects replay. Its
ground, cadence and control identity are supplied; no renderer or live sound is
exercised. Parentless allocation is outside the new herd-child entry.

## Optional subjective Mecc lifecycle check

The main owned inspection's model settings and limits are documented in
[OWNED_MODEL_DRAW.md](OWNED_MODEL_DRAW.md). For a silent repeated resource check,
set `GIANTS_MISSION_MECC_BACKEND=owned-inspection`,
`GIANTS_MISSION_REPLAY=owned-player-inspection-cycles` and optionally
`GIANTS_MISSION_MODEL_QUALITY=0`. Launch the same executable with `--mecc-menu`,
your `--data-root`, a separate `--user-data-root`, and
`--offscreen --silent-capture --capture` pointing into ignored local output.
This repeats the acquired 128-step inspection and cancellation, not normal
campaign admission or the unresolved longer movement/landing trajectory.

For a single silent descent snapshot, use the same owned backend and
`GIANTS_MISSION_REPLAY=owned-player-descent-inspection`. This follows the general
movement replay prefix through step 145, asserts its owned descent clip and checks
mesh upload before allowing capture. It stops before the known pending contact
branch at step 158. It does not validate that landing or the complete trajectory.

For the ordinary partial controller, `GIANTS_MISSION_REPLAY=campaign-descent-pose`
uses normal menu/mission admission, supplies prolonged thrust and boost, releases
thrust after step420 and captures only when the real selector chooses clip41.
It refuses after1600 steps if no such pose occurs. Run with a fresh isolated
`--user-data-root`, `--mecc-menu --offscreen --silent-capture --capture <private.png>`
and `GIANTS_AUDIO_MUTE=1`; leave the experimental owned backend unset. This is a
GPU observation with supplied commands, not held OS input or landing validation.
It does not enable the pose: ordinary right-mouse thrust/release already consumes
the same selector without this diagnostic. Keep capture/build identities private
and separate from downloadable-preview availability.

Set `GIANTS_MECC_WORLD_ROOT` to a user-supplied installation and use the isolated
target directory described above:

```powershell
cargo test --locked -p giants-game source_mecc_subjective_publishes_before_fire_and_retains_late_failure --target x86_64-pc-windows-gnu -- --ignored
cargo test --locked -p giants-game source_mecc_deferred_primary_requires_post_and_retains_real_fire_failure --target x86_64-pc-windows-gnu -- --ignored
cargo test --locked -p giants-game source_mecc_produced_camera_orders_post_fire_and_outer_publication --target x86_64-pc-windows-gnu -- --ignored
cargo test --locked -p giants-game source_mecc_preworld_camera_prepares_or_retires_actual_display --target x86_64-pc-windows-gnu -- --ignored
```

The pre-world camera control uses the acquired shared mode policy, actual Session
owner/class identities and prepared resources for admission, inactivity and
re-entry. Current mode/force/profile words remain supplied; it does not establish
their startup writers, native world ordering or application input integration.

The other silent CPU controls check the Session-bound subjective resource and shared
reference publication, retained effects after failure, inhibited fire and stale
identities after cancellation. Its fire failures are explicit test providers;
it executes no original program, real shot, whole-world scheduler or audio device.
The second control executes the owned deferred primary/held/post/local-fire path,
including real Rust projectile/effect construction and an injected late sound
failure. Camera inputs and an empty positional sound bank are controlled; it
creates no window, original process, full mission or audio device.
The produced-camera variant calculates the inner view before placement/fire and
requires post completion before the outer publication. It exercises success,
no-shot and late sound failure using a controlled flat world and known startup
camera. It does not draw a subjective actor or enable main-driver owned firing.
The same source-data controls check completed-post model positions, normal/forced
cache reuse and rejection of skipped, stale, failed, foreign or cleared geometry.
They preserve the body's invalidated reference binding after virtual publication.
The same installed-data checks cover the normal subjective held matrix, unchanged
real binding/firing cache and rejection after shared reference replacement. They
execute no live held geometry, position/spatial finalization or graphics upload.

## Optional positional-impact birth check

With `GIANTS_MECC_WORLD_ROOT` and an isolated target directory, run:

```powershell
cargo test --locked -p giants-game source_positional_impact_retains_ordered_actors_and_late_failure_without_replay --target x86_64-pc-windows-gnu -- --ignored
cargo test --locked -p giants-game source_positional_impact_ticks_owned_clocks_and_marks_only_after_stop --target x86_64-pc-windows-gnu -- --ignored
cargo test --locked -p giants-game source_positional_impact_retires_only_after_tracked_release_and_rechecks_references --target x86_64-pc-windows-gnu -- --ignored
```

This silent source-resource control constructs actual owned state-78 actors,
checks publication before the required cue provider, shared RNG interleaving,
retained late failures, replay/foreign-session refusal and resource ownership.
The provider is explicit test code. It creates no window or audio device,
launches no original executable and does not admit the impact into the main world
scheduler. The second control executes the actual owned timed callback, descriptor
clocks/references and shared ANM epoch; it checks deferred marking after a required
test stop provider and retained late failure. Marked actors remain owned. Neither
of the first two controls performs live audio transport or common retirement.
The third checks actor-local core retirement after a required test release
provider, reference rechecks, preserved peers, reuse and late-failure quarantine.
It does not execute the enclosing global cleanup prepass or a real device release.
See OWNED_MECC_CONTACT.md.

## Optional owned impact damage boundary checks

With `GIANTS_MECC_WORLD_ROOT` and an isolated target directory, run:

```powershell
cargo test --locked -p giants-game source_mecc_impact --target x86_64-pc-windows-gnu -- --ignored --nocapture
```

The secondary control checks actual Session health before returned channel
publication, damage-before-contact precedence, real positional preparation and
retained late carried-service failure. The primary control supplies a falling
entry, executes its real physical/ANM/secondary prefix and retains health at the
required missing pain-voice boundary, without cooldown/FX RNG draws or replay.
No window, device, active effect actor or complete trajectory is exercised.
See OWNED_MECC_CONTACT.md for the separate native comparisons and open services.

The same command also runs two additional installed-data controls. The voice
publication control observes committed Session health and timers through explicit
cue callbacks, including late failures and stale/foreign owner refusal. The
primary voice control uses the real post-prefix shared countdown: supplied cues
complete one falling-entry frame and prepare one context; either cue failure
retains health and prevents replay. These are required-service CPU controls,
not an audio device, main-driver admission or a playable trajectory.

## Optional actual impact audio checks

The local Mecc damage-voice transport has a separate silent device control. With
the same user-supplied `GIANTS_MECC_WORLD_ROOT`, an actual audio device, an isolated
target directory and `GIANTS_AUDIO_MUTE=1`, run:

```powershell
cargo test --locked -p giants-app source_owned_mecc_damage_voice --target x86_64-pc-windows-gnu -- --ignored --nocapture --test-threads=1
```

It uses actual installed pain/shared PCM and typed class-2 root registrations;
health/timers are supplied to the acquired voice tail. Four damage-cue starts,
one jet start, unique reuse, distinct RNG owners, nonlocal/root refusal and lost
sink handling are checked on real muted sinks. Quarantine/release removes all
owned audio assets. This is not a complete player, primary/world trajectory,
original audio-device ABI or audible-fidelity test. No window is created. The
ordinary primary context/provider stays unbound pending its acquisition gates.

Set `GIANTS_MECC_WORLD_ROOT` to a user-supplied installation and isolate the Cargo
target as above. The positive control requires an actual audio device and explicit
muting; the negative control creates no audio device. Neither creates a window.

```powershell
$env:GIANTS_AUDIO_MUTE = '1'
cargo test --locked -p giants-app source_owned_impact_audio --target x86_64-pc-windows-gnu -- --ignored --nocapture --test-threads=1
```

This runs actual owned birth/callback/retirement with real muted ready sinks,
checks separate RNG, exact owner/sound stop, late sink loss and full audio-asset
release. The queued-without-device control refuses playback without audio draws
and retains the failed actor/context without replay. Local anchor/listener inputs
are supplied. These tests do not bind the main secondary impact reaction, execute
the global cleanup prepass or certify audible/native-device fidelity.

## Optional Delphi resource check

For the separate Delphi resource-preparation control, set `GIANTS_DELPHI_ROOT`
to a user-supplied installation and run:

```powershell
cargo test --locked -p giants-game source_delphi_start_and_resources_remain_separate_from_mecc --target x86_64-pc-windows-gnu -- --ignored
```

This reads one selected mission's marker/class/state/specification resources.
It creates no window or sound and does not execute a Delphi constructor,
controller, water behavior or mission admission. Game data and private outputs
must stay outside Git. Keep a separate Cargo target for this worktree.

## Optional cold Delphi special check

The acquired Delphi cold special phase has a separate silent source-data control,
using the same `GIANTS_DELPHI_ROOT` installation:

```powershell
cargo test --locked -p giants-game source_delphi_special_group_owns_references_and_refuses_replay --target x86_64-pc-windows-gnu -- --ignored
```

It constructs the owned copy/held-object group, checks reference ownership,
special-only versus cold-player fields, owned initial physics, replay/failure
guards, rejected invalid frames and registry clearing. It executes no original process,
window or audio and does not admit a live Delphi controller. See OWNED_DELPHI.md.

## Optional owned Delphi physical composition check

For the owned ordinary Delphi physical/jump/ANM/history composition, with the same
`GIANTS_DELPHI_ROOT` and isolated target, run:

```powershell
cargo test --locked -p giants-game source_delphi_physical_owns_motion_shares_events_and_quarantines_failure --target x86_64-pc-windows-gnu -- --ignored
```

This silent CPU control checks seven owners, persistent motion/contact/jump,
shared class events, physical/locomotion/controlled/authored-camera profiles using external
clip definitions, separate look state, body roll settling, paused terrain exclusion, late secondary
and invalid-rotation quarantine, profile refusal and cleanup. It creates no window or audio device
and does not execute the complete Delphi primary/secondary or live controls.

The same control also samples real owned camera references and checks Delphi's
standalone and entered camera callbacks: retained/reset views, inactive inputs,
foreign targets, rejected profiles and failure quarantine. It verifies that a late
rotation failure retains completed camera effects and prevents their replay. It does not acknowledge camera rendering
or production full-primary scheduling. It also performs 64 ordinary third-person
camera updates from the completed controlled actor, using a supplied synthetic
profile and flat/no-hit providers. Future frames and other profiles refuse before
querying those providers. This is separate from the external immutable profile
reader and from production camera dispatch. A further 64 ordinary view updates
select both external profiles and compose lens/projection/pose, binding actual
owned kind/fields. Lens controls, viewport/depth and world data remain supplied;
future-frame/profile refusals happen before camera-state/provider consumption.
Private native comparisons retain their
separate arithmetic and test-driver scope in [OWNED_DELPHI.md](OWNED_DELPHI.md).

The fifth owner uses the acquired dry post-physical profile for 64 updates,
including paused timing/input cleanup. A required supplied positional-stop
provider records cues; it does not play audio. Wrong frozen entries refuse before
providers or epoch changes. A late secondary refusal retains completed physical /
ANM/history/post effects and quarantines retry. Full primary/other callbacks,
active sound transport, mission admission and fidelity remain unverified.

The sixth owner adds 64 dry passive reserve-profile updates, including supplied
recharge inhibition. Wrong entries refuse before provider/epoch consumption;
late secondary failure retains completed reserve timing/clearing and quarantines
retry. Active turbo, immersion and required movement/splash/nonempty-retirement
services remain unprepared. Native scalar and callback-prefix comparisons retain
separate supplied boundaries; neither this control nor those CPU comparisons
execute live Delphi gameplay.

The seventh owner adds four readiness-off movement requests during its
64 item/absent-auxiliary-tail updates after reserve and
checks retention of completed timing/history/ANM when a nonempty auxiliary
resource is injected. Retry preserves both payload and epoch. The actual common
actor list is scanned using acquired hidden/state/owner gates; eligible target
services remain refusals. This control supplies offline mode context and does not
execute active item/spell services, network transport or live Delphi gameplay.

The seventh owner also publishes 64 acquired debug held caches and target rays.
It keeps constructor reference storage unevaluated, checks replay/future refusal,
stale held position, replaced references and failed-parent rejection. These use
actual owned fields/bindings/reference buffers; the supplied normalization threshold
is a test input. Whole-world order, point/candidate selection and live Delphi
controls are not certified by this source-data check.

The locomotion profile reads the original executable's data as an external asset.
Set `GIANTS_DELPHI_EXECUTABLE` for a separate configurable metadata file; otherwise
this source-data control reads `Giants.exe` inside `GIANTS_DELPHI_ROOT`. The file
is never launched. Delphi currently accepts two acquired PE data layouts; other
variants refuse explicitly. These checks do not establish execution parity across
all native builds.

## Optional cold Kabuto resource check

For the cold Kabuto constructor/resource control, set `GIANTS_KABUTO_ROOT` to a
user-supplied installation and run:

```powershell
cargo test --locked -p giants-game source_cold_kabuto_keeps_player_setup_separate_and_refuses_unacquired_profiles --target x86_64-pc-windows-gnu -- --ignored
```

This reads the selected mission, constructs ordinary owned actors and checks
resource release, refused profiles and the cold player-field initializer without
a window or sound. It does not execute live type changing, camera completion,
movement or growth. See OWNED_KABUTO.md.

For the separate cold auxiliary constructor/reset control, use the same data
variable and isolated target:

```powershell
cargo test --locked -p giants-game source_kabuto_auxiliaries_construct_declared_states_then_clear_empty_effects --target x86_64-pc-windows-gnu -- --ignored
```

This checks the declared effect-state lifecycle, typed resource ownership,
refused nonempty/mask profiles and release without a window or sound. It does
not bind the auxiliaries to the player, tick them or admit a mission.

For the atomic cold group composition/refusal/re-entry control, use:

```powershell
cargo test --locked -p giants-game source_kabuto_group_composes_fields_links_and_physics_atomically --target x86_64-pc-windows-gnu -- --ignored
```

It uses `GIANTS_KABUTO_ROOT` and creates no window or sound. The extended control
also reads `GIANTS_KABUTO_EXECUTABLE`, defaulting to `Giants.exe` in that supplied
installation, to prepare optional cold animation definitions from a supported
PE data layout. It never launches that executable. Unknown layouts refuse; no
archive fallback or bundled original table is used. It checks cache/specification
ownership and release. Process/world cache-reset scheduling, material, registration
and camera services remain unacknowledged; no mission is admitted by this control.

The group control also prepares the optional normal-cold procedural CPU texture,
checks its shared RNG phase, warm reuse, group ownership and release. It does not
upload that image, infer GPU alpha/blending, or acknowledge a material binding.
It now also checks the registered cold path: actor/registry/RNG rollback on a late
foreign-ticket failure, same-ticket foreign-scope cleanup refusal, deferred marks,
unchanged ordered lists, last-material retirement and clean regeneration after
clear/re-entry. This is the custom callback phase, not common actor retirement.

For the bounded owned input/orientation and force-to-position prefix:

```powershell
cargo test --locked -p giants-game source_kabuto_motion_prefix_owns_fields_and_retains_pending_primary --target x86_64-pc-windows-gnu -- --ignored
cargo test --locked -p giants-game source_kabuto_classification --target x86_64-pc-windows-gnu -- --ignored
cargo test --locked -p giants-game source_kabuto_object_contacts --target x86_64-pc-windows-gnu -- --ignored
cargo test --locked -p giants-game source_kabuto_model_motion --target x86_64-pc-windows-gnu -- --ignored
cargo test --locked -p giants-game source_kabuto_animation_gravity --target x86_64-pc-windows-gnu -- --ignored
cargo test --locked -p giants-game source_kabuto_control_force --target x86_64-pc-windows-gnu -- --ignored
```

This uses the same `GIANTS_KABUTO_ROOT`, real cold resources, actual common
history and retained prefix state. Optional `GIANTS_KABUTO_MOTION_CONTROLS` /
`GIANTS_KABUTO_MOTION_RECEIPT` paths select private controls and receipts; neither
belongs in Git. Incoming commands/forces/preferences are supplied. It refuses
foreign factories, bad time and repeated/unfinished phases; it cannot acknowledge
the remaining contacts/control forces or start a second primary frame. No window,
audio device, world scheduler or gameplay admission is created.

For factory-bound target decisions and size pursuit composed with the following
auxiliary selection:

```powershell
cargo test --locked -p giants-game source_kabuto_growth_composes_size_and_auxiliary_without_admitting_gameplay --target x86_64-pc-windows-gnu -- --ignored
```

This silent control uses `GIANTS_KABUTO_ROOT`, traverses sizes and checks factory
ownership, failed-input rollback, retained gates and clear. Optional private
`GIANTS_KABUTO_GROWTH_CONTROLS` / `GIANTS_KABUTO_GROWTH_RECEIPT` paths replay the
acquired independent or successive-state source profiles. Neither the inputs
nor receipts belong in Git. `GIANTS_KABUTO_GROWTH_DECISION_CONTROLS` selects the
state-decision composition instead of supplying the resulting target. Poses, root
identity and the threshold remain supplied; this does not establish its producer,
feeding, world collision-cache updates or live gameplay. The default control also
checks the state-bound threshold/target decision, ordered sound request, foreign-
factory refusal and refusal after clear.

For second-auxiliary selection from actual owned resources and actor lists:

```powershell
cargo test --locked -p giants-game source_kabuto_auxiliary_animation_binds_owned_fields_and_refuses_stale_inputs --target x86_64-pc-windows-gnu -- --ignored
```

The control checks channel publication, unchanged payload/lists/ownership,
unsampled references, invalid-input rollback, foreign/stale identities and clear.
Private source-resource controls/receipts may use `GIANTS_KABUTO_AUXILIARY_CONTROLS`
and `GIANTS_KABUTO_AUXILIARY_RECEIPT`. Keep those inputs and outputs outside Git.
This is selection over supplied preceding poses/root identity, not world-pose
production, ANM advancement or a playable Kabuto controller.

For the owned geometry-transition composition, with that same supplied root:

```powershell
cargo test --locked -p giants-game source_kabuto_transition_preserves_order_and_quarantines_partial_failure --target x86_64-pc-windows-gnu -- --ignored
```

This silent installed-data control exercises successful create/mark and shared
ANM/reference publication, then an injected sound-provider failure and refusal
to replay. Pending initializer gates remain. Optional private control/receipt
paths can be supplied through `GIANTS_KABUTO_TRANSITION_CONTROLS` and
`GIANTS_KABUTO_TRANSITION_RECEIPT`; neither control inputs nor receipts belong in
Git. This test does not admit a player or verify actual audio/rendering.

For separately prepared dynamic auxiliary ownership, with the same supplied root:

```powershell
cargo test --locked -p giants-game source_kabuto_extra_owns_binding_refuses_replay_and_marks_without_retirement --target x86_64-pc-windows-gnu -- --ignored
```

This checks actual class/specification preparation, the selected template-clock
query from owned channels/resources, parent/reference binding,
foreign and replay refusal, active-extra cleanup refusal, deferred marking,
second construction and Session clear. Marked actors remain owned/listed.
It creates no window or sound, launches no original executable and admits no
live Kabuto controller. Active-extra teardown and full transition scheduling
remain unverified.

For cumulative animation-cache reset/rebind and retained node-owner checks, use
the same two input variables and isolated target:

```powershell
cargo test --locked -p giants-game source_kabuto_cache_retains_bounds_and_node_owners_across_reset --target x86_64-pc-windows-gnu -- --ignored
```

This uses supplied data plus deliberate timeline/absent-slot variations in owned
memory. It checks warm reuse, retained bounds, source identity, failed refresh
and release without launching the original executable, a window or audio. It
does not execute the full world teardown or acknowledge runtime admission.

## Optional canonical timed-effect ownership check

With `GIANTS_FLICK_ROOT` set to the user-supplied installation and an isolated
GNU target, run:

```powershell
cargo test --locked -p giants-game source_timed_session_preserves_canonical_state_and_quarantines_services --target x86_64-pc-windows-gnu -- --ignored
```

This CPU-only source check observes actual Session state at a required retirement
service, creates the real child context, retains late failures, refuses recursive
and foreign calls, and checks clear/re-entry. Parent motion and time are supplied;
no window, audio service, original executable or live world admission is created.
Optional `GIANTS_FLICK_TIMED_PARENT_CONTROLS` selects the private retained
parent/child rows; positive and zero-delta variants are separate runs. The control
now checks detached canonical physical writes and affine invalidation/publication
as well; velocity/history and initial detached ordinary preparation remain supplied.
Keep those inputs and detailed receipts private. Passing is not film fidelity.

## Optional silent runtime lifecycle check

After building a preview, supply your own installation and a fresh private output:

```powershell
python -m tools.runtime_smoke --executable .local/build/giants-rs.exe --receipt .local/build/build-receipt.json --data-root 'D:/Games/Giants' --output .local/runtime-smoke-NEW
```

This launches two offscreen GPU processes without a window or audio device. Each
performs three development Mecc launches; one returns to the menu three times,
the other finishes in the third development session after two returns. The runner
checks exit status, fresh health/projectile markers, owned-entity cleanup markers,
endpoint and capture completion. A timeout (default 120 seconds per process) or a
missing checkpoint fails the run. The executable must match its build receipt.

Profiles, logs, captures and their checksums remain in the output directory; keep
them private and outside the original installation. They must never enter Git or
release assets. This optional check requires real game data and a GPU and is not
part of synthetic CI. A passing report does not validate manual input, campaign
mission loading, total memory reclamation, pixel fidelity or distribution rights.

Before extraction, run `python -m tools.verify_preview PATH_TO_DRAFT.zip` with its
adjacent `.zip.sha256` file. This checks package integrity and consistent draft/public review
state; it does not run the executable or confer legal clearance. See
docs/RELEASES.md for the complete local packaging sequence with the project license.

## Optional silent cinematic ownership check

```powershell
python -m tools.cinematic_smoke --executable .local/build/giants-rs.exe --receipt .local/build/build-receipt.json --data-root 'D:/Games/Giants' --output .local/cinematic-smoke-NEW --world Story1 --mission 0
```

This uses the integrated menu's selected-mission request and cancellation services
three times. It disables the audio device, creates no window and injects no input.
The runner checks distinct text-owner tickets, releases and a final menu capture;
its receipt must match the GNU executable. Mission indices are zero-based.
The device-disabled profile retains prepared audio assets for ownership checks
and queues no master/voice players. Its retirement receipts must confirm zero
players for all three tickets; a startup audio flag alone is insufficient.
Keep logs, profiles and captures private and separate from the installation.

Add `--persistent-cancel` on a mission with a bounded persistent opening effect
to wait for an actual owned object to advance twice before calling the same
cancellation service. This is an observation gate, not an effect duration or
native scheduling claim. The runner requires three distinct owner tickets and
matching object/context release receipts after each cancellation. It still
injects no input and does not validate interactive skipping. This mode is
separate from `--full` natural transport.

For an active-effect capture from the real menu/mission path, set
`GIANTS_MISSION_REPLAY=showcase-persistent`, `GIANTS_SHOWCASE_WORLD` and
`GIANTS_SHOWCASE_MISSION`, then launch with a private `--user-data-root`,
`--mecc-menu --offscreen --silent-capture --capture <private.png>`.
Optional `GIANTS_PERSISTENT_SAMPLE_SEQUENCE` selects an actual director identifier
in that mission chain. `GIANTS_PERSISTENT_SAMPLE_COMPLETION=1` requests the
existing `CompletionReplay` intent used by the menu's completion replay; it does
not complete objectives, acknowledge a win or reconstruct native replay state.
With an explicit sequence, optional `GIANTS_PERSISTENT_SAMPLE_MIN_TICK` supplies
a nonnegative i32 observation bound. Playback must reach that actual tick in the
selected director and evaluate its command cursor through that tick before the
observer may suspend for capture. The effects consumer must have processed the
same revision, and every live persistent object must have advanced twice. An
older object cannot acknowledge a newly dispatched peer. It does not seek,
set the clock or override an effect's lifetime. An unreachable bound produces no
sample acknowledgement. A reached tick alone does not prove native image phase.
The observer waits for an actual Session-owned persistent
effect to advance twice, records the reached director/tick and suspends that
playback for GPU capture. It creates no effect or input and sends no readiness
acknowledgement. Missing effects/sequences do not acknowledge a sample. This is
an inspection tool; frames still need source/phase attribution and comparison.

The sampled frame count is not proof of audio-clock progression, full playback,
visible-subtitle atlas cleanup, natural ending, skip input or gameplay admission.
Interactive verification remains explicitly pending during background-only work.

For natural-ending transport and second-entry checks, use the explicit muted
transport mode (real audio sinks, zero volume before creation):

```powershell
python -m tools.cinematic_smoke --executable .local/build/giants-rs.exe --receipt .local/build/build-receipt.json --data-root 'D:/Games/Giants' --output .local/cinematic-full-NEW --world Story4 --mission 0 --full --require-text
```

This performs two complete selected-opening cycles, waits for the source terminal
and drained voices, observes the real post-film endpoint, returns to the menu and
re-enters. It does not press keys or mouse buttons. The runner compares declared
director/voice/sync counts with observed terminals and sinks, requires zero sink
volumes, and rejects an unverified local audio tail. A voice may be observed empty
at retirement if it finished after that frame's audio update; a missing or active
sink cannot count as a finished voice. `--require-text` additionally requires
nonempty glyph pools and atlas release on both entries. Omit it for sources with
no text. The default full-run timeout is 1800 seconds; a timeout fails the check.
Passing this check establishes transport/lifecycle observations for the selected
opening only. It does not establish native cadence, audio mixing, skip gestures,
mission rules, other sources or visual fidelity. Preserve all outputs privately.

## CPU-only whole-corpus cinematic inventory

Use a private world/sequence TSV (one declaration per line, no header), a fresh
private output directory and a receipt-bound build. The manifest must cover the
installation's FLK directory exactly; the default expected count is 545.

```powershell
python -m tools.cinematic_inventory --executable .local/build/giants-rs.exe --receipt .local/build/build-receipt.json --data-root 'D:/Games/Giants' --cases .local/world-sequence.tsv --output .local/cinematic-inventory-NEW
```

This calls the real CPU scene preparers before creating an App, GPU, window or
audio service. It inventories all source commands, declared stream file presence,
main mission FX binding and final-consumer diagnostics. Specialized ownership
routes are not counted as missing geometry; identical repeated preparation
messages are deduplicated within each source. A truncated or incomplete run is
rejected. `sources.tsv`, `inventory.json`, the HTML report and raw logs remain
private. The separate `public-summary.json` contains only fixed cause labels,
aggregate counts and build identity; review that projection before publishing it.

Priorities put preparation barriers first, then order common causes by distinct
affected sources within each level. Diagnostic units overlap and are not counts
of missing rendered objects. A clean preparation result leaves visual quality,
native timing, whole-session playback and full audio/subtitle parity unverified.
This tool changes no cinematic behavior and does not require another gameplay
preview merely to publish an inventory summary.

The inventory distinguishes final ownership from preparation logs: specialized
attachment routes and admitted terminator-only FX are not missing geometry.
Strict-profile rejection is a review item when preview consumers prepare the
source. FLK and MDV opcode spaces are distinct; music commands inside MDV are
outside this batch's scope. Detailed output remains private, including cases with
no reported gap; all complete-scene fidelity fields remain unverified.

## Completion cinematic inspection

In the main menu choose **Preview tools → Completion cinematics**, then a world
and one of its source-declared completion entries. Choices retain their original
mission slot, even when the filtered list is shorter. The normal mission loader
prepares the declared film; no startup-name alias or slot-zero fallback is used.
Escape or double-click skips the film into the partial free-camera preview;
Escape returns to the list. A natural ending uses that same preview boundary.
This route does not mark campaign victory, restore original live replay state or
admit gameplay. Original data is read-only; preview unlock uses a dedicated profile.

The optional Kabuto classification controls bind the completed motion prefix to
source-backed physical fields, resolved radius bounds and the affine cache. They
supply prepared geometry/peer and terrain services, check late-failure quarantine,
and retain all initializer gates. They do not enable live world admission or a
second primary frame. Private native control inputs/receipts stay under ignored
local output and are not required by the portable synthetic suite.

The additional-contact source checks consume the real classified owner and raw
preceding reference buffers, test ordered Session action effects, and refuse
reentry/foreign/stale identities. They retain all initializer/history gates.
Prepared scene/action providers and private supplied-boundary PC controls are
separate from live world/ANM production, actual action execution and playability.


## Optional owned Kabuto resource-phase check

With `GIANTS_KABUTO_ROOT` and the isolated target described above:

```powershell
cargo test --locked -p giants-game source_kabuto_resources_publish_before_services_and_preserve_primary --target x86_64-pc-windows-gnu -- --ignored
```

This silent CPU control runs the actual owned cold/history/motion/contact/model/
ANM/gravity/control-force pipeline before the new guard/resource phase. It checks
current descriptor/damage/clock binding, real Session prefix publication and
provider mutations, retained late failures, missing lifecycle, cancellation and
replay refusal. Sound/network entry providers are explicit test code; it opens
no window, executes no device backend and enables no live mission controls.

The same control then exercises the read-only reference query against actual
Session masks, list membership and parent/category metadata. Its ordinary target,
declared visibility/category context, ray and analytic bounds are test inputs;
they do not establish current world bounds, target publication or live selection.
Foreign factories, stale frames and missing eligible bounds refuse explicitly.
It also composes actual owned size parameters with the acquired ray producer and
that guarded query. Mode/reference presence and analytic viewport remain supplied;
the check does not establish a live camera, target lifetime or primary completion.

## Optional Mecc damage-smoke preparation control

With `GIANTS_MECC_WORLD_ROOT` and an isolated GNU target directory, run:

```powershell
cargo test --locked -p giants-game source_mecc_damage_smoke --target x86_64-pc-windows-gnu -- --ignored --nocapture
```

This silent source-resource control checks all three prepared smoke levels,
reuse/recreation, actual context release and refusal/late-failure ownership.
Damage and the display-reset decision are supplied. It performs no active effect
update, original-process launch, window or audio; the full primary remains guarded.
See [OWNED_MECC_CONTACT.md](OWNED_MECC_CONTACT.md).

## Optional wounded Mecc primary controls

With `GIANTS_MECC_WORLD_ROOT` and the isolated target directory:

```powershell
cargo test --locked -p giants-game source_mecc_wounded_primary --target x86_64-pc-windows-gnu -- --ignored --nocapture
cargo test --locked -p giants-game source_mecc_impact_primary_voice --target x86_64-pc-windows-gnu -- --ignored --nocapture
```

The first command checks persistent prepared smoke during 384 supplied command
frames, owner/frame binding, cancellation and late-failure quarantine. The second
also checks the next wounded frame after an actual owned falling-entry impact
with required supplied cues. Floor, clocks, wounds/display snapshots and cue
providers remain controlled inputs. No active effect, device or normal mission
input is exercised. An optional `GIANTS_MECC_PRIMARY_RECEIPT` path writes a fresh
private normalized-state receipt; it must stay outside tracked/public artifacts.
See [OWNED_MECC_CONTACT.md](OWNED_MECC_CONTACT.md) for native comparison scope.

For the separate pre-wrapper constructor/list comparison, supply the private
`GIANTS_FLICK_TIMED_REGISTRATION_CONTROLS` and the same installed-data root:

```powershell
cargo test --locked --workspace native_timed_positioned_constructor_keeps_registration_order --target x86_64-pc-windows-gnu -- --ignored
```

This replays eight controlled class-personality cases against actual constructor
registration, including six that differ after wrapper stores. Installed files
are unchanged; this check does not admit native stock or a live world actor.

The `source_herd_frame` controls compose actual owned herd/marker fallback and
Vimp callbacks through the mutable common-list walker with supplied flat ground,
clock, control identity, quiet signals and empty collision/platform cohorts.
They retain same-step births, shared ANM epochs, affine publication, cancellation,
late environment writes and failure quarantine. They enable no live mission AI.

The `source_vimp_threat` controls invoke isolated consideration on owned Vimps
with supplied noise. They check remembered timestamps, unsupported linked state,
invocation ownership and retained writes/quarantine after a late finite-profile
failure. Response, actual world-noise production and primary admission stay open.


## Optional owned Mecc smoke callback check

With `GIANTS_MECC_WORLD_ROOT` and an isolated Cargo target, run:

```powershell
cargo test --locked -p giants-game source_mecc_smoke --target x86_64-pc-windows-gnu -- --ignored
```

This silent installed-resource check advances the real owned emitter, prepares
its deferred child program in the originating pool and checks strict-zero timing,
first-call delay, retained guide-failure prefixes, quarantine and bounded cleanup
after owned callbacks. The guide supplies an explicit world affine and the frame
supplies actor-local time. It exercises no active particle or renderer. See
[OWNED_MECC_CONTACT.md](OWNED_MECC_CONTACT.md#owned-emitter-callback--10-november-2026).

The same filter now checks canonical child activation after the separate null pass,
actual active draws and context retirement, first-call ANM and retained failure at
the required velocity/physical service. All three installed smoke child profiles
are checked. No moving particle or rendered smoke is implied.

The owned Mecc smoke filter also executes the concrete Session motion/gravity
adapter through finite expiry, checks its independently required physical clock
and retained launch/service failure, and rejects replay. It supplies no populated
terrain or renderer. The actor remains allocated after being marked dead.


For the silent expired-child destruction control, use the same installation and
isolated target:

```powershell
cargo test --locked -p giants-game source_mecc_expired_smoke --target x86_64-pc-windows-gnu -- --ignored
```

It verifies actual weak instance-owner release, peer and newer-context retention,
canonical count/list retirement and cleanup-phase/optional-service refusals. It
neither renders particles nor supplies a common-world cleanup schedule.


For source-backed canonical smoke draw packets and the App consumer's CPU asset
lifecycle, set `GIANTS_MECC_WORLD_ROOT` and `GIANTS_MISSION_OWNED_ROOT` to the same
user-supplied installation, keep the isolated target and run:

```powershell
cargo test --locked -p giants-game source_mecc_smoke_draw --target x86_64-pc-windows-gnu -- --ignored
cargo test --locked -p giants-app source_owned_smoke_draws --target x86_64-pc-windows-gnu -- --ignored
```

These silent controls check identity/admission, current source vertices/alpha,
no draw-driven simulation, actor expiry and Bevy mesh/material/image/buffer/entity
retirement. They create no graphics device and do not verify rendered pixels or
ordinary gameplay birth. The normal mission consumer creates no particles itself.

With the same `GIANTS_MECC_WORLD_ROOT`, the silent source-backed pistol action
composition runs through the actual owned wield factory:

```powershell
cargo test --locked -p giants-game source_mecc_pistol_draw --target x86_64-pc-windows-gnu -- --ignored
```

The group checks supplied-window hand replacement and peer preservation, plus
160-frame complete grounded primaries using actual ANM callbacks. The neutral
case reaches draw completion; the movement case interrupts it. A missing-material
control preserves completed state and refuses replay. Floor, commands, clocks and
empty populations remain supplied; no graphics device is created.

The optional `GIANTS_MECC_DRAW_RECEIPT` records selected Rust words for private
comparison. Use a fresh path under ignored `.local` and run only one of
`source_mecc_pistol_draw_interleaves` or `source_mecc_pistol_draw_interruption`
per receipt. These artifacts must stay private.

With the same installation and isolated target, the silent ordinary-idle control
executes 512 complete grounded primaries through actual ANM and owned services:

```powershell
cargo test --locked -p giants-game source_mecc_idle_primary --target x86_64-pc-windows-gnu -- --ignored
```

Its default supplied seed reaches an idle variant and returns to idle. Optional
`GIANTS_MECC_IDLE_SEED` selects another supplied 32-bit CRT seed; it does not
reconstruct native global startup. `GIANTS_MECC_DRAW_RECEIPT` can write a fresh
private selected-word receipt for this single test. Keep the floor, clock,
empty populations and nongraphical scope distinct from ordinary gameplay.

With the same explicit Kabuto root/executable, `source_kabuto_action_gate_`
continues 512 independent installed contexts through the retained-slot gate and
509 eligible actual animation commits. It keeps the three held-reference exits
separate and refuses foreign/replayed or non-commit requests. Optional private
`GIANTS_KABUTO_GATE_INPUTS` / `GIANTS_KABUTO_GATE_RECEIPT` export the selector/gate
projection; `GIANTS_KABUTO_RESOLVED_COMMIT_INPUTS` /
`GIANTS_KABUTO_RESOLVED_COMMIT_RECEIPT` export eligible animation contexts. All
paths must be absolute. No window or audio device is created. This does not
complete primary/history/initialization or validate live gameplay.

For the effect-free wounded primary, use the same installed Mecc root:

```powershell
cargo test --locked -p giants-game source_mecc_effect_free_wounded --target x86_64-pc-windows-gnu -- --ignored
```

This silent 512-frame control introduces supplied light damage, retains actual
health and an empty smoke binding, and needs no fabricated display snapshot.
An optional fresh private `GIANTS_MECC_DRAW_RECEIPT` includes the extra health and
smoke words for this test; do not share a receipt path with another control.

The App `source_owned_mecc_runtime_terrain_and_presentation` and
`source_owned_mecc_jet_with_reserved_zero_pcm_device` controls accept test-only
`GIANTS_MECC_RUNTIME_FRAMES` (128 through 4096, default 128),
`GIANTS_MECC_RUNTIME_THRUST_UNTIL` (default 48) and
`GIANTS_MECC_RUNTIME_BOOST_UNTIL` (default 33). Device thrust starts at frame 16
and boost at 32; endpoints are exclusive and must fit the frame limit.
Set `GIANTS_AUDIO_MUTE=1` for the zero-PCM device control. The baseline 512-frame
profile passes. Prolonged thrust/boost still reaches an unbound impact provider;
these knobs describe supplied diagnostic inputs, not new gameplay options.

## Combined graphical owned-inspection acceptance — preview .8

The receipt-bound .8 executable also runs the internal
`owned-player-inspection-cycles` replay through three 128-frame GPU cycles with
actual cleanup. The explicit owned backend and supplied replay commands remain
diagnostic inputs. A separate `owned-player-inspection` run with the reserved
backend exercises jet lift/fuel through 128 frames and captures active thrust at
supported step 36. Real `--muted-transport` installs the audio plugin at zero
master volume and requires the existing offscreen/silent/menu and
`GIANTS_AUDIO_MUTE=1` guards. Device-free silent capture still rejects reserved
owned audio. Use fresh private profiles/capture paths and await the GUI-subsystem
child as described above. These checks do not certify manual input, ordinary
world admission, native clocks or visual/audio fidelity. The render consumer
accepts a genuinely missing held pose during draw; stale present poses still fail.


With the same explicit Kabuto root/executable, `source_kabuto_primary_tail_`
runs three silent installed controls after the actual owned resource/action/
selection/ANM composition. They check typed links, generic marking, shared
reference-camera sampling, recorded world entries and retained error/removal
boundaries. The success control reports 509 admitted component contexts and
three refused held routes, not completed gameplay frames. Camera/UI/audio/rate/
transport rendering or delivery and link event producers remain unvalidated.

The shared completion-countdown phase uses synthetic tests without game data:

```powershell
cargo test --locked -p giants-game mission_completion_countdown --target x86_64-pc-windows-gnu
```

These cover temporary suppression ownership, wide expiry versus stored rounding
and retained stores after a failed second clock query. The service takes actual
caller-owned state and clock callbacks; it supplies neither mission admission
nor the following progression phase. Native comparison inputs remain private.

The same `mission_completion_countdown` synthetic filter also covers ordered
admission: precheck before later gates, record replacement/rereads and retained
markers after callback failure. The `admission::Services` methods are required
providers; the precheck is not a one-time initializer. Snapshot production,
actual callback/clock backends and common runtime ownership remain explicit
prerequisites; continuation orchestration is acquired separately below.

The same `mission_completion_countdown` filter includes continuation failure
controls: message-marker retention, post-message replacement increment and
post-resolution queue failure. `continuation::Services` requires actual storage,
text identities, UI/music/save/load/event/countdown-start providers. No method
has a default successful implementation; missing providers must remain errors.
The acquired caller return is separate from mission completion or live admission.

## Optional Kabuto initial-body inspection

This is an intermediate renderer dependency, not playable Kabuto. A compatible
user-supplied Kstory1 installation is required. The ignored App control
`source_kabuto_menu_entry_constructs_owned_group_and_reenters` uses
`GIANTS_KABUTO_ROOT`; it checks the actual menu request/common constructor,
current-channel body preparation, four pending initializers and cleanup/re-entry.

For private GPU inspection set `GIANTS_MISSION_REPLAY=showcase-kabuto-body`,
`GIANTS_SHOWCASE_WORLD=Kstory1` and `GIANTS_SHOWCASE_MISSION=0`. Launch an awaited,
receipt-bound common executable with `--mecc-menu`, your `--data-root`, a fresh
`--user-data-root`, and `--offscreen --silent-capture --capture` to a fresh private
PNG. This explicitly activates the existing environment inspection camera and
suspends opening playback without emitting mission readiness. The body is an
initial snapshot; native camera/projection, live ANM, input and ordinary startup
remain unverified. Inspect the actual image: GPU preparation and a successful PNG
write alone do not prove body visibility or correct framing.

## Optional Delphi initial-body inspection

This is an initial body snapshot, not playable Delphi. The installed App control
`source_delphi_menu_entry_constructs_declared_group_and_retains_player_role` uses
`GIANTS_DELPHI_ROOT` and checks body ownership, untouched cold caches, retirement
and a second ticket. The shared renderer preserves known absence of normals and
uses its existing no-normal route; normal-declaring preparation errors still fail.

For private GPU inspection use `GIANTS_MISSION_REPLAY=showcase-delphi-body`,
`GIANTS_SHOWCASE_WORLD=Rstory1` and `GIANTS_SHOWCASE_MISSION=0`. Run an awaited,
receipt-bound common executable with `--mecc-menu`, your `--data-root`, a fresh
`--user-data-root`, and `--offscreen --silent-capture --capture` to a fresh private
PNG. This diagnostic suspends opening playback and uses the declared-start
inspection camera. It acknowledges no projection reset, normal input, live ANM,
held/copy placement or gameplay readiness. Inspect the actual image separately
from pipeline completion. The source-backed menu request uses a test preview
profile; it does not prove ordinary campaign unlocks.

With `GIANTS_DELPHI_ROOT` and an explicit `GIANTS_DELPHI_EXECUTABLE`, run
`cargo test --locked -p giants-game source_delphi_projection_checkpoint --target x86_64-pc-windows-gnu -- --ignored`
for ordered cold activation. It uses actual resources and external camera
coefficients with supplied mode/current-owner inputs and a projection-request
sink. It checks unchanged pre-projection history, later history/children/animation,
retained projection failure and retained late special failure. No App device,
startup, real input or native exception equivalence is established.

### Kabuto owned camera-header control

With a user-supplied installation in `GIANTS_KABUTO_ROOT` and a compatible inspected
PC executable in `GIANTS_KABUTO_EXECUTABLE`, run:

```powershell
cargo test --locked -p giants-game --target x86_64-pc-windows-gnu source_kabuto_camera_command_updates_exact_owned_look_angle -- --ignored
```

This is a silent CPU control of actual constructed-owner storage, external cold
camera words, current profiles and the projection kernel. It checks owner
isolation, retained effects on failure and retirement. Loader-image state does
not establish actual mission-entry/warm state, device publication or gameplay.


### Explicit Kabuto profile-camera inspection

This source diagnostic is not ordinary Kabuto gameplay and is not in previews .9 or .10.
With a compatible user-supplied PC executable selected by
`GIANTS_KABUTO_EXECUTABLE`, set:

```powershell
$env:GIANTS_MISSION_REPLAY = "showcase-kabuto-camera"
$env:GIANTS_SHOWCASE_WORLD = "Kstory1"
$env:GIANTS_SHOWCASE_MISSION = "0"
# GIANTS_KABUTO_EXECUTABLE must name the compatible executable from your installation.
```

Run the built application with `--data-root` pointing to your game folder,
`--mecc-menu`, a fresh `--user-data-root`, `--offscreen --silent-capture` and a
`--capture` PNG path. The actual menu request/spawn and mode-1 service supply the
owner/profile/look angle/size inputs. The shared renderer consumes the unclipped
position and projection kernels. Floor/chase/walls, actual startup/warm ordering,
live ANM/controls, placement fidelity and initializer admission are excluded.

The silent CPU lifetime control requires the same installation/executable:

```powershell
cargo test --locked -p giants-app --target x86_64-pc-windows-gnu source_kabuto_inspection_camera_uses_owned_mode_profile_and_reenters -- --ignored
```

### First-living component controls (included in preview .11.2)

With a user-supplied installation selected by `GIANTS_NPC_ROOT`, run:

```powershell
cargo test --locked -p giants-game --target x86_64-pc-windows-gnu source_vimp_living_noise_cycles_consume_common_frames_and_effects -- --ignored --nocapture
cargo test --locked -p giants-app --target x86_64-pc-windows-gnu source_normal_vimp_component_uses_player_clock_noise_and_reentry -- --ignored --nocapture
```

The first control supplies admission, terrain, clock/control/noise/session and an
absent sound bank. The second uses actual Story2 resources/terrain and the normal
owned partial runtime, with supplied host timestamps and Mecc commands; it queues
audio without a device or OS input. Neither certifies native complete-world
admission or GPU fidelity. The optional private native selector comparison also
requires `GIANTS_NPC_NOISE_ANIMATION_CONTROL` pointing to a reviewed private
hex-case file; raw controls and original data stay outside Git.

### Guarded first-living graphical diagnostic

For a supplied-command GPU/audio check of the connected Story2 component, set
`GIANTS_MISSION_REPLAY=showcase-mecc`, `GIANTS_SHOWCASE_WORLD=Story2` and
`GIANTS_MISSION_VIMP_PROBE=1`. Run with `--offscreen --silent-capture
--muted-transport` and a fresh user-data directory. This guard requires muted
transport. The probe navigates toward the actual nearest Vimp using ordinary
Mecc look/movement/jet/nitro/fire commands; it does not overwrite actor positions,
poses or readiness. Capture waits for actual startle entry and return counters.
This is supplied-command graphical evidence, not OS-held input or native
complete-world scheduling equivalence. Normal Story2 controls enable the same
partial living component without these diagnostic environment variables.

### Connected Vimp combat candidate

With `GIANTS_NPC_ROOT` set to the compatible user-supplied installation, run:

```powershell
cargo test --locked -p giants-app source_normal_pistol_kills_vimps_and_runs_all_remains --target x86_64-pc-windows-gnu -- --ignored --nocapture
cargo test --locked -p giants-app source_normal_120hz_vimp_hunt_recovers_and_retires --target x86_64-pc-windows-gnu -- --ignored --nocapture
```

This exercises actual partial-pistol hits, injuries/recovery, source death FX
and meat/proxy/ghost expiry on two supplied-navigation/time entries. Its external
retirement provider is a test boundary, not a real audio/GPU acknowledgement.
The normal application binds the actual owned device and common render consumers.
Run all `mission_runtime::npc::tests::` installed controls for pose/contact/render
asset/re-entry regressions. Then repeat ordinary menu/left mouse hit/right mouse
jet/Space nitro and return/re-entry on the exact executable with actual sinks.
Do not close the delivery on these CPU controls. FX host scheduling, terrain-only
contact and splat retention remain explicit preview policies in OWNED_NPCS.md.

The many-actor graphical Vimp probe advances one fixed step per rendered host
frame so asynchronous sink creation can run between steps. It does not increase
the normal transport's bounded backlog or acknowledge missing sinks. Adding
`GIANTS_MISSION_VIMP_DEATH_CAPTURE=1` to that graphical probe ends its supplied
controller sixteen steps after the first actual lethal hit, for a remains/effect
capture. It neither enables combat nor creates a synthetic hit or actor pose.
Keep the full lifecycle and cancellation probes as separate checks.
For an independent contact passage, set `GIANTS_MISSION_VIMP_CONTACT_PROBE=1`
instead of the death-capture flag. The supplied controller stops firing, lands
near the herd and continues walking toward an actual Vimp. Its 12,000-step
endpoint requires real player contact responses; no collider or pose is injected.
