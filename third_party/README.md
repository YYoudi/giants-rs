# Dependency notice review

The inventory covers 338 conservative normal-dependency packages reached from
the Windows GNU application metadata, including transitive build requirements.
Root/declared crate license texts are collected in THIRD_PARTY_NOTICES.md.

Some published crates omit workspace notices. notice-sources.json records exact
crate versions, VCS commits, upstream raw URLs and SHA-256 values for 27 additional
texts covering 16 such packages. The packager verifies those receipts before use.
It never modifies the Cargo registry or assumes that a successful filter clears rights.

Unresolved: constgebra 0.1.4 declares MIT OR Apache-2.0 in package metadata, but its
published crate and recorded upstream tree contain no license text. Do not invent
the copyright notice or treat its SPDX expression as a completed notice review.
The compiler/MinGW runtime redistribution notices also need explicit review.

This is a technical inventory, not comprehensive human provenance review or a
legal opinion. The owner has not yet selected a license for project source.
