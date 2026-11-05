# Owned local target publication

`Session::scan_mecc_local_target` now runs the existing local query and publishes
its result into the owning normal Mecc. The numerical-only query remains available
separately. The header service remains explicit; `OwnedHeaders` now obtains
current context from the acquired Session producers. Special-header values,
camera and viewport snapshots, current collision geometry and vertex epoch
still require their actual providers.

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

## Owned context producers — 5 November 2026

The class loader's group initialization establishes context zero, then ordered
context commands replace it. `Fields::load` now owns this value and ordinary
instantiation/placement preserve all its bits. This is measured initialization,
not a default for an unavailable current provider. Mecc special children inherit
the actual parent context instead of retaining their class contexts. The acquired
cold generic-to-player path retains its zero context; the held-object profile
continues to require that context.

The acquired Ripper primary now starts from the owned context and publishes its
context changes alongside its other fields. Source snapshots used by firing and
surface-marker services also carry the actual context. Basic projectile
construction and enemy launch preserve their explicitly supplied dispatch context
in the header. These explicit launch inputs do not acquire new source/fire timing.
Vimp and chunk constructors reject disagreement between their context owners.

`Session::mecc_target_header` and `OwnedHeaders` read the resulting state. Foreign,
raw or cancelled registrations, a running/failed primary and inconsistent owned
context copies are refused. The class805 special header remains unavailable:
the query still refuses a same-context candidate when that value is required.
The special actor-header value is separate from the target aim point in the Mecc
payload; no value is inferred from that payload.

Controlled native/Rust checks match 64 ordered context-command cases and 64
ordinary-constructor/three-child inheritance cases, comparing 384 scalar values.
Separate native group-initialization controls cover 160 class contexts. The local
scan comparison also passes its 64 cases / 1,085 words with the production
`OwnedHeaders`, including 16 selections and 128 failure-preservation checks.
Those scan cases supply the current class-context values, view/pose and clock
inputs; they do not prove complete mission mutation coverage or scheduling.
The main mission driver still does not execute the local scan or its fire phase.

Additional private REA identities: ordinary constructor
`ev_78f6054ad4b75e537feedb45437386dbf02474e4b148c0a0bc67199a800714b3`;
second field copy
`ev_abe5ec8143e50afd57f0be8e046327e3bcd09c98d687b10d3d2c7e74371c031c`;
child/type inheritance
`ev_d13386ab3ac355c0a7644636240d9bc509e4a22d01eb2bd66554f07330fa58cc`.
These identify static inspection; the controlled execution comparisons are
separate evidence. All original instructions and probe inputs remain private.

Runtime source `9211e93b882451ae9e8a8f67d1874c0d45ddc7e2` passes 695 synthetic
Rust tests (14 ignored), 19 Python tests, Rustfmt, Clippy, site export/JavaScript
validation and the GNU Windows release build. Both silent three-launch menu/arena
checks and the three-cycle cinematic opening/cancellation check pass on that
receipt-bound executable. These regressions do not execute the local target scan.
Pages validation/deployment passed; the hosted Windows build was still running
when these local results were recorded.

An asset-free local draft has 10 allowlisted members, complete notices and no
package-verifier blockers. Its executable SHA256 is
`2fb9eceaa5565f6dcf92685696e49b27c3f231959fb25c012140685a35fd9537`;
archive SHA256 is
`97b1034abce3ceec6d071462f9e76917b26529ef0e169fbd2fd9ebd3c52be761`.
It remains a draft, with no new accessible weapon and no new public download.

## Publication evidence and limits — 5 November 2026

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

The owned producers above replace the earlier requirement for a fixture-only
context service. Complete mission mutation coverage, the special-header producer
and the actual primary/camera/muzzle/fire boundary remain open. BACKLOG.md is the
detailed task list.

Private REA evidence identities: local primary
`ev_4cbaf1fe40899a74005249b38e09aceed0a0426287582f4ec7302085179138a1`;
configuration initializer
`ev_4cef658d3f280bd727873102358aa1ffabdb9c7292c44e404ec54c2f3a2194e1`;
class loader
`ev_e7665d5d89f2fd02d1ed1c1803a4207f92f532572a9ff4dfac058c285105c6d6`.
These identify static analysis, not execution. Original instruction listings,
artifacts, paths and controlled-run inputs remain private and are not exported
with the source or dashboard. This is technical evidence, not rights clearance.
