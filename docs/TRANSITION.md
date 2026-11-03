# Working-repository transition

Destination: `YYoudi/giants-rs`, fresh history. The owner has now authorized the
agent to finish the source review and make this candidate public. The AI-assisted
technical review found no concrete source-publication blocker within its stated
scope; see PUBLICATION_REVIEW.md. This is not legal clearance. MIT applies within
NOTICE.md's scope. Binary distribution review remains open.

The private staging remote already uses the intended destination name. After
explicit owner approval, recheck that remote and push the reviewed candidate if
needed before changing visibility. Use it directly for all runtime development,
roadmap updates and releases after cutover.
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

The owner asked the agent to perform the verification and then make the repository
public. The documented source review is now complete within its technical scope.
The cutover will fast-forward reviewed source into main, verify a clean clone of
the private remote, then change this repository's visibility and activate its
asset-free Pages dashboard. Binary approval remains unset and the draft packager
still refuses public packages. The archive and all of its worktrees stay in place.
