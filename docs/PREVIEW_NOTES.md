# Windows development preview - preview-2026.11.11.2

Runtime source: `0da06b449ed83394ec17ff9d2b1ca2ba945ee9c9`.
Executable SHA256: `891b16220b9609e062bf48ae0ec6eb8d0dba234c87e39440450dacaf19dcfc56`.

Supply your own compatible PC installation. No original game assets or original
executables are included. The ordinary partial Mecc controller remains the
shared menu-first runtime; no script enables its normal supported actions.

## Try the first living interaction

Use **Preview tools -> Browse all missions -> Mecc -> Starving!** (Story2),
enter the Mecc preview and skip the opening with Escape. Use the normal profile
bindings: WASD movement, mouse aim, right mouse thrust, Space nitro while
thrusting, left mouse partial pistol, R view, Escape menu. The F8 hint gives the
nearest Vimp distance and turn direction. Approach a herd and repeatedly shoot
nearby: accumulated actual player noise can trigger the source startle animation
and its return to wandering. The authored start is roughly 2,000 source units
from the nearest herd; the test does not teleport actors or invent a spawn.

The Story2 first-interaction component uses actual source herd/marker children,
mission ownership, player position/clock and pistol noise, shared partial-runtime
RNG, completed poses and positional sound requests. It renders through the
existing canonical NPC mesh consumer. The first positional cue loads through the
existing sound registry and a real sink acknowledges playback. Pending fire and
nitro presses are cancelled on focus/menu loss.

This is an explicitly partial component. Other staged AI remains inactive.
Complete native-world scheduling/population, platform contacts, linked targets,
feeding, combat damage, meat collection and mission objectives remain open.
The owned slow-area namespace starts empty because normal input exposes no birth
action; this does not certify absence of native slow areas. Native clock/RNG
interleaving and complete visual/audio/behavioral fidelity remain unverified.

Delphi/Kabuto retain initial-body inspection rather than gameplay. The selected
cinematic transport remains the explicit preview scheduling policy; the separate
cinematic census remains pinned to .7. No new confirmed prototype PS2 attribution
is established. Scores or recovered names are not behavioral proof.

Earlier previews and captures retain their exact identities. The owner estimates
approximately 99% AI-generated project content, with no comprehensive human
review; this is not a completion measure.

## Validation

Portable checks: 965 Rust passes, 162 ignored; 44 Python passes; formatting,
strict workspace/all-target Clippy and source-history confidentiality audit pass.
Installed controls separately exercise two actual Story2 tickets, each with 6,000 supplied
frames, 14 Vimps, 259 startle entries and 253 returns, real terrain and player
noise, sound requests and cleanup. These are CPU checks with supplied commands.
The exact executable passes the Story2 supplied-command approach/fire GPU
probe: 14 startle entries, 7 returns before capture, canonical rendered Vimps
44 units away and real muted positional sinks. Three common Mecc cycles cover
admission, movement, pistol, normal jet/nitro/release, audio cleanup and zero-entity
returns. The actual near-herd reaction also passes service cancellation with
mission/audio cleanup and zero retained mission entities. Separate actual
terrain landing/dust/muted positional audio passes.
Two runtime lifecycle modes and full selected cinematic source-terminal/re-entry
transport pass. All final shader pipelines resolve. These are supplied-command
or service-level checks; actual OS-held flight and near-herd interaction have not
been verified on this executable. Earlier actual-input evidence retains its own
build identity. No new complete mission or full native fidelity is certified.
