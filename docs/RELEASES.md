# Preview releases and milestone integration

The main deliverable is one Windows preview reached through the integrated menu,
using a user-supplied game installation. Individual CPU examples are research
controls and are not a substitute for a playable milestone.

After each meaningful validated milestone:

1. Review which acquired services can be connected safely to the main
   menu-to-mission path. Integrate as many coherent, verified features as possible;
   record prerequisites for anything still disconnected in BACKLOG.md.
2. Run the relevant source checks, Rust/Python tests, formatting, JavaScript and
   application build. Keep CPU evidence, actual runtime integration and fidelity
   separate. A build that only compiles is not a gameplay milestone.
3. Verify menu launch, mission selection/loading, supported player controls,
   return to menu and resource cleanup. Compare the new behavior appropriately.
   The owner has authorized manual tests, including necessary test windows and
   interactions. Use silent offscreen checks where appropriate; preserve unrelated
   applications and label missing human interaction validation explicitly.
4. Build from the recorded source commit and toolchain. Package only the independent
   executable, the launcher, instructions, checksums, manifest and necessary
   third-party notices. Never add original archives, extracted assets, original
   binaries, reference screenshots, dumps or user profiles to release assets.
5. Generate the README/Dashboard status and checklist. Release notes must state
   the features accessible from the menu, their limitations, data compatibility,
   changes since the last preview and exact validation performed.
6. Prepare locally, review the member allowlist and notices, and publish the
   Windows preview after technical validation. On 4 November 2026 the owner
   authorized downloadable previews and removed the mandatory human release
   review. Do not request another routine approval; do retain accurate limitations
   and complete notices. Source publication alone does not confer third-party rights.

Use `preview-YYYY.MM.DD.N` tags; keep older previews and their source identities
available. Do not replace a tested executable silently. A public release must
include an asset-free Windows ZIP and SHA256SUMS; GitHub's automatic source ZIP
alone is not a playable download.

Packaging is performed with `python -m tools.package_preview --help`. This tool
requires an explicit executable hash and matching build receipt. Dependency
license texts must accompany the binary; SPDX metadata alone is not a substitute
for the applicable notices. Packages cannot be cleared for publication merely
because their files pass an extension or secret-pattern filter.

Example draft preparation (use fresh private output directories):

```powershell
.\tools\build_preview.ps1 -OutputDirectory '.local\preview-build-NEW'
python -m tools.package_preview --executable .local/preview-build-NEW/giants-rs.exe --receipt .local/preview-build-NEW/build-receipt.json --metadata .local/preview-build-NEW/cargo-metadata.json --notes docs/PREVIEW_NOTES.md --output .local/preview-build-NEW/giants-preview-windows-x64.zip --draft --project-license LICENSE
python -m tools.verify_preview .local/preview-build-NEW/giants-preview-windows-x64.zip
```

The build receipt pins the source commit and executable hash. Private metadata
and logs remain outside the ZIP. A public package additionally requires `--review`
with a JSON receipt whose authorization is `owner_authorized_technical_review`,
matching source_commit, executable_sha256, target, toolchain and linker from the
build receipt, plus SHA-256 values of the project license, notice catalogues and
docs/DISTRIBUTION_REVIEW.md. The packager rejects missing notices and mismatches.

For public output, omit `--draft` and supply `--review` along with `--project-license`.
Run tools.verify_preview on the final ZIP and its adjacent checksum. It accepts
consistent uncleared drafts or reviewed public packages and verifies every member.
Publish the exact source tag, the ZIP containing the executable and notices, and
its external checksum. Do not offer a bare EXE detached from its notices.

Local validation can support a preview even while a later hosted run is pending;
state the exact local checks and the exact commit certified by hosted CI. Neither
checksums nor a successful build certify complete gameplay or legal clearance.

## Cinematic integration pass — 5 November 2026

The dedicated Flick branch adapts individual reviewed runtime corrections; it does
not import the historical private series. Ticket-owned text cleanup is connected
to the common selected-mission film mount/release path. CPU support-basis and
visibility fixes have existing private reference evidence and synthetic controls.
The repeatable silent opening/cancellation check is documented in BUILD.md.
It does not certify complete scenes, natural endings, skipping or gameplay entry.
Keep BACKLOG.md authoritative for remaining cinematic coverage and prerequisites.

Published Windows preview: `preview-2026.11.05.2`, source
`d880f62ff5ff0bba9865824f1840c55d3616cebe`.
Executable SHA256: `132535420ce13356a147c88a7d0b015a5447af5e2f5587298195be088f6561cd`.
ZIP SHA256: `a7d0588656dde9da18392961a87847ef9574f3c09f8849e86ce0cb99cefe1051`.
The packaged executable passed three silent opening/cancellation cycles and both
development-session lifecycle endpoints. Synthetic checks: 694 Rust passed,
14 ignored; 19 Python passed; formatting and Clippy passed. Package notices,
receipt identities and the downloaded ZIP/checksum were independently verified.
These checks do not certify the remaining cinematic coverage listed in BACKLOG.md.

## Cinematic stream-retirement pass - 5 November 2026

Published Windows preview: `preview-2026.11.05.3`, source
`046e9f03e4202e9d66595baa78cfa1d4fc672484`.
Executable SHA256: `55e7ee88d36961b54f4db0c2a4311cb97e9f8a290e205dc0ba524e08cd53572c`.
ZIP SHA256: `83ad700fbbbe152a6e720837eaed57dfc75c381a7060abc8688edb00b6bcb157`.
The common reader now reports unavailable audio time after stream retirement,
without synthesizing a cursor from realtime. Receipt-bound controls passed two
source-terminal/re-entry cycles for Story1 mission 5 (Mecc preview) and Story4
mission 1 (Gameplay), including nonempty text-atlas release in Story4 and actual
sync-track exhaustion before the Story1 director ended. A retirement receipt
accounts for a voice that drains between the audio update and cleanup; active
or missing sinks never count as finished voices.

Final validation: 695 synthetic Rust passed, 14 ignored; 20 Python passed;
formatting and Clippy passed. All artifacts were rebuilt in an isolated target
after rejecting a shared-cache test mismatch. Both development lifecycle endpoints
passed. Notices, build/review identities, ZIP members and downloaded checksums
were verified. Private exact-reference CPU controls cover 32 time queries and six
retirement paths given supplied device/event inputs, not actual device timing.
Full rendering, all-source transport/campaign coverage, native cadence, skip
controls, audio/subtitle parity and complete memory reclamation remain open.
