# Windows x64 development preview - preview-2026.11.06.3

Place giants-rs.exe beside the original Giants.exe and double-click it to open
the integrated menu. The adjacent installation is detected automatically. From
another folder, a picker asks for the installation and remembers the selection.
The ZIP includes the executable, launcher, instructions, license and notices.
Supply your own game installation. Linux and macOS downloads remain future work.

Approximately 99% of current project content is AI-generated according to the
owner's estimate and has not received comprehensive human review. This estimate
is not a completion metric. The owner authorized Windows previews without a
separate human release review. Technical checks do not establish full gameplay
fidelity or third-party rights; see DISTRIBUTION_REVIEW.md in the source repository.

## Persistent cinematic effect rendering

Compared with preview-2026.11.06.2, the main film now follows both acquired drawing
traversals for its bounded constant attached effects. The ordinary traversal uses
the current source camera and director draw range; the adjacent effect traversal
remains independent of that range. Both consume the same owned matrix, animation
channel, mesh and material. The first owned preparation also consumes the source
constructor gate, including zero-delta preparation. No placement, duration or
second animation clock is added.

Private original-code controls compare 50 source-object part-selection cases
and 14 preparation states, with supplied camera, alpha-query and model-slot
boundaries. The 23-source / 25-request cohort passes both entry/release cycles
and near/far traversal checks. These establish bounded CPU behavior; native
global sort order, full materials/raster and complete scenes remain unverified.
Exact executable runtime checks and package identities are recorded in RELEASES.md.

The existing **Preview tools → Completion cinematics** route is retained, with
four opening and 19 completion sources in the bounded persistent cohort. Filtered
choices preserve original mission slots. Replay does not record victory, restore
native live replay state or admit character gameplay. Natural ending or skipping
enters the partial free-camera preview; Escape returns to the menu.

## Accessible path and limits

The dedicated Preview All profile exposes mission browsing, completion cinematics,
a Mecc development arena and installation changes. Existing profiles are preserved;
original saves are not imported. Selected missions can present partial cinematics
and supported Mecc controls. Unsupported entries may remain scene inspection or
refuse startup. User state is outside the read-only installation. Read QUICK_START.md.

The combined source passes 768 synthetic Rust tests (27 ignored), 40 Python tests,
Rustfmt and Clippy. Exact executable/package receipts and runtime checks are
recorded in RELEASES.md and the release manifest. Earlier previews' UI or natural-
ending checks are not re-labelled as measurements of this combined executable.

Zero complete cinematic scenes are verified. The inventory retains 59 gap sources,
including 56 general-FX queue prerequisites, and 282 separate strict-profile
reviews. Native gameplay triggers, world admission/capacity, full character
controllers, exact camera/material/light/shadow/water/effect rendering, audible
voice mix, subtitles and temporal parity remain incomplete or unverified.
