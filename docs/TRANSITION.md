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

## First downloadable Windows preview — 4 November 2026

The owner removed the mandatory human-release-review gate and explicitly
authorized Windows downloads. AGENTS.md and the release policy now preserve that
decision. constgebra's declared Apache alternative, its upstream declaration and
the full Apache license are included; conservative GCC/MinGW notices and Rust's
complete standard-library inventory accompany the executable. The technical basis
is recorded in DISTRIBUTION_REVIEW.md; no comprehensive human audit is claimed.

[preview-2026.11.04.1](https://github.com/YYoudi/giants-rs/releases/tag/preview-2026.11.04.1)
is public, with the Windows ZIP and two equivalent external checksum files.
Its tag and build receipt identify `57ed1de83985122ee1ca7171b3b78501182eebac`.
Concurrent authorized dashboard/gallery changes were merged without moving any
other checkout. No gallery images or original game data entered the ZIP.

- Executable SHA-256: `7c3d96dae3fe49f028c71f458b1535fc47700d18ed8f2ee7b7d896bf831c4fc2`.
- ZIP SHA-256: `05456e2834880a6163114e44a21064491e477465598c3e6233ffe1b20cee8b0d`.
- Final ZIP: 47,118,410 bytes, 10 allowlisted members, zero manifest blockers.
- Anonymous download returned HTTP 200 and matched the local ZIP hash and
  published SHA256SUMS. The release tag resolves to the recorded source commit.
- All 18 Python tests passed, also from a fresh public clone. Rust checks remain
  684 passed / 14 ignored, with Rustfmt, Clippy and optimized GNU build passing.
- Both silent development lifecycle endpoints passed from the final extracted
  package outside the source folder. Manual controls, campaign mission loading,
  full-session stability and fidelity remain unverified.

The exact-revision dashboard workflow succeeded. The newly triggered Windows
workflow was still running at publication; this preview relies on the recorded
local checks, not a claim that the later hosted build has finished. Linux and
macOS packages remain in BACKLOG.md for future platform work.

## Drop-in desktop preview — 4 November 2026

The owner requested a distinct giants-rs.exe that can be placed beside the
original game executable and started without a command line. The desktop entry
now checks adjacent game data (including an executable in Bin), falls back to a
native installation picker and stores logs/configuration outside the installation.
The separate Preview All profile unlocks the installed mission catalog, retains
existing profiles and does not import or overwrite original saves. Preview tools
offers clickable mission browsing, the development Mecc arena and folder changes.
The initial window is 1280x720 and resizable; the release uses the Windows GUI
subsystem instead of opening a console.

[preview-2026.11.04.2](https://github.com/YYoudi/giants-rs/releases/tag/preview-2026.11.04.2)
is public, tagged at `2d9529ddbf866584b5535304598c40c74c9569a1`. The previous
preview remains unchanged. The new archive contains 10 allowlisted members,
including giants-rs.exe, documentation, notices and checksums; no original assets
or gallery captures are bundled. Download verified anonymously: HTTP 200,
47,069,741 bytes, matching published SHA256SUMS.

- Executable SHA-256: `b1fdb81cda210c047e210abd14e46885143d8a3a6ef6ec6008139b45da00cdda`.
- ZIP SHA-256: `ed0e99ed691b4fcf3821b8244d33063e51d316538914f851a4a45ddcc6f80baa`.
- Synthetic checks: 687 Rust tests passed / 14 ignored; 18 Python tests passed,
  also from a fresh public clone; Rustfmt, Clippy and optimized GNU build passed.
- Interactive functional checks observed adjacent no-argument startup, default
  preview progress (56 entries for the supplied installation), tools opening,
  all-world browsing, arena launch/Escape return and native picker cancellation.
  A private junction-based test layout preserved the original installation.
- Both silent three-launch lifecycle modes passed from the final extracted ZIP
  outside the source folder. The older preview's ZIP still passes the verifier.
- The full-history selected-pattern scan covered 24 commits / 1,154 blobs with
  zero findings, including the exact authorized gallery exception. This is not
  a rights determination. The locked dependency graph and runtime notice set did
  not change; new folder selection uses Windows system APIs.

The exact-revision dashboard workflow passed. Hosted Windows CI was still running
when the package was prepared; local checks identify the released executable.
Game menus and stability remain partial: unlocked selection is not completed
campaign gameplay, restored original inventory or verified fidelity. Broader
edition, settings, input and full-session coverage remains in BACKLOG.md.

## 5 November 2026 — preview pistol terrain occlusion

The preview pistol's object query now uses the terrain-clipped movement segment,
matching the existing owned-flight ordering. The development arena applies the
same limit to its static synthetic target. Objects beyond a terrain stop can no
longer replace that stop; objects before it still receive contact. The shared
native-compatible object-query scratch semantics are unchanged. This correction
does not connect the separately gated owned firing/targeting services.

The behind-terrain regression fails on the previous unbounded segment and passes
with the correction. The before-terrain control and existing pistol tests pass.
All 689 synthetic Rust tests passed (14 ignored), plus 18 Python tests, Rustfmt,
Clippy and the GNU Windows release build. Silent real-data GPU replays passed
both three-launch menu/arena lifecycle endpoints, four declared-object hits,
the empty-obstacle control, and nine development-target hits/death/reset. Private
captures were inspected; these are development-path checks, not native trajectory
or pixel comparison, manual input validation or completed campaign combat.

[preview-2026.11.05.1](https://github.com/YYoudi/giants-rs/releases/tag/preview-2026.11.05.1)
is published at runtime source `47fe3f105fda48dc3a69ea8258d37c0e338a2aca`.
Its ZIP has 10 allowlisted members and retains the independent executable,
launcher, documentation, dependency/toolchain notices, manifest and checksums.
The locked dependencies and notice applicability are unchanged. The verified ZIP
was extracted outside the source tree; CLI startup and both silent lifecycle
endpoints passed again with the extracted executable. Older previews are retained.
The public ZIP and adjacent checksum were then downloaded anonymously and passed
the independent verifier with the same archive and executable hashes.

- Executable SHA-256: `e3245418b9e3474ca36a93c0f3b4fb84c977e33b5ef931eaaa80d7a63ba2777c`.
- ZIP SHA-256: `f1b6a2d114a42d24c7c8d90ee3f18a65925949367143778d298ea2cbf13ff495`.
- The runtime source's hosted Pages validation/deployment passed. Hosted Windows
  CI was still running at publication; local checks cover the released binary.

No complete game feature is newly marked done. Required owned target-header,
camera/muzzle/fire-phase services, live actor admission and impact reactions,
objectives and progression remain in BACKLOG.md.
