# Dependency and runtime notices

The inventory conservatively covers 338 normal-dependency packages from Windows
GNU application metadata. THIRD_PARTY_NOTICES.md records dependency notices;
packaging regenerates them from locked metadata and hash-verified supplements.
No dependency in the current inventory lacks license text or metadata.

For constgebra 0.1.4, the Apache-2.0 option declared in its exact upstream manifest
is selected. The official Apache text and unmodified upstream declaration are
included with provenance; no copyright statement is invented. Rust's complete
standard-library notices and conservative GCC/MinGW runtime notice sets are
included in binary packages. See docs/DISTRIBUTION_REVIEW.md for the assessment.

The owner authorized Windows previews without a mandatory human review on
4 November 2026. A public package still requires a distribution review bound to
its source commit, binary, compiler, linker, license and notice catalogues. A
changed compiler or dependency graph requires a fresh assessment. This is an
AI-assisted technical process, not comprehensive human review or legal clearance.
