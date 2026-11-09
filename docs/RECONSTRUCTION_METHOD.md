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
   differences explicit. Across executable/DLL boundaries, establish the owner
   of each clock and random state. Identical RNG algorithms or supplied seed
   controls do not prove a shared live stream. Verify TLS/initialization and
   interleaving before merging streams or moving draws across service calls.
   Do not import PS2 signatures, ABI or offsets into PC services. Validate with
   the existing PC controls and the synthetic commands
   in [BUILD.md](BUILD.md); add scoped behavioral controls where needed.
7. Preserve every previous identifier as an alias. Add reviewed attributions
   incrementally; do not mass-rename code or analysis databases from tables.
   Carry the lookup, attribution status, implementation location, unresolved
   differences and next verification into the private continuation note.

Automatic scores, `SAFE_APPLY`, similar sizes, byte-hash labels, graph propagation,
module proximity and generated names are candidate-discovery aids only. None is
sufficient confirmation. A script named "verify" is not itself independent proof.

Retaining generated PC aliases in a reviewed public summary can trigger the
history scanner's label pattern. After reviewing the exact minimal document,
record only its normalized Git-blob SHA256 and sole public path in
`tools.audit_source_tree.REVIEWED_ATTRIBUTION_LABELS`. Do not broaden the pattern
or approve an extraction/listing. Other hazard checks remain active. Stage the
document before hashing its blob, and rerun the all-reference history audit after
publication-related fetches; a concurrent branch can introduce a new document
revision even when the current main file has not changed. Review every finding
before treating the publication audit as passed.

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

Use `--pc-profile` to retain the build label and repeat `--alias` for every earlier
identifier. These supplement the PC digest/address; a profile name cannot replace
the digest. Supply `--demangler` only when the actual decoder/version is known.
An omitted profile or decoder stays null, never inferred from a filename. The
worksheet records a digest of the supplied CSV text re-encoded as UTF-8 (after
Python newline normalization), separately from the exact binary digests. Retain
the original CSV file digest in the private resource inventory as well.

The tool pins the prototype hash, reads its actual ELF function symbols, checks
supplied CSV function facts against that ELF, hashes the PC input and records
the lookup even with zero candidates. It never executes inputs, imports research
scripts, applies renames, assigns confidence scores or promotes an attribution.
Its output starts at `unknown`, with publication review false. It is a private
worksheet, not the complete reviewed record or a public export format.

### Continuation record

Every private task handoff must contain the following short record for its next
function, including services already implemented and revisited on touch:

```text
PC: SHA256 / profile / virtual address / previous aliases
Lookup: queries / literal ELF verification / explicit match or no-match result
Prototype PS2: SHA256 / virtual address / raw symbol, or unassigned
Demangling: interpretation / actual decoder version or unknown / ambiguities
Attribution: confirmed, probable or unknown / independent evidence / alternatives
Rust: module::symbol / source commit / implemented and unverified boundaries
Next: missing comparisons / platform divergences / next behavioral control
Publication: private by default / minimal-summary review if publication is needed
```

Link the existing private record instead of copying corpus excerpts into the
handoff. On resume, verify that the recorded binary and source identities still
apply; reuse a valid completed lookup rather than mechanically repeating it.
A changed binary needs a new identity-bound lookup. Missing evidence remains an
explicit prerequisite; neither a recovered name nor a successful lookup permits
automatic renaming, a new ABI assumption or a gameplay progress promotion.

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
| `0x0040d7b0` | `0x00114c00` / `SetupCameraDataNormal__FP6OBJECTT0` | `SetupCameraDataNormal(OBJECT*, /*same*/)`; second argument unresolved by the supplied decoder | **confirmed** routine identity, not full behavior equivalence. Rust `mecc_camera_update::State::advance` implements the acquired ordinary/direct-angle subset. |
| `0x00454bb0` | `0x00134738` / `chase_value_time` | Literal unmangled name; no complete signature inferred | **confirmed** service identity. Rust `mecc_camera_update::approach` implements the acquired PC camera call profile. |
| `0x00416130` | `0x0011c0f8` / `collide_camera_check_walls` | Literal unmangled name | **probable** from camera caller role and clipping purpose; independent complete PS2 algorithm/control review remains pending. Rust `mecc_camera_wall::clip` has separate PC controls. |
| `0x0040caa0` | `0x00113e28` / `camera_get_curcamdef__Fv` | `camera_get_curcamdef(void)`; decoder interpretation | **probable** from profile selection and caller context; variant-specific branches remain to compare. Rust `mecc_camera_update::Profile::normal` covers only supplied normal Mecc profiles, not the full selector. |
| `0x0051e510` | Not assigned | Not assigned | **unknown** PS2 counterpart. Rust `mecc_camera_update::State::advance` produces the acquired PC viewport cache; similar-purpose PS2 camera-component services use a different renderer path. |
| `0x0049f5d0` | `0x001743f8` / `is_firstp_display__FP6OBJECT` | `is_firstp_display(OBJECT*)`; decoder interpretation, no return type inferred | **confirmed** predicate identity. Rust `first_person::Display::owns` compares actual camera ownership and active display status; it does not prepare a subjective actor. |

The Rust implementation and integration boundaries are recorded separately in
[OWNED_CAMERA.md](OWNED_CAMERA.md); their addition does not promote the three
remaining uncertain PS2 correspondences.

The display predicate adds an independently reviewed identity: both bodies
compare the object with a camera-owner getter and require a nonzero active-display
flag. A 512-case control agrees between the full original PC helper, Rust and the
PS2 comparison body within declared boundaries. The latter supplies incoming
source/getter values and translates two return moves within a 32-bit domain;
it excludes the EE ABI. PS2 callers observed in effect/display services do not
prove identical PC/PS2 world scheduling. The separate PC pre/post-world routines
remain without assigned PS2 counterparts. See [OWNED_LOCAL_FIRE.md](OWNED_LOCAL_FIRE.md).

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

## Current-function lookup follow-up — 6 November 2026

The following PC functions have private, identity-bound name-search worksheets.
They use the PC debug digest stated above. These are **unknown attributions**,
not additional matches in the six-service reviewed summary. Literal vocabulary
hits identify candidates to investigate; they do not establish a counterpart.
No PS2 address, raw symbol or demangled signature is assigned to these rows.

| PC address / retained alias | Current Rust relationship | Missing attribution evidence |
| --- | --- | --- |
| `0x0050d8b0` / `FUN_0050d8b0` | Bounded pistol profile in `mecc_secondary::ordinary`, including post-impact/directional-air continuation; no complete controller equivalence | Independent PS2 selector branches, constants and call ordering |
| `0x0045b580` / `FUN_0045b580` | Observed weapon-commit entry following `mecc_secondary`; helper body and idle-draw callback are not acquired by the continuation control | Independent PS2 commit/callback algorithm and caller comparison |
| `0x0047c3d0` / `FUN_0047c3d0` | `Session::advance_mecc_primary_with_history`, including its deferred-fire route | Independent PS2 primary/controller algorithm and call-order comparison |
| `0x0049e620` / `FUN_0049e620` | `mecc_subjective::State::prepare_from_session` | Independent pre-world copy/resource and caller correspondence |
| `0x0049eaf0` / `FUN_0049eaf0` | `Session::advance_mecc_subjective_post` and `mecc_subjective::State::post` | Independent post-world selection/ANM/fire ordering correspondence |
| `0x0049f1e0` / `FUN_0049f1e0` | `first_person::animation` | Independent selector branches, constants and caller correspondence |
| `0x0049ece0` / `FUN_0049ece0` | `first_person::place` | Independent placement/camera invocation and transform correspondence |
| `0x0045b520` / `FUN_0045b520` | Selected-weapon boundary in `resident_states/subjective.rs`; no standalone getter equivalence claimed | Independent getter/storage and caller correspondence |
| `0x004b1450` / `FUN_004b1450` | Authored icon-family boundary in `resident_states/subjective.rs`; no standalone getter equivalence claimed | Independent resource-selection and caller correspondence |
| `0x0049f3f0` / `FUN_0049f3f0` | Class-2 profile of `mecc_draw_policy::submit`; preparation and model callbacks remain external | Independent PS2 dispatcher/caller and character-branch comparison |
| `0x0049f560` / `FUN_0049f560` | `mecc_draw_policy::update_held_pose`; historical alias `submit_held`; owner getter and attachment update remain external | Independent PS2 temporary-parent and detachment-order comparison |
| `0x00473b30` / `FUN_00473b30` | Normal matrix-only profile in `mecc_wield_pose::sample` and `sample_subjective`; position/history/spatial tail remains external | Independent PS2 attachment/reference/scale algorithm and caller comparison |
| `0x00432330` / `FUN_00432330` | Observed model-submission entry; existing `render_parts` is a resolved-snapshot planner, not live entry integration | Independent PS2 model/material/queue and caller comparison |
| `0x0042fcf0` / `FUN_0042fcf0` | `lod::Table::select_positioned`, used by `mecc_model_draw::prepare` with current owned source/camera | Independent PS2 table/sentinel, distance/quality and caller comparison |
| `0x00430bd0` / `FUN_00430bd0` | Model preparation relates to `mecc_model_draw`, `camera_facing`, `body_pipeline` and `material_descriptor`; full consumer/graphics equivalence is not claimed | Independent PS2 model/material/lighting and primitive-submission comparison |

Earlier private worksheets referring to `subjective_player::State` remain
retrievable aliases, not proof that the presentation player implements the native
pre/post-world contract. Subsequent records must name the current owned module
and retain that older relationship as historical. A worksheet's default
`unknown` is also retained after a separate reviewed record confirms a match:
the discovery worksheet is immutable evidence, not the attribution authority.
For the three confirmed rows above, use the reviewed rationale and scoped control
receipts rather than silently editing the original searches to look confirmed.

## Publication boundary

Before publishing any additional attribution, select only the necessary binary
digests, virtual addresses, raw symbol, qualified demangling, retained PC alias,
Rust relationship, status and independently written evidence/limits. Review that
selection separately from the full private record; never export a worksheet or
candidate table as the public representation.

The six-service table is a deliberately small reviewed attribution summary: binary
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
