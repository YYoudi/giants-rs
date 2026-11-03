# Roadmap, Pages and automatic builds

The repository is private staging, with no public deployment. On 3 November 2026,
both hosted workflows succeeded for `0d43e84c1dd61b344174786ac44f3bd5f286f7f5`:
[Windows MinGW build](https://github.com/YYoudi/giants-rs/actions/runs/41691166679)
and [dashboard validation](https://github.com/YYoudi/giants-rs/actions/runs/41691166629).
Binary artifact upload, Pages upload and Pages deployment were skipped. Neither
approval variable was configured when checked. These results validate that source
revision; subsequent local preparation still requires its own hosted validation.
The candidate remains subject to initial owner approval and
provenance/distribution review. A repository with fresh history is not evidence
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
It exports only an explicit field list, without screenshots, private evidence,
original data or research annexes. Automated tests cover field exclusion,
rejection of private paths and HTML escaping. These are technical checks, not
rights clearance. The owner's approximate 99% AI-generated estimate remains visible.

## GitHub workflows

- `build.yml`: every push, PR and manual run installs a GNU toolchain and UCRT64
  MinGW compiler, runs synthetic Python/Rust tests, formatting and Clippy, then
  compiles the main application and produces an asset-free draft ZIP.
- `pages.yml`: validates the roadmap and site on pushes to main and PRs; after
  explicit site approval it publishes only the generated `_site` directory to Pages.
- Third-party actions are pinned by commit. Neither workflow receives game data
  or private research inputs. Pull requests have read-only repository permissions
  and cannot deploy Pages or upload binary artifacts through these workflows.

The Windows runner is configured to use MSYS2 UCRT64, not Visual Studio. Its exact
compiler/version and executable hash are recorded in each build receipt. Rolling
toolchains/package indexes mean a later build need not be bit-identical; source
and dependency versions are recorded, and Cargo uses the lockfile.

## Activation after the initial owner-approved cutover

1. Review the private staging `YYoudi/giants-rs` repository with MIT source license
   and fresh history, then approve its first public exposure. Retain the private research
   repository unchanged as an archive. Attach agent checkouts explicitly.
2. Actions is enabled and the first hosted Windows and site-validation runs passed.
   Recheck the exact reviewed candidate revision before cutover; earlier successful
   runs do not validate subsequent changes.
3. Select GitHub Actions as the Pages publishing source and configure the
   `github-pages` environment for main. Set repository variable
   `PUBLIC_SITE_APPROVED=true` only after approving the generated site. The workflow
   then updates Pages on subsequent main pushes; it never commits back into source.
4. Binary dissemination is a separate gate: set `PUBLIC_DISTRIBUTION_APPROVED=true`
   only after distribution review. The artifact step ALSO requires an empty blocker
   list and `cleared_for_publication=true` from the package manifest. A variable
   alone cannot override the current draft-only packager. No release is published
   automatically by this candidate. Resolve notices, source licensing and recorded
   runtime-compiler applicability before implementing the reviewed release path.

MIT was selected under the owner's delegated choice; see LICENSE and NOTICE.md.
Current blockers: human provenance/distribution review,
constgebra 0.1.4 notice text, exact compiler runtime notice applicability. The
current packager deliberately always marks drafts uncleared. CI can compile while
binary distribution remains blocked. After review, prepare one manifest-based
release path; do not add a parallel unreviewed ZIP upload.

References: [MSYS2 setup action](https://github.com/msys2/setup-msys2),
[GitHub custom Pages workflows](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages).
