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
   Respect the owner's current background-only restriction: interactive checks
   requiring focus remain pending until permitted. Use silent offscreen checks
   where available; label missing human interaction validation explicitly.
4. Build from the recorded source commit and toolchain. Package only the independent
   executable, the launcher, instructions, checksums, manifest and necessary
   third-party notices. Never add original archives, extracted assets, original
   binaries, reference screenshots, dumps or user profiles to release assets.
5. Generate the README/Dashboard status and checklist. Release notes must state
   the features accessible from the menu, their limitations, data compatibility,
   changes since the last preview and exact validation performed.
6. Prepare the draft locally and inspect its archive and dependency notices. Create
   a hosted draft only after owner authorization, then publish only when its
   distribution review passes. MIT is selected within the scope of NOTICE.md;
   provenance and distribution review remain open. A draft does not imply clearance.

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

The build receipt pins a source commit and hashes the copied executable. The
metadata contains local dependency paths and remains private. Only the filtered
inventory and applicable notices enter the archive. Current packaging deliberately
refuses non-draft output until the public-release review is resolved.

The bundle includes NOTICE.md to preserve the project license scope and method
statement. The verifier reads the ZIP without extraction or execution, checks the
exact member allowlist, executable identity, manifest hashes, internal SHA256SUMS
and external archive checksum. It requires the current uncleared draft state;
passing integrity checks does not grant distribution rights.
