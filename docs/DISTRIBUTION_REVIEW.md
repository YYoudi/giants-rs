# Windows preview distribution review — 4 November 2026

Current work selection and delivery reporting follow
[GOAL_GAMEPLAY_FIRST.md](GOAL_GAMEPLAY_FIRST.md). Prioritize actual testable
interactions and their blockers. This direction does not expand any historical
review, redistribution authorization or technical evidence recorded below.

The owner explicitly authorized downloadable Windows previews and removed the
mandatory human-release-review gate in this chat. This is an AI-assisted technical
distribution review, not a human provenance audit or a guarantee of third-party
rights. The approximately 99% AI-generated estimate and absence of comprehensive
human review remain unchanged. Linux and macOS downloads are future work.

## Package scope

The ZIP contains the independent Rust/Bevy executable, launcher, documentation,
MIT project license within NOTICE.md's scope, third-party notices, dependency
inventory, manifest and checksums. Original game assets, programs, profiles,
captures and private research are excluded. The executable loads the user's own
installation read-only. Public source review is recorded in PUBLICATION_REVIEW.md.
The desktop preview adds independent launcher/profile/menu code. The locked
dependency graph is unchanged. Windows shell/COM folder selection uses system
APIs already covered by the Windows runtime; no extra library is redistributed.

## Dependency notice treatment

The normal-dependency traversal conservatively includes transitive packages.
Root notices plus hash-verified supplements are collected for each package.
Missing license text or metadata remains a packaging error for public output.

For constgebra 0.1.4, upstream commit
`eae8e094e5779f42f1db2a7adf9036aa33744bc8` explicitly declares
`MIT OR Apache-2.0` in Cargo.toml. The Apache-2.0 alternative is selected.
The package includes that unmodified upstream declaration and the full license
text from the Apache Software Foundation, with provenance and hashes. The crate
and upstream tree contain no NOTICE or explicit copyright header to preserve;
no copyright holder or year has been invented. This records the technical basis
for exercising the declared license option, rather than treating a missing
LICENSE filename as an automatic prohibition on redistribution.

Sources: [upstream declaration](https://github.com/knickish/constgebra/blob/eae8e094e5779f42f1db2a7adf9036aa33744bc8/Cargo.toml),
[Apache license terms](https://www.apache.org/licenses/LICENSE-2.0.txt), particularly
section 4's license-copy and applicable attribution/NOTICE retention conditions.

## Compiler and runtime scope

The initial downloadable build uses Rust 1.98.0 GNU Windows x64 and the WinLibs
GCC 16.2.0 / MinGW-w64 14.0.0 UCRT toolchain. No compiler, linker or toolchain
executable is shipped. The PE import inventory contains Windows system DLLs and
UCRT API sets; no extra non-system DLL must be bundled for this build. This does
not exclude statically linked runtime code.

Conservatively include all recorded MinGW runtime notices, AUTHORS, winpthreads
notice, GCC GPLv3 and Runtime Library Exception 3.1, and Rust MIT/Apache/COPYRIGHT
notices. Include the complete standard-library copyright/license inventory
shipped with this Rust release, converted from HTML to text without omitting
entries, to retain third-party Rust runtime attributions and exceptions.
MinGW's own licensing instructions recommend keeping the complete runtime
notice collection rather than attempting to remove supposedly unused notices.
The standard GNU Rust/MinGW compilation process uses no proprietary GCC
intermediate-representation plugin. GCC runtime inclusion is assessed under its
Runtime Library Exception, not by licensing the whole application under GPL.

Source versions and hashes are in third_party/toolchain-notice-sources.json.
The review applies only to an explicit matching build receipt; a future compiler,
linker or dependency change requires a new receipt-bound assessment.

## Publication checks

Before publication, run synthetic checks, build the recorded source revision,
verify CLI startup and both silent offscreen development lifecycle endpoints,
create a review receipt binding source, executable, compiler, linker, license,
notice catalogue and this document, then package and independently verify the ZIP.
Publish the reviewed source/tag, ZIP and adjacent checksum. No raw executable is
offered separately from its notices. A preview is explicitly incomplete: manual
controls, campaign completion and fidelity are not certified by these checks.
