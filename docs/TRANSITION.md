# Working-repository transition

`YYoudi/giants-rs` is now public and is the primary runtime, roadmap and release
authority, following the owner's explicit instruction. The AI-assisted technical
review found no concrete source-publication blocker within its stated scope; see
PUBLICATION_REVIEW.md. This is not legal clearance. MIT applies within NOTICE.md's
scope. Binary distribution review remains open.

The previous private repository remains a research archive with its history and
visibility preserved. Do not mirror ongoing runtime changes into two authorities.

Existing discussions must move explicitly to new checkouts and codex/ branches.
Do not relocate, reset or delete their existing worktrees. Preserve outstanding
patches and private evidence in the archive; review and port only independent
source changes against this baseline. Never merge the old Git history here.

Candidate baseline preserves recent main source plus uncommitted owned-target
Rust implementation. Unmerged cinematic/actor-reference work is retained in
private backup and per-discussion handoffs, not silently marked integrated.
Research observers, CPU replay examples and reference inputs stay private.

## Cutover sequence (only after approval)

1. Recheck all worktree heads and dirty-file hashes against the private handoffs.
   Back up and review changes made since the candidate freeze.
2. Apply the owner's chosen source license where the owner can grant rights;
   retain dependency notices and resolve release blockers separately.
3. Recheck the existing private destination, push the reviewed candidate if needed,
   and validate a remote clean clone before any approved visibility change.
4. Create explicit new discussion checkouts/branches from that destination.
   Reapply reviewed pending patches; keep originals available for comparison.
5. Update persistent tracking in both repositories and each discussion's handoff.
   The new repository becomes the only runtime, roadmap and release authority.
6. Publish a build only after its manifest, notices and distribution review pass.
   Keep research documents and original data outside public history and assets.

## Preparation checkpoint — 3 November 2026

The following records the earlier private preparation, before the owner's later
explicit publication instruction. It does not override that later authorization.

Observed: GitHub reports `YYoudi/giants-rs` as PRIVATE. Both baseline CI workflows
passed for `0d43e84c1dd61b344174786ac44f3bd5f286f7f5`; see docs/AUTOMATION.md
for run links and skipped publication steps. Approval variables were absent.
No visibility change, push, release or hosted deployment is authorized by this
checkpoint. Current preparation remains on a local `codex/` branch.

Implemented: draft bundles now include the license-scope NOTICE.md. An independent
ZIP integrity verifier checks the member allowlist and all recorded checksums
without extraction or execution. Hosted verification of this later change remains
pending until a separately authorized push.

Unknown/pending: comprehensive human provenance review, the missing constgebra
license text and exact compiler/runtime notice applicability. Interactive checks
remain pending under the background-only instruction. No whole-game roadmap
status changes follow from CI or packaging checks.

### Local validation of the preparation commit

Source/build identity: `8f7c3659bfc1fa87c93115359a23fdfeb27bec3a`, Rust 1.98.0,
GNU Windows x64, MinGW GCC 16.2.0. No runtime source changed relative to the hosted
baseline. Local results:

- Synthetic Rust tests: 684 passed, 14 ignored, none failed. Ignored tests were
  not enabled; no original game data or audio devices were supplied.
- Python: 10 tests passed, including tampering/allowlist rejection and preservation
  of LICENSE plus NOTICE.md. Rustfmt, Clippy with warnings denied, generated roadmap
  consistency, dashboard export and JavaScript syntax checks passed.
- Optimized Windows build and CLI `--help` passed. The help path returns before
  Bevy initialization; this is not a menu, gameplay or rendering check.
- Draft packaging and independent ZIP verification passed: 10 allowlisted files,
  matching receipt/executable, manifest, internal SHA256SUMS and external checksum.
  The package remains uncleared with all three recorded distribution blockers.

Private local output: `.local/preview-2026-11-03-transition/`. The dependency
metadata and build logs remain ignored and are not release members.

Executable SHA-256:
`0ed83d501d5cfe94ada27d672c55134aa82e911018019ebaa02d5aa9c4c48690`

Draft ZIP SHA-256:
`02bffa22230c6fdcc201fec32faeb9dd5e872d522699e73d226c2de52bb972c8`

These hashes identify this local build only. The later documentation checkpoint
does not change its recorded source identity. No push or remote setting was changed.

## Authorized source cutover

Completed on 3 November 2026, after the owner asked the agent to perform the
verification and then make the repository public:

- Reviewed `dc99be9948b4dfcf6ead88d7edf38b947b0dafee` was pushed to private main
  first. A new remote clone passed all 12 Python tests, generated-roadmap consistency,
  site export and JavaScript syntax validation. Its full history scan covered 10
  commits / 560 distinct file blobs with zero selected-hazard findings.
- [Hosted source/dashboard validation](https://github.com/YYoudi/giants-rs/actions/runs/41724991825)
  passed on that exact revision before exposure. Runtime Rust source and its
  dependency lockfile are unchanged from the previously tested baseline.
- GitHub visibility was changed to PUBLIC under the owner's explicit authorization.
  The private research repository was checked and remains private. All 12 existing
  archive worktrees remain in place; dirty-file hashes were recorded privately.
- Pages uses Actions with `PUBLIC_SITE_APPROVED=true` and a main-only deployment
  environment. The [first public dashboard deployment](https://github.com/YYoudi/giants-rs/actions/runs/41725146683)
  succeeded. Repository homepage and README point to this new dashboard.
- The latest Windows CI is tracked in Actions; a running/cancelled newer run is not
  a substitute for a completed exact-revision result. The baseline Windows workflow
  and local Rust tests, Clippy and optimized build have passed as recorded above.
- `PUBLIC_DISTRIBUTION_APPROVED` remains unset. The draft packager refuses public
  packages and no GitHub executable release has been published.

Subsequent documentation commits record the completed operation without changing
the already recorded executable/package identities. The old discussions require
explicit new runtime checkouts; none was moved automatically.

## Recovery continuation — 4 November 2026

Observed: public main `b98acbebbbd3254a679558554594546c9387cf1a` completed
[Windows MinGW CI](https://github.com/YYoudi/giants-rs/actions/runs/41725413930)
successfully, including synthetic checks, optimized compilation, draft packaging
and ZIP integrity verification. Artifact upload was skipped; no artifacts were
attached to the run. The corresponding dashboard workflow also succeeded.

Implemented: `tools.runtime_smoke` provides receipt-checked silent offscreen
development-session lifecycle checks with private logs, profiles, captures and a
path-free result report. It refuses output in the original installation or an
existing output folder and records failure on timeout or missing checkpoints.

Observed locally: both three-launch replay endpoints passed on the transition
executable identified above (`8f7c3659bfc1fa87c93115359a23fdfeb27bec3a`, SHA-256
`0ed83d501d5cfe94ada27d672c55134aa82e911018019ebaa02d5aa9c4c48690`).
One ended at the menu after three returns; the other ended in development gameplay
after two returns. Fresh-state and owned-entity cleanup markers, render capture
completion and zero process exit were checked without a window or audio device.

Local synthetic checks: 684 Rust tests passed, 14 ignored; Python checks,
Rustfmt and Clippy passed. No ignored data/audio tests were enabled. The runtime
Rust source and dependency lockfile are unchanged. The stability roadmap remains
partial. Campaign mission loading, manual controls, total memory reclamation and
visual fidelity remain unverified; binary distribution blockers remain open.

### Validated continuation build

The new local build receipt identifies
`63ef034d3d326e1122d3c13f09ecc71f4e53d313`. Its executable hash is unchanged
from the transition executable above, as no runtime Rust or lockfile changed.
Both silent development lifecycle modes passed again with this new receipt.
All 15 synthetic Python tests passed; the generated roadmap/site and JavaScript
syntax checks passed. The reachable-history scan examined 12 commits / 577 blobs
and reported zero selected-pattern findings (not provenance or rights clearance).

The refreshed private draft ZIP contains 10 allowlisted files, preserves the
license-scope and dependency notices and passes independent checksum verification.
Its SHA-256 is
`736c7062bc0c3d97842fbc073e2ebab7f60a7d2b0fc0029d56909c9af8edc35d`.
The manifest remains uncleared with human provenance/release review, compiler
runtime applicability and constgebra 0.1.4 notices unresolved. No binary was
uploaded or published. This continuation is retained on a local `codex/` branch;
the hosted main CI result above certifies `b98acbe`, not this later tooling change.
