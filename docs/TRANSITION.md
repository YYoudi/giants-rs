# Working-repository transition

Intended destination: `YYoudi/giants-rs`, fresh history. This local repository has
a private GitHub staging remote and is not approved for public exposure yet.
MIT was chosen under the owner's delegation; provenance/distribution review
remains open. Do not expose this candidate publicly.

After explicit owner approval, create the destination, push this reviewed history,
and use it directly for all runtime development, roadmap updates and releases.
The previous private repository remains a research archive with its history and
visibility preserved. Do not mirror ongoing runtime changes into two authorities.

Existing discussions must move explicitly to new checkouts and codex/ branches.
Do not relocate, reset or delete their existing worktrees. Preserve outstanding
patches and private evidence in the archive; review and port only independent
source changes against this baseline. Never merge the old Git history here.

Candidate baseline preserves recent main source plus uncommitted owned-target
Rust implementation. Unmerged cinematic/actor-reference work is retained in
private backup and per-discussion handoffs, not silently marked integrated.
Research observers, CPU replay examples and reference inputs stay private.

## Cutover sequence (only after approval)

1. Recheck all worktree heads and dirty-file hashes against the private handoffs.
   Back up and review changes made since the candidate freeze.
2. Apply the owner's chosen source license where the owner can grant rights;
   retain dependency notices and resolve release blockers separately.
3. Create/push the reviewed destination and validate a remote clean clone.
4. Create explicit new discussion checkouts/branches from that destination.
   Reapply reviewed pending patches; keep originals available for comparison.
5. Update persistent tracking in both repositories and each discussion's handoff.
   The new repository becomes the only runtime, roadmap and release authority.
6. Publish a build only after its manifest, notices and distribution review pass.
   Keep research documents and original data outside public history and assets.
