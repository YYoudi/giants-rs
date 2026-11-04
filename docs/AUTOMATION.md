# Roadmap, Pages and automatic builds

The source repository is public following the owner's explicit instruction and
the AI-assisted technical review in PUBLICATION_REVIEW.md. GitHub Pages is configured
for Actions, `PUBLIC_SITE_APPROVED=true`, with the `github-pages` environment limited
to main. The site contains the roadmap only; executable distribution is not approved.

Before cutover, both hosted workflows succeeded for
`0d43e84c1dd61b344174786ac44f3bd5f286f7f5`:
[Windows MinGW build](https://github.com/YYoudi/giants-rs/actions/runs/41691166679)
and [dashboard validation](https://github.com/YYoudi/giants-rs/actions/runs/41691166629).
Binary artifact upload, Pages upload and Pages deployment were skipped. Neither
approval variable was configured at that earlier checkpoint. The reviewed cutover
revision `dc99be9948b4dfcf6ead88d7edf38b947b0dafee` then passed
[hosted dashboard validation and history scanning](https://github.com/YYoudi/giants-rs/actions/runs/41724991825).
Its Windows workflow was relaunched; do not confuse an in-progress current run with
the completed baseline build. See Actions for the latest exact-revision result.
A repository with fresh history is not evidence
of a separated-team clean-room process; this project makes no such claim.

## One progress source

PROGRESS.json holds the 67 whole-game roadmap rows. Run:

```powershell
python -m tools.update_roadmap
python -m tools.export_site .local/site-review
```

The first command regenerates the README checklist and SVG; CI rejects stale
generated output. The second generates the English dashboard with search and
status filters, counts by feature family, and separate technical foundations.
It exports an explicit field list and the owner-authorized comparison display
copies pinned in docs/gallery/manifest.json, without private evidence,
original data or research annexes. Automated tests cover field exclusion,
rejection of private paths and HTML escaping. These are technical checks, not
rights clearance. The owner's approximate 99% AI-generated estimate remains visible.

## GitHub workflows

- `build.yml`: every push, PR and manual run installs a GNU toolchain and UCRT64
  MinGW compiler, runs synthetic Python/Rust tests, formatting and Clippy, then
  compiles the main application and produces an asset-free draft ZIP.
- `pages.yml`: checks full reachable Git history, validates the roadmap and site
  on pushes to main and PRs, and publishes only generated `_site` content on main.
- Third-party actions are pinned by commit. Neither workflow receives game data
  or private research inputs. Pull requests have read-only repository permissions
  and cannot deploy Pages or upload binary artifacts through these workflows.

The Windows runner is configured to use MSYS2 UCRT64, not Visual Studio. Its exact
compiler/version and executable hash are recorded in each build receipt. Rolling
toolchains/package indexes mean a later build need not be bit-identical; source
and dependency versions are recorded, and Cargo uses the lockfile.

## Current publication gates

1. Source cutover was approved and performed after the documented technical review
   and a clean remote-clone check. The private research repository and existing
   discussion worktrees remain in place; attach future runtime checkouts explicitly.
2. Actions remains enabled. Assess each exact revision's result; earlier successful
   builds do not certify later code or full gameplay.
3. Pages uses GitHub Actions and `PUBLIC_SITE_APPROVED=true`. It updates the roadmap
   on subsequent main pushes and never commits back into source. Source/site
   publication does not certify third-party rights or gameplay fidelity.
4. Binary dissemination is a separate gate: set `PUBLIC_DISTRIBUTION_APPROVED=true`
   only after distribution review. The artifact step ALSO requires an empty blocker
   list and `cleared_for_publication=true` from the package manifest. A variable
   alone cannot override the current draft-only packager. No release is published
   automatically by this candidate. Resolve notices, human release review and
   runtime-compiler applicability before implementing the reviewed release path.

MIT was selected under the owner's delegated choice; see LICENSE and NOTICE.md.
Current binary-release blockers: human provenance/distribution review,
constgebra 0.1.4 notice text, exact compiler runtime notice applicability. The
current packager deliberately always marks drafts uncleared. CI can compile while
binary distribution remains blocked. After review, prepare one manifest-based
release path; do not add a parallel unreviewed ZIP upload.

References: [MSYS2 setup action](https://github.com/msys2/setup-msys2),
[GitHub custom Pages workflows](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages).
