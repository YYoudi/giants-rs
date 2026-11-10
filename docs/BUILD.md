# Build on Windows with MinGW

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

Default tests use synthetic inputs without installed game data. Ignored tests
requiring external data or audio devices are not part of this portable check.

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

For NPC source-resource ownership regression, set `GIANTS_NPC_ROOT` to the
user-supplied installation and use an isolated Cargo target:

```powershell
cargo test --locked -p giants-game source_npc_herd --target x86_64-pc-windows-gnu -- --ignored --nocapture
cargo test --locked -p giants-game source_vimp_world_history --target x86_64-pc-windows-gnu -- --ignored --nocapture
cargo test --locked -p giants-game source_vimp_newborn --target x86_64-pc-windows-gnu -- --ignored --nocapture
```

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
