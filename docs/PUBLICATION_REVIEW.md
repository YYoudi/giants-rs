# Source publication review — 3 November 2026

**Not cleared for public exposure.** The owner's request to go public is conditional
on a clean README, a dashboard tracking this project and copyright confidence.
The technical checks below do not satisfy the unresolved provenance condition.
No repository visibility, Pages setting or approval variable has been changed.

## Observed technical state

- GitHub reports `YYoudi/giants-rs` as private, with `main` as its default branch
  and no Pages deployment configured. Local preparation is on a `codex/` branch.
- README is English, identifies the project as unofficial and incomplete, requires
  user-supplied game data and states the scope of MIT in NOTICE.md. It preserves
  the owner's approximate 99% AI-generated estimate and lack of comprehensive
  human review. That estimate is not a completion metric.
- The README checklist, SVG and dashboard derive from this project's PROGRESS.json.
  Dashboard links target this repository. Export is restricted to selected fields;
  source tests cover private-field exclusion and HTML escaping. No game images,
  branding artwork or original assets are part of the site export.
- A local pattern scan at `c06dc06ea52fefa7af00a71b7f2c0b8febc2e73d`
  examined all 8 commits reachable through the current local refs and 547 distinct
  file blobs. No matches were found for the scanned binary/asset extensions, NUL
  bytes, personal-directory paths, selected credential patterns or common decompiler
  function labels. The ignored scan report remains local. This is neither a full
  secret detector nor a comparison against original code, installations or corpora.
- Baseline hosted CI and the later local build/package checks are recorded in
  TRANSITION.md. Neither result establishes rights or gameplay fidelity.

## Unresolved source question

The repository records reconstruction informed by private research; it does not
claim a separated-team clean room. The available technical checks cannot determine
whether every implementation and fixture is independently expressed or whether
protected material was carried over. Rewriting in Rust, selecting MIT, generating
code with AI or excluding game assets does not answer that provenance question.

For a France/EU review, the distinction between unprotected ideas/principles and
protected program expression matters. Article L122-6-1 permits specified observation
and interoperability-related acts under conditions, with limits on subsequent use
and disclosure. Whether this project's research and publication satisfy those
conditions has not been established. A qualified intellectual-property review of
the actual method and source is appropriate before treating this gate as cleared.

Primary references consulted on 3 November 2026:

- [French Intellectual Property Code, Article L122-6-1](https://www.legifrance.gouv.fr/codes/article_lc/LEGIARTI000006278920)
- [Directive 2009/24/EC, especially Articles 1, 5 and 6](https://eur-lex.europa.eu/legal-content/EN/ALL/?uri=CELEX%3A32009L0024)

This records unresolved questions, not a legal finding that the project infringes
or a promise that publication cannot attract a claim.

## Source, dashboard and binary distribution

Source exposure and Pages deployment publish their own contents and require the
source/provenance decision above. A first public exposure would include the Git
history, not just the README. Pages remains disabled until that decision.

Binary dissemination has additional unresolved checks: the constgebra 0.1.4
license text and exact compiler/runtime notice applicability. Those binary-package
blockers must not be confused with a legal conclusion about publishing only this
repository's source. The current packager deliberately produces uncleared drafts.

BACKLOG.md remains the detailed task list. No release, public push or visibility
change follows automatically from this review record.
