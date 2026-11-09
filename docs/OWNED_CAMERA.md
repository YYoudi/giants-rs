# Ordinary owned Mecc camera

Further acquisition follows [RECONSTRUCTION_METHOD.md](RECONSTRUCTION_METHOD.md).
The normal-camera and temporal-approach routine identities are confirmed there;
wall/profile correspondences remain probable and the renderer-cache counterpart
remains unassigned. Naming does not expand the behavioral evidence below.

## Producer and consumers

### Current-owner and pre-world admission acquisition — 10 November 2026

Current camera ownership, camera reference-point status and subjective display
activity have separate lifecycles. A 162-case native prefix control checks owner
selection: an eligible previous owner is retained; otherwise the controlled actor
is the fallback, with an enabled nonnull override taking precedence. A missing
controlled actor exits before selection. The optional reference-point service is
an explicit supplied callback in this control. Its status word is not reset when
that service is skipped and is not the subjective display-active word.

A separate 864-case control executes the complete original ordinary-Mecc pre-world
entry, actual camera-owner getter, camera-admission helper and character predicates.
All pass, including sixty active copies. Each invocation clears display activity
first. Camera admission includes transition/mode/view state and a separate profile
byte with controlled-character eligibility; it cannot be inferred from a selected
first/third-person boolean. The current owner's ordinary model and subjective
model reference must also be present. The active branch preserves the virtual
representation's retained fields while copying the source header and changing its
model reference. Rejected branches retain the prior virtual header with activity
cleared. These controls supply current globals, valid actor identities and resource
memory; they do not replace downstream calls in the admitted Mecc branch.

This is native prerequisite evidence, not a new Rust equivalence or runtime
availability claim. The existing `State::prepare_from_session` still requires the
enclosing owner to decide admission. Acquire camera startup/mode/override writers
and their world ordering, then connect actual owner selection and model preparation
before passing a frame-bound `first_person::Display` to primary damage/fire.
Keep reference-point status separate, preserve inactive transitions, and do not
substitute the presentation player's camera mode or prepared meshes for this
pre-world result. Auxiliary-character preparation, full world traversal, drawing
and visual fidelity remain open. Prototype PS2 lookups for newly examined routines
remain unknown; no additional attribution is confirmed.

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

### Ordered subjective camera production - 6 November 2026

`mecc_world_camera::State` binds the ordinary producer to one Session source and
separates world-frame numbering from camera invocation numbering. An active
subjective frame performs an inner call after virtual animation selection and
before placement, first ANM/reference publication and fire. Its outer call runs
after post completion. The following frame cannot replace either unfinished
obligation. Only the matching completed outer publication is eligible for rendering.
The mission inspection driver now uses that outer publication and reads camera
inputs directly from the completed Session source.
`Publication::frame()` identifies the world frame; its output's `number` identifies
the camera invocation. Future consumers must retain that distinction when a world
frame contains both calls.

`Session::advance_mecc_subjective_post_produced` accepts no numeric camera packet.
It supplies placement with the previous actual camera globals and the generated
inner result, and replaces fire's camera/origin/forward before executing the real
callback. Known startup globals are an explicit boundary; an unknown initial
camera cannot enter subjective placement. Missing/stale owners or phases refuse
before providers run. A failed camera pass retains its last completed diagnostic
publication and refuses replay; it cannot become a new rendered view.

Four continuous native/Rust controls now pass 320 frames, 144 inner calls and 320
outer calls, twelve shots and 404,560 normalized state/cache/view words. Numeric
camera inputs in the private runner are deliberately invalid; Rust produces both
the prior immediate-fire view and the current subjective view. The controls retain
their original source/held/virtual references, projectile, FX, RNG and epoch checks.
A further 64-frame control captures the native inner renderer upload and matches
all inner/outer matrices plus the original state checks (79,666 words). This is
an additional observation of the same selected input profile, not another world.

Two synthetic sequence controls and a source-backed three-case lifecycle control
cover invocation/frame separation, unknown startup, wrong-order/replay refusal,
camera-provider quarantine, successful post/outer completion and a retained late
sound failure. The existing source-backed supplied-camera controls remain distinct.
Native camera eligibility/resource presence, known zero startup globals, flat
terrain/empty mesh population and partial outer-world ordering remain controlled.
The positional sound bank is empty. Main-driver subjective drawing/posed vertices,
complete target contexts and positional audio remain prerequisites; `fire: None`
is retained. These checks establish neither live mission firing nor visual fidelity.

### Earlier primitive and inspection controls

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
