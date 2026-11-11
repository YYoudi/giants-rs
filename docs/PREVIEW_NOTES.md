# Windows development preview - preview-2026.11.11.3

Runtime source: `5dbebce3a7a37bd5be2e61ef4a0107277b2815d0`.
Executable SHA256: `76541755c6688aeae2bb310e25d51743c979a8309574905c5a32fb9a16d4eb1f`.

Supply your own compatible PC installation. No original game assets, original
executables or private research are bundled. The ordinary partial Mecc controller
is the shared menu-first runtime; no script enables these supported actions.

## Try the Vimp interaction

Use **Preview tools -> Browse all missions -> Mecc -> Starving!** (Story2),
enter the Mecc preview and skip the opening with Escape. Normal profile defaults:
WASD movement, mouse aim, right mouse thrust, Space nitro while thrusting,
left mouse partial pistol, R view, Escape menu. F8 shows the nearest herd's
distance/turn hint. The authored start is roughly 2,000 source units away.
Approach the herd, walk into a Vimp and shoot it. Growing adults can require
multiple hits. No teleport, lowered damage or forced actor pose enables this path.

Current completed collision meshes feed player and projectile contact queries.
Actual hits apply source-bearing damage/recoil, hurt/recovery, danger responses,
attached blood and positional/owner-bound audio. Death creates source FX,
the animated ghost and tiered meat drops; their actual owners update and retire.
Attached loops follow/stop with their owner. A dropped mesh is not collection:
meat pickup, feeding, mana transfer and their economy remain unavailable.

This remains a partial component. Other staged AI is inactive. Complete native
world admission/scheduling, mission objectives, linked/controlled variants,
the full Mecc primary and complete visual/audio fidelity remain open. Death FX
explicitly use host-frame scheduling, terrain-only contacts and splat retention
for their source owner's lifetime. No complete native state78/decal callback is
acknowledged. Delphi/Kabuto retain initial-body inspection rather than gameplay.
The independent cinematic census remains pinned to .7.

## Validation and boundaries

Portable checks: 978 Rust passes / 168 ignored, 45 Python passes, formatting,
strict workspace/all-target Clippy and source-history confidentiality review.
Scoped installed-data controls exercise the combined source services and
120 Hz hurt with an active danger response, ownership, expiry and re-entry.
Their external release provider and supplied clocks remain explicit boundaries.
Forty original owned-audio starts separately exercise loop and same-owner/sample
reuse with supplied allocation, position upcall, device returns and audio seeds.
No new prototype PS2-to-PC attribution is confirmed; recovered names are not
implementation or fidelity proof. Selected names are searchable in the sources
through docs/SYMBOL_NAVIGATION.md and the central reconstruction method.

On this exact executable the graphical contact passage records 3534 player
responses against actual living Vimp meshes, without shots or pose injection.
The hunt passage records 5 injuries and 3 deaths, uploaded blood,
rendered FX/ghost/meat and actual muted injury/death sinks. A separate first-death
capture and near-herd service cancellation pass. Three common Mecc cycles,
actual terrain landing/dust/audio, two lifecycle modes and full selected cinematic
source-terminal/re-entry transport pass. Final shader pipelines resolve.
These graphical passages use supplied controller commands; they are not OS-held
input, audible parity, native timing or complete scene-fidelity verification.

The normal menu rendered during the Windows input attempt, but safe target
selection failed while another foreground application covered it. Desktop input
was stopped after one recovery. Keyboard/mouse gameplay and normal OS return/
re-entry on this executable remain unverified; prior-build manual evidence is
not transferred to it. The graphical ordinary-controller checks remain separate.

The package carries 338 dependency and eleven toolchain notice entries, with
29 system DLL imports and no technical distribution blockers. Earlier previews
and captures retain their exact identities. The owner estimates approximately
99% AI-generated project content with no comprehensive human review; this is
not a completion measure. Linux and macOS downloads remain future work.
