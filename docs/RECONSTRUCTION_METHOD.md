# Reconstruction method and prototype PS2 attribution

This is the central method for all future native-function research, implementation
and context handoffs. `AGENTS.md` delegates to it. `BACKLOG.md` remains the single
detailed backlog. A recovered name is neither an implementation nor a fidelity
result. The PC binary under study remains the behavioral authority for its PC
profile; prototype PS2 metadata is an independent source of naming evidence.

## Required function workflow

1. Pin the exact PC binary SHA256, profile, virtual address and previous aliases.
   Never transfer an address between retail/debug or other builds by proximity.
2. **Search prototype PS2 names before further analysis or implementation.**
   Search the literal ELF function names, candidate demangled names and relevant
   module vocabulary. For existing work, perform this lookup before the next
   acquisition or change. Record an explicit no-match result when appropriate.
   Unavailable metadata means `unknown`, with the missing prerequisite retained
   in the handoff; it does not justify inventing a name.
3. Verify each candidate's address and raw name against the pinned ELF symbol
   table. Keep the raw name separate from the demangled interpretation and its
   parser/version/uncertainties. A type mentioned in a signature does not prove
   its field layout. Preserve overloads and repeated-parameter encodings.
4. Compare independent evidence: caller/callee roles, discriminating constants,
   string-use context, algorithm and side-effect ordering, and controlled
   behavior. Track counterevidence and competing candidates. Repeated tables
   derived from the same seed count as one source, not independent agreement.
5. Assign `confirmed`, `probable` or `unknown` to the **attribution**, separately
   from observed/inferred/hypothetical behavior, implementation readiness,
   runtime integration and fidelity. `confirmed` requires converging independent
   evidence and review of plausible alternatives. `probable` has meaningful
   support but unresolved alternatives or coverage. `unknown` includes no match,
   score-only candidates and unresolved contradictions. Record rejected
   candidates and reasons privately rather than silently promoting them later.
6. Implement the acquired PC contract in Rust + Bevy. Keep PS2/PC version,
   platform, arithmetic, scheduling, calling convention and memory-layout
   differences explicit. Do not import PS2 signatures, ABI or offsets into PC
   services. Validate with the existing PC controls and the synthetic commands
   in [BUILD.md](BUILD.md); add scoped behavioral controls where needed.
7. Preserve every previous identifier as an alias. Add reviewed attributions
   incrementally; do not mass-rename code or analysis databases from tables.
   Carry the lookup, attribution status, implementation location, unresolved
   differences and next verification into the private continuation note.

Automatic scores, `SAFE_APPLY`, similar sizes, byte-hash labels, graph propagation,
module proximity and generated names are candidate-discovery aids only. None is
sufficient confirmation. A script named "verify" is not itself independent proof.

## Private record and repeatable lookup

Keep a private record per PC function, with these separate fields:

| Field | Required contents |
| --- | --- |
| PC identity | SHA256, build/profile, virtual address, all earlier aliases |
| Prototype identity | SHA256, PS2 virtual address, raw ELF symbol and symbol-table verification |
| Demangling | Display name/signature, decoder identity, unresolved encodings and alternatives |
| Rust | Repository module and symbol, source commit or explicitly unpublished worktree state; null if absent |
| Attribution | `confirmed`, `probable` or `unknown`, review rationale and rejected alternatives |
| Evidence | Private content-addressed records, tool versions, independent observations and counterevidence |
| Validation | Exact control/build identities, supplied boundaries, observed results and untested behavior |
| Divergences | Platform/version/ABI/layout/arithmetic differences and remaining prerequisites |
| Publication | Explicit minimal-field review; false for generated worksheets |

`python -m tools.ps2_symbol_lookup --help` provides a read-only starting point.
Supply `--elf`, `--pc-binary`, `--pc-address`, one or more `--query` values,
`--rust` and a fresh `--output` under the checkout's ignored `.local` directory.
Optionally supply `--symbol-csv` for unreviewed demangled labels. Configure private
input paths locally; do not add them to repository documents or commands.

The tool pins the prototype hash, reads its actual ELF function symbols, checks
supplied CSV function facts against that ELF, hashes the PC input and records
the lookup even with zero candidates. It never executes inputs, imports research
scripts, applies renames, assigns confidence scores or promotes an attribution.
Its output starts at `unknown`, with publication review false. It is a private
worksheet, not the complete reviewed record or a public export format.

## Resource audit — 5 November 2026

Observed prototype identity: SHA256
`be17fa4b1b5c5120dd685891894c93e65509e55bed59756110f39b0ea48258ef`.
Both supplied copies are byte-identical: 4,041,488 bytes, ELF32 little-endian MIPS,
4,769 nonzero-size function symbols, and a 1,015,349-byte `.mdebug` section.
The audit verified the supplied function CSV's address/raw-name/size triples
against the ELF; all 4,769 agree. This does not authenticate a PC correspondence.
The prototype-to-ISO identity is owner-supplied; this audit compared the two ELF
copies and did not independently extract an ISO.

| Resource class | Evidentiary role and limits |
| --- | --- |
| ELF `.symtab` | Actual raw function names and PS2 addresses for this hash. Other symbol kinds, including file entries, must not be treated as functions or published wholesale. |
| `SLUS_201.78.symbols.csv`, function listing | Extractions to cross-check against the ELF. The CSV's demangled column is generated, not literal ELF naming. |
| Demangled signatures | Supplied extractor uses a partial old-GNU parser. At least 264 function rows contain visible unknown/repeated/member markers; absence of a marker does not prove a complete or correct signature. For example repeated-parameter encodings remain unresolved. |
| `MODULE_INDEX.md` | Derived semantic/name grouping for navigation; not an original compilation-unit or PC module map. |
| `TYPE_CATALOG.md` | Derived from demangled parameter frequencies, not a verified `.mdebug` type/layout extraction. `.mdebug` presence alone validates no type. |
| `pc_ps2_matches*`, constant anchors, similarity and graph tables | Proposed correspondences. Check exact PC identity before use; a filename suffix does not supply its hash. Graph descendants retain their seed's uncertainty. |
| `pc_rename_apply*` | Generated sanitized/de-duplicated names. The builder assigns `SAFE_APPLY` from a score threshold; it is not independent confirmation. |
| `unified_names*`, naming waves, `llm_*` | Mixed inferred/generated labels. The audited primary directory contains 265 `llm_*` files. Empty PS2-address cells occur; an apparent PS2-name column can contain an inferred PC label. Recover provenance per row before use. |
| Extraction, matching, verification and rename scripts | Read as untrusted research tooling. Do not execute in the corpus or allow automatic database writes; reproduce needed read-only operations with pinned inputs and private outputs. |

Private audit retains file hashes, candidate searches, selected scalar/call
observations, rejected alternatives and control receipts. No raw tables, listings,
extractions, debug data or research scripts are copied into this repository.

## First reviewed attributions

All PC addresses below refer **only** to PC debug SHA256
`690bad39847b0f578163e547e6a6de10ffe9213635a069bd1ce96a75dfd39ead`.
All PS2 addresses refer only to the prototype hash above. Addresses are virtual,
not file offsets. Previous PC address labels remain aliases in private evidence.

| PC address | PS2 address / literal symbol | Demangled interpretation | Attribution and Rust relationship |
| --- | --- | --- | --- |
| `0x0040d7b0` | `0x00114c00` / `SetupCameraDataNormal__FP6OBJECTT0` | `SetupCameraDataNormal(OBJECT*, /*same*/)`; second argument unresolved by the supplied decoder | **confirmed** routine identity, not full behavior equivalence. Published `mecc_camera::pose` covers only a steady-profile subset; the stateful producer remains unpublished work in progress. |
| `0x00454bb0` | `0x00134738` / `chase_value_time` | Literal unmangled name; no complete signature inferred | **confirmed** service identity. Stateful camera approach implementation remains unpublished work in progress. |
| `0x00416130` | `0x0011c0f8` / `collide_camera_check_walls` | Literal unmangled name | **probable** from camera caller role and clipping purpose; independent complete PS2 algorithm/control review remains pending. Rust clipping work is unpublished. |
| `0x0040caa0` | `0x00113e28` / `camera_get_curcamdef__Fv` | `camera_get_curcamdef(void)`; decoder interpretation | **probable** from profile selection and caller context; variant-specific branches remain to compare. No complete published Rust counterpart. |
| `0x0051e510` | Not assigned | Not assigned | **unknown**. PS2 camera-component services use a different renderer path; similar purpose is insufficient. No exact Rust attribution assigned. |

Normal-camera attribution combines the characteristic 50-iteration adjustment,
four calls to the same temporal-approach service with minimum-rate/maximum-gap
arguments 10/50, profile/trigonometry/obstacle call sequence and final camera
publication. Prototype-specific character and renderer branches remain distinct.
The apparent score-1.00 suggestion `camera_override_refpt` is rejected: that
literal PS2 function is a short override setter, not the normal-camera algorithm.
It does not establish the candidate table's PC binary identity either.

Approach-service attribution independently combines the camera's four call sites,
frame-seconds dependency, proportional/minimum step selection, directional
overshoot handling and maximum-gap correction. The nearby `chase_value_time_jim`
alternative lacks the same minimum-step/maximum-gap contract. A private 256-case
control of the original PS2 scalar body passes against an independently expressed
float32 model. Caller registers/frame seconds are supplied; four EE return-register
restores are replaced in memory by no-ops. Entry/exit ABI, full PS2 execution and
PC/PS2 bit equivalence are excluded. Existing PC camera controls establish only
their declared PC boundaries; these names do not enlarge them.

Read-only scalar/call inspection used standard MIPS encodings. The general decoder
does not cover all EE/MMI instructions and can mislabel them as other extensions;
those renderings were excluded from the attribution evidence. No claim of complete
PS2 disassembly, `.mdebug` recovery or original source reconstruction is made.

## Publication boundary

The table above is a deliberately small reviewed attribution summary: binary
digests, addresses, a few literal names, uncertain interpretations, existing Rust
relationships and independently written rationale. It is not a raw symbol-table
export. Before adding a row, review necessity, identity, ambiguity, aliases,
evidence scope and confidentiality. Never publish private input locations,
personal provenance, executable/ISO files, corpus tables, raw listings,
decompilations, extracted debug data or private control captures/reports.

Use only the neutral designation **prototype PS2**. Available symbols confer no
redistribution rights or expansion of the project license. Keep package/notices
validation and source review separate under [RELEASES.md](RELEASES.md) and
[PUBLICATION_REVIEW.md](PUBLICATION_REVIEW.md). Documentation-only attribution
changes do not justify a new playable-preview or game-completion claim.
