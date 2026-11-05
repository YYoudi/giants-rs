# Source publication review — 3 November 2026

## Prototype PS2 methodology addition — 5 November 2026

The reviewed public addition is the [central method](RECONSTRUCTION_METHOD.md),
a read-only lookup tool, synthetic tests and a minimal five-row attribution
summary. It contains two confirmed identities, two probable candidates and one
unassigned service, with explicit binary hashes and independent rationale.
No corpus file, executable, ISO, raw table, listing, debug extraction or private
research report is included. Detailed receipts and personal provenance stay
private. Available symbols grant no redistribution rights. This addition changes
neither the historical review's scope nor the current binary-release policy in
[RELEASES.md](RELEASES.md).

## Historical source-publication review

**Technical review completed; source publication performed under owner authorization.**
After being told the limitations of an AI-assisted review, the owner explicitly
asked the agent to perform the verification and make the repository public.
No concrete source-publication blocker was identified within the scope below.
This decision is not a legal clearance or a guarantee against claims. Executable
distribution remains blocked. Actual cutover status is recorded in TRANSITION.md.

## Observed technical state

- Pre-cutover GitHub inventory: private `YYoudi/giants-rs`, only `main` remotely,
  no tags, releases, retained Actions artifacts, issues or pull requests. Discussions
  are disabled; the wiki Git endpoint reports no repository. All four retained
  workflow logs were downloaded locally and checked for selected credentials and
  owner-directory patterns; none were detected. Hosted runner directories are not
  owner workstation paths. The private research repository remains private.
- README is English, identifies the project as unofficial and incomplete, requires
  user-supplied game data and states the scope of MIT in NOTICE.md. It preserves
  the owner's approximate 99% AI-generated estimate and lack of comprehensive
  human review. That estimate is not a completion metric.
- The README checklist, SVG and dashboard derive from this project's PROGRESS.json.
  Dashboard links target this repository. Export is restricted to selected fields;
  source tests cover private-field exclusion and HTML escaping. No game images,
  branding artwork or original assets are part of the site export.
- History scan at `0f4e03de34ff1be5faf4e29b6594dfb074157ada`: 9 reachable
  commits and 551 distinct file blobs, without findings for the selected hazards.
  The reproducible `tools.audit_source_tree` scanner now also checks every historical
  pathname, rather than only one representative name per blob. It flags selected
  binary/assets/font extensions, links/submodules, NUL bytes, personal-directory
  paths, credentials and common decompiler labels. Tests include a secret removed
  at HEAD and a renamed binary removed at HEAD. Reports do not print secret values.
- Baseline hosted CI and the later local build/package checks are recorded in
  TRANSITION.md. Neither result establishes rights or gameplay fidelity.

## Source provenance triage and targeted inspection

The prepared runtime contains 433 Rust files and 23 WESL files: 139,281 lines,
5,115,202 bytes before documentation/review-tool additions. Automated triage covered
these files for large numeric arrays, encoded payloads, embedding mechanisms,
foreign-source/decompiler markers and long string literals. The only foreign-source
text hit was a comment stating that a normal-preparation table is calculated from
mathematics rather than imported from a binary. It was inspected with its function.

Targeted source inspection covered the archive reader and synthetic construction
test, frontend catalog/text access, read_asset, external font selection, shader
embedding sites and representative shader bodies, model/material preparation,
animation arithmetic, target-query selection and resident actor ownership. The long
string triage identified diagnostics and CLI/help text, not an embedded game script
or dialogue corpus. This was a targeted AI review, not line-by-line human review.

Observed in those paths: artwork, mesh, sound, catalog and displayed game text are
read from user-supplied files; system/explicit fonts are external. Embedded project
assets are WESL shaders that perform rendering calculations on supplied resources.
The inspected tests construct small synthetic archives and geometry. Numeric format
identifiers, field offsets, flags and arithmetic coefficients are not machine-code
instruction dumps. No embedded original artwork, native instruction-signature probe,
dialogue corpus or identifiable decompiled-code listing was found in this review.

No vendored third-party implementation was identified in the inspected source. Cargo fetches
locked dependencies separately. Committed third-party license texts retain their
scope and are not relicensed by MIT. Existing notice receipts and the package
builder's hash validation remain in use. This does not close binary notice review.

The private transition handoffs were consulted to establish the selection method.
The fresh candidate excludes research observers, CPU replay examples, reference
inputs, galleries and original-history branches. The current 12 archive worktrees
were inventoried read-only, with local hashes of dirty tracked files. They were
neither moved nor reset; pending branch changes were not silently integrated.

## Residual provenance and legal uncertainty

The repository records reconstruction informed by private research; it does not
claim a separated-team clean room. The available technical checks cannot determine
whether every implementation and fixture is independently expressed or whether
protected material was carried over. Rewriting in Rust, selecting MIT, generating
code with AI or excluding game assets does not answer that provenance question.

For a France/EU review, the distinction between unprotected ideas/principles and
protected program expression matters. Article L122-6-1 permits specified observation
and interoperability-related acts under conditions, with limits on subsequent use
and disclosure. Whether this project's research and publication satisfy those
conditions has not been established as a legal conclusion. A qualified
intellectual-property review can assess the actual method and source if legal
assurance is required. The owner-authorized technical publication decision above
must not be presented as that assurance.

Primary references consulted on 3 November 2026:

- [French Intellectual Property Code, Article L122-6-1](https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000006278920)
- [Directive 2009/24/EC, especially Articles 1, 5 and 6](https://eur-lex.europa.eu/legal-content/EN/ALL/?uri=CELEX%3A32009L0024)

This records unresolved questions, not a legal finding that the project infringes
or a promise that publication cannot attract a claim.

## Source, dashboard and binary distribution

Source exposure includes reachable history and hosted logs, not just the README.
The owner's authorization covers this reviewed source candidate and its asset-free
roadmap dashboard. It does not cover exposing the research archive, its gallery,
pending worktrees or private evidence. The dashboard links to this repository and
will track subsequent PROGRESS.json changes through its main-branch Pages workflow.

The source-only approval above did not itself authorize executable distribution.
On 4 November 2026 the owner separately requested Windows downloads and removed
the mandatory human release-review gate. The subsequent technical binary notice
assessment and receipt-bound checks are in DISTRIBUTION_REVIEW.md. constgebra's
Apache license alternative and complete conservative runtime notices are included.
This does not turn the source review into a human audit or legal clearance.

BACKLOG.md remains the detailed task list. Binary authorization was given separately
on 4 November 2026, as recorded above.

## Dashboard update — 4 November 2026

The owner explicitly authorized restoring all historical original / Bevy
comparison screenshots to the public GitHub Pages dashboard. This replaces the
previous asset-free/gallery exclusion for these selected display copies only.
It does not authorize exposing private research reports, paths or executables,
and does not establish third-party redistribution rights.

The gallery contains 302 comparisons and 502 content-addressed, metadata-free
WebP display copies. Labels, historical revisions and limitations are retained;
private report links and source paths are omitted. Compression changes pixels,
so these copies are not a fidelity-validation dataset. See gallery/README.md.
The roadmap now uses grouped tables, state colors and counts by domain, with
technical foundations separate from whole-game feature families. The approximate
99% AI-generated estimate remains unrelated to game completion.

