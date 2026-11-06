# Build on Windows with MinGW

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

Default tests use synthetic inputs without installed game data. Ignored tests
requiring external data or audio devices are not part of this portable check.
For an optional shared cache set GIANTS_CACHE_ROOT. To add the MinGW bin directory
set GIANTS_MINGW_BIN. Otherwise normal Cargo/Rustup settings and PATH are respected.
Use a separate Cargo target directory for each active worktree. Sharing downloaded
registry/toolchain caches is fine; sharing build artifacts between concurrently
modified worktrees can produce a test inventory from the wrong source. Reject
that result and rebuild in an isolated target before recording a release receipt.

Package a draft with `python -m tools.package_preview --help`. Run the extracted
bundle with `./Play.ps1 -DataRoot 'D:/Games/Giants'`. Fonts are loaded from Windows
or GIANTS_INSPECTION_FONT, never bundled. Game data is read-only; user state goes
outside the installation. Interactive coverage is limited to the exact preview
and mission recorded in RELEASES.md; broader gameplay and fidelity remain pending.

## Optional subjective Mecc lifecycle check

Set `GIANTS_MECC_WORLD_ROOT` to a user-supplied installation and use the isolated
target directory described above:

```powershell
cargo test --locked -p giants-game source_mecc_subjective_publishes_before_fire_and_retains_late_failure --target x86_64-pc-windows-gnu -- --ignored
cargo test --locked -p giants-game source_mecc_deferred_primary_requires_post_and_retains_real_fire_failure --target x86_64-pc-windows-gnu -- --ignored
cargo test --locked -p giants-game source_mecc_produced_camera_orders_post_fire_and_outer_publication --target x86_64-pc-windows-gnu -- --ignored
```

This silent CPU control checks the Session-bound subjective resource and shared
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

This silent CPU control checks two owners, persistent motion/contact/jump, shared
class events, physical-only and locomotion profiles using external clip definitions,
late secondary quarantine, profile refusal and cleanup. It creates no window or audio device
and does not execute the complete Delphi primary/secondary or live controls.

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

For cumulative animation-cache reset/rebind and retained node-owner checks, use
the same two input variables and isolated target:

```powershell
cargo test --locked -p giants-game source_kabuto_cache_retains_bounds_and_node_owners_across_reset --target x86_64-pc-windows-gnu -- --ignored
```

This uses supplied data plus deliberate timeline/absent-slot variations in owned
memory. It checks warm reuse, retained bounds, source identity, failed refresh
and release without launching the original executable, a window or audio. It
does not execute the full world teardown or acknowledge runtime admission.

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
