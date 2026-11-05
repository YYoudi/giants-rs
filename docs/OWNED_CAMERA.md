# Ordinary owned Mecc camera

Further acquisition follows [RECONSTRUCTION_METHOD.md](RECONSTRUCTION_METHOD.md).
The normal-camera and temporal-approach routine identities are confirmed there;
wall/profile correspondences remain probable and the renderer-cache counterpart
remains unassigned. Naming does not expand the behavioral evidence below.

## Producer and consumers

`mecc_camera_update::State` owns sequential ordinary-camera updates and its last
completed output. It produces direct-angle first/third-person profiles, bounded
terrain adjustment, retained chase scalars, the ordered prepared-mesh clip query,
source-precision view matrix and distinct cached viewport origin/forward.
Cold chase initialization occurs before providers run. Invalid/stale input calls
no provider; a provider failure quarantines the sequence and preserves the last
completed publication without replaying partially applied state.

`mecc_camera_wall::clip` retains supplied object/triangle order, eligibility and
initial segment bounds. It shortens the eye through actual point-in-triangle
tests with the acquired minimum-distance rule. Its `World` adapter uses the
same scene terrain and prepared rigid geometry as the owning mission driver.
This adapter does not produce the native object list, model/part admission or
skeletal transformations; those remain caller preparation and open prerequisites.

The mission-owned driver runs the producer after its real primary and held
updates. Actor position, heading, look pitch, scale, look height and flags come
from the completed owned state. The presentation frame retains the generated
cache with its frame number. Body rendering consumes that exact view matrix and
viewport origin; subjective presentation consumes the generated camera angles
and eye rather than reconstructing them from unadjusted player look input.
The ordinary third-person inspection aim also uses the produced camera.

The route is the existing explicit `owned-inspection` backend in the one
menu-first executable. Default development/campaign routes are not promoted to
owned readiness by this change. The driver still supplies `fire: None`; its
optional firing phase needs independently acquired camera/world/subjective
scheduling and positional audio. A post-primary rendered camera is not a valid
substitute for an unacquired pre-primary firing view.

## Validation and boundaries

On the pinned PC profile, 64 supplied camera cases match 2,688 normalized words,
including view/cache values and 1,025 actual floor queries, with a maximum of 50
in one case. They cover changing poses, profiles, frame deltas, scale/offsets,
terrain heights and floor-skip behavior. Eligible camera meshes are a known empty
cohort in these camera cases; final device uploads are supplied.

A separate 256-case prepared-mesh control matches 1,792 words: 294 candidate and
triangle visits, 281 inclusion tests and 248 hits. Geometry includes multiple
axis orientations and shears. Actual clipping callees run against explicitly
prepared cached-identity/unskinned geometry. This does not validate live object
ordering, nonidentity transform production, current part masks or animated mesh
admission. Earlier arithmetic mismatches were corrected before these comparisons;
exact source snapshots and private control receipts retain the boundaries.

Four synthetic tests cover invalid/stale input, floor/mesh failures without
republication or replay, the floor iteration cap/skip and unusable view bases.
The combined workspace passes 713 Rust tests (17 ignored), 35 Python tests,
formatting and Clippy. These checks do not establish visual fidelity, complete
world integration, camera configuration/smoothing, alternate characters,
vehicles/carried targets, shake or first-person firing scheduling.

Runtime validation and the receipt-bound draft are recorded at the corresponding
checkpoint in [RELEASES.md](RELEASES.md). Private game inputs, logs, captures,
comparison tables and original binaries remain excluded from public source and
packages. CPU proof and attribution are separate from that runtime checkpoint.
