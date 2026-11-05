# Build on Windows with MinGW

Install Rust with the GNU Windows toolchain (Rust 1.98 or newer), Python 3.11+,
Git, and x86-64 MinGW-w64 GCC/binutils. Put their bin directories on PATH.
Visual Studio is not required. The lockfile pins dependency versions.

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
outside the installation. Interactive validation remains pending in this candidate.

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
Keep logs, profiles and captures private and separate from the installation.
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
