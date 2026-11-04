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
python -m tools.package_preview --executable .local/preview-build-NEW/giants-app.exe --receipt .local/preview-build-NEW/build-receipt.json --metadata .local/preview-build-NEW/cargo-metadata.json --notes docs/PREVIEW_NOTES.md --output .local/preview-build-NEW/giants-preview-windows-x64.zip --draft --project-license LICENSE
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
