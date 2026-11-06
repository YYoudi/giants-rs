# Public comparison gallery

On 4 November 2026 the owner explicitly requested publication of **all** historical
original / Bevy comparisons from the previous private dashboard. This supersedes
the earlier asset-free dashboard restriction only for these display captures.
The private repository, reports, inputs, executables and research paths remain
excluded. The archive was read without modifying it.

`manifest.json` contains 304 comparison records, their labels, limitations and
source revisions, plus two standalone build screenshots. Its `images` map pins
the SHA-256 of 507 distinct WebP display copies: 502 historical images (from 508
source PNG files) and five additional build/world-inspection captures.
Only explicitly listed files are exported. Source paths and report links are not
retained. Image metadata was stripped; dimensions were preserved. WebP quality 90
reduces approximately 692 MB of source PNGs to 89 MB. These lossy display copies
must not be used for pixel-validation claims. Original PNGs remain private.

The gallery is historical evidence. It does not establish that a feature is in
the current public build, that complete missions work or that fidelity is verified.
Original/game-data-derived artwork and trademarks remain outside the project MIT
license. Owner publication authorization is not third-party rights clearance.
Executable publication and dependency notice review remain separately gated.

The source-history auditor recognizes only exact hashes at the declared media
paths, with a WebP format check. Undeclared images, changed payloads, historical
aliases, other binaries and private path/credential findings remain reportable.

The site exporter validates all image hashes before creating output and emits
only public comparison fields. Run `python -m tools.export_site NEW_OUTPUT_DIR`
and `node --check NEW_OUTPUT_DIR/app.js`. No game installation is needed to build
the dashboard. Pages continues to deploy main through the existing workflow.

Each comparison opens as superimposed captures with a horizontal draggable divider.
The native range control also supports arrow keys, Home and End; buttons reveal
either full image or restore the 50/50 split. Both images keep their proportions
in the same frame. Side-by-side views and full-size links remain in a disclosure
below each comparator. A shared frame does not imply aligned cameras or phases;
the historical context and limits still apply.

## Recorded Bevy screenshots - 6 November 2026

At the owner's request, two inspected display captures from recorded build
`2a907b04d582e97baea2055ab09266379b8d2928` now appear above the progress tables:
the menu/profile screen and the Mecc development arena. They are actual engine
renders, not Delphi gameplay or fidelity evidence. Their metadata-free WebP
copies are pinned in the manifest's `images` map and described in `screenshots`.
The current total is 304 comparisons and 507 distinct display images.
No private capture reports, source paths or original executables are published.
The static screenshot section and hero remain visible without JavaScript; the
historical comparison dividers retain their existing interactive behavior.

The two dated KStory1 comparison records retain their inspected display copies
from capture stage `0316bd9`, rendered by preview `preview-2026.11.06.1`. They
compare world layers and fog settings, not native-engine parity or playable Kabuto.
