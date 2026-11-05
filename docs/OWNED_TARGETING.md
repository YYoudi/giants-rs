# Owned local target publication

`Session::scan_mecc_local_target` now runs the existing local query and publishes
its result into the owning normal Mecc. The numerical-only query remains available
separately. Header context, special-header values, camera and viewport snapshots,
current collision geometry and vertex epoch are still explicit required inputs.

The published state contains three typed relationships, the aim point, the
terrain-proximity flag and the supplied vertex epoch. Original actor pointers are
never encoded as runtime identities in the payload. The three pointer slots stay
normalized to zero; the aim-point words preserve their exact floating-point bits.
The native self-result rule clears the target. A later miss clears the previous
relationships and publishes the new point and proximity flag.

Incoming-reference inventory includes these target links so retirement cannot
silently orphan a referenced actor. Ticket cancellation clears the local reticle
owner and all actor state. Foreign identities, an unavailable source/profile,
unnormalized pointer slots, invalid epochs and nonfinite points are refused.
A query/provider failure does not replace an earlier target publication. Geometry
preparation is a separate service and is not described as a transaction rollback.

The existing no-target fire callback also checks typed target state: normalized
bytes cannot disguise a selected actor as a null-target firing profile. The actual
local-target firing branch remains unacquired and is refused.

## Evidence and limits — 5 November 2026

Static REA inspection located the publication immediately after the local query.
A private controlled execution then compared the original local prepass with
source-backed Rust constructors, current poses/bounds, the Session query and the
new publication. All 64 cases matched: 1,085 compared words, 16 selected targets,
96 geometry visits and 29 mesh visits. The effective declaration range remains
700; the independent normal-pistol caller distance remains 500.

A further 128 Rust error-preservation checks covered a missing header provider
and zero vertex epoch after a successful scan, retaining payload, relationships
and reticle state. Portable tests cover replacement, misses, self exclusion,
signed zero, payload footprint, incoming references, foreign identities and
cancellation. These backend refusal checks are not original error-path parity.

The native comparison used four actor constructors and supplied context/view/
clock inputs. It does not establish complete actor coverage, mission context
assignment, correct live camera timing, a primary/fire scheduling boundary,
manual input, damage, rendered reticles or complete playable missions. No new
user-facing firing capability or binary preview is claimed by this source step.
The main mission driver continues to reject unsupported firing commands.

All 691 synthetic Rust tests passed (14 ignored), with 18 Python tests, Rustfmt,
Clippy and the GNU Windows release build. Both silent three-launch menu/arena
lifecycle regressions passed on runtime source
`66074d4c6607fccd5ac3d1562abff7ec1942c5c8`. These checks do not exercise a live
target scan. A local draft includes the receipt-bound executable, complete
notices, manifest and checksums and passes the independent package verifier.
It is retained locally; the current downloadable preview remains unchanged.

The combined main build at
`7fd24444f41c4dcbae9fc5c18e1a72690f678a96`, including the independently developed
cinematic changes, subsequently passed 694 Rust tests (14 ignored), 19 Python
tests, Rustfmt, Clippy and the GNU Windows release build. Both silent
three-launch menu/arena regressions passed again. The cinematic regression also
passed three fresh text-owner releases and real cancellations, with no panic or
injected skip. These lifecycle checks do not establish live targeting or visual
fidelity.

The portable publication regressions can be run without game data:

```powershell
cargo test --locked -p giants-game --target x86_64-pc-windows-gnu local_target_publication
```

The class-loader investigation also identified initial template zeroing and a
class-program context assignment. That narrows the next producer investigation;
current context inheritance and mission mutations must be represented and
validated before replacing the required header service. The special-header
producer remains open. BACKLOG.md is the detailed task list.

Private REA evidence identities: local primary
`ev_4cbaf1fe40899a74005249b38e09aceed0a0426287582f4ec7302085179138a1`;
configuration initializer
`ev_4cef658d3f280bd727873102358aa1ffabdb9c7292c44e404ec54c2f3a2194e1`;
class loader
`ev_e7665d5d89f2fd02d1ed1c1803a4207f92f532572a9ff4dfac058c285105c6d6`.
These identify static analysis, not execution. Original instruction listings,
artifacts, paths and controlled-run inputs remain private and are not exported
with the source or dashboard. This is technical evidence, not rights clearance.
