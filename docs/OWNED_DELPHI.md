# Owned Delphi construction, physical state and animation

Further native acquisition and implementation follow the central
[reconstruction method](RECONSTRUCTION_METHOD.md), including prototype PS2 name
lookup and separate PC identity, aliases, platform differences and attribution.

## Manual rotation kernel - 6 November 2026

`player_rotation::advance` implements the acquired finite manual heading/look
helpers with supplied object time, sensitivity, ownership and class rates in the
measured Double53 profile. A 2,531-case PC/Rust scalar comparison matches 488,483
complete header words. It preserves wide upper-heading correction, stored-float
lower correction, nonzero-input look clamping and the distinct pause behavior.
Automatic target heading is refused. Prototype PS2 attribution remains unknown.

`Resources::with_manual_rotation` now fixes this producer into a fresh owned
locomotion profile. `Session::advance_delphi_controlled` executes entered prefix,
manual heading/look, jump, physical/first ANM/history, dry sword secondary and
zero-time ANM in source order. Look is actor-owned independently of body angles;
class rotation rates and actual object-local time are consumed directly.

Five continuous 256-step PC/Rust controls match 10,202,880 normalized words over
all 22 actors, reference buffers, owner payload and fifteen ordered lists, plus
shared RNG endpoints. They cover signed turn/look, heading wrap, local/nonlocal
look limits, pause/unpause, movement/jump, variable global time and ordered
slow-area boundaries with owner exclusion. Native snapshots remain outputs only.
The paused physical branch retains first ANM/history while skipping integration,
placement, new model rates and force production. Jump launch is pause-gated;
existing hold/release semantics remain separate.

The source-data lifecycle control now interleaves three owners across physical,
locomotion and controlled profiles. It checks separate look state, pause without
terrain queries, wrong-entry refusal and retained state after invalid rotation.
Full primary/camera/target/item and auxiliary callbacks, nonordinary effects,
water/turbo, mission input and live admission remain pending. These CPU controls
add no playable capability or verified visual fidelity and are not a full
controller or scheduled-world proof.

`Factory::construct_player_fields` adds the acquired cold local-player field
projection around the special phase. It restores class scale, resets pitch,
transfers supplied position history, creates the special group, selects the
initial animation through the shared channel service and sets the activation
field. It returns a separate, explicit projection-reset requirement; no renderer
acknowledgement or live player scheduling is invented. Prototype PS2 names were
searched against the pinned symbol table; attribution remains unknown pending
independent algorithm and caller review. Detailed records remain private.

`delphi_resources::Resources` prepares the kind-3 mission marker and immutable
class/state specifications. `delphi_special::Factory` now also constructs an
ordinary owned class-3 actor and executes its acquired cold special phase. These
are separate stages; neither acknowledges a fully initialized local player.

The special phase creates the declared auxiliary-copy group and its held
objects, sets their typed source/attachment relationships and parameters, resets
the initial equipment slots and constructs the owner's initial held object. It
uses the existing held-object replacement kernel with Delphi-specific ownership
checks. Mecc player/fire/footprint admission is unchanged.

For the supplied normal data, this is an owned cohort of 22 actors. Copies are
registered in their actual constructor order. Later field changes do not silently
reclassify list membership. Native addresses remain typed owned identities and
resource references; none are stored as emulated pointers in the Rust payload.

The cold player entry takes a supplied finite frame. It does not destroy or
retype an arbitrary existing actor. Invalid input is rejected before allocation;
special-phase failures retain the existing quarantine behavior. The normal
special-only entry still leaves player selection and activation untouched.

## Evidence and scope

`delphi_animation_definitions::Definitions` and
`delphi_physical::Resources::load_locomotion` now read the required animation
definitions from an explicit user-selected executable-data file. The bounded
initialized-data reader is shared with Kabuto; its existing three layout profiles
are preserved. Delphi accepts the two independently inspected layouts, retains
94 rows and both equipment columns, and refuses missing required definitions or
other unacquired layouts. No original table or executable is bundled or run.

Both inputs match all 188 identifiers at their independently inspected data
references. The same six persistent motion/ANM comparisons pass through the public
loader; a separate replay with retail metadata retains the reference behavior.
This is data agreement against the acquired PC behavioral profile, not execution
proof for every executable build. Header tuples identify layouts, not file
authenticity. The complete primary/camera/target and live controller remain pending.

`delphi_locomotion` and `Session::advance_delphi_locomotion` now compose the
acquired ordinary living dry sword selector with owned physical state. Prepare
the physical resources with `with_locomotion(clips)` and use that profile's
explicit entry; the physical-only entry retains its earlier scope. Supplied
clip identifiers must resolve in the actual owned class. The service embeds no
original executable mode table. `load_locomotion` supplies definitions through
the read-only metadata provider; full controller integration still gates admission.

The order is physical/jump/first ANM, physical history, secondary selection and
zero-time ANM. It preserves the jump latch across changed/released inputs,
shared random idle/walk decisions, current-mode retention when a mapped clip
does not change, and canonical references/stamps. Six independent 256-step
native/Rust cases compare 12,243,456 normalized words over all 22 actor headers,
reference buffers, complete owner payload and fifteen ordered lists, plus final
shared RNG states. Both sides use real installation animation bounds/definitions,
their own persistent trajectories, supplied flat terrain/time/commands and three
explicit random seeds, continuous forward/side/back movement and deferred foot
sound-block preparation. Native outputs are not replayed into Rust.

Those controls execute the original entered prefix, jump, physical/history body,
secondary dispatch and second ANM. They exclude the surrounding full primary,
camera/target/item producers, other actor callbacks and general world scheduling.
The broader native-only acquisitions below remain separate. No live controller,
water/turbo, nonordinary landing effects, combat or fidelity is certified.
Four synthetic controls check latch/contact behavior, shared-clip semantics,
idle draw ordering and refusal-state preservation. The silent source-data
ownership control also covers both phase profiles with external clip definitions,
including a late secondary failure that retains payload/references and quarantines
the owner. These are owned-backend guards, not native error-path equivalence.

Combined source `b53e7b28d7705fefa8311848e67fae66691dca95` preserves the latest
Mecc subjective and Kabuto cache-lifecycle work. All six comparisons pass again
with stable source/runner/input hashes. The workspace passes 736 Rust tests
(22 ignored), 36 Python tests, Rustfmt, Clippy, the source-data ownership control
and site/JavaScript checks. Source-history review reports zero findings across
167 commits and 1,809 unique blobs.

Its exact GNU build has a private ten-member validation draft with complete
dependency/toolchain notices and matching receipt/review, without technical
package blockers. Executable SHA256:
`446aa2bb7a6cfd70fb8b9f9a3038f0ed82f818255147eba8b6ef7ec45fdb5c56`;
draft ZIP SHA256:
`9139daab22ad347686660f4d26db4821df39fada88020d4224045f50c2b9b7d4`.
The public download is unchanged by this CPU-only stage.

That exact executable also passes both silent common-runtime lifecycle endpoints
and three selected-opening cancellation cycles, without a window or audio device.
These exercise the menu/Mecc/cinematic lifecycle, not Delphi input, active effects,
complete playback or visual fidelity.

The subsequent Kabuto-texture merge at `4503331` retains identical Delphi,
shared ANM, effect-initialization and RNG source. It passes 737 Rust tests
(22 ignored), 36 Python tests, Rustfmt and Clippy. This later source identity
does not replace the exact source in the earlier executable/package receipt.

Native-only continuation records the complete selected-owner primary/secondary
flow around the acquired physical body. Observers retain 3,009 direct primary
calls and 576 phase calls while preserving the original 64 final cohort states.
The character secondary changes mode and clip after physical history storage,
before the second zero-time ANM pass. Delphi retains some jump states across
command changes; Mecc's locomotion selector is not a substitute.

Three 256-step original-instruction controls additionally cover landing, return
to rest and both idle variants under explicitly supplied random seeds, with 22
actor headers/reference buffers and the full owner payload observed each step.
The offline Windows TLS/error API boundary supplies one thread context; original
CRT lookup and RNG instructions execute. An earlier missing TLS environment was
diagnosed and corrected without replacing a game RNG result. Prototype PS2
attributions remain unknown. These 768 steps are native acquisition, not a Rust
full-primary comparison, complete world scheduling or live gameplay proof.

The next implementation must compose the acquired secondary, heading/look/pitch,
camera/target inputs, energy/item timers and second ANM with owned state. The
existing Rust comparison below still covers only its stated physical-phase scope.

The latest Kabuto cache integration at `2b3fd30` preserves the Delphi and shared
ANM source. The combined workspace passes 731 Rust tests (20 ignored), 36 Python
tests, Rustfmt, Clippy and site/JavaScript checks. No new executable is produced
by this acquisition/documentation stage; earlier build receipts retain their
exact source identities.

`Session::prepare_delphi_physical` and `advance_delphi_physical` now compose the
ordinary dry physical body with the acquired jump service and owned persistent
motion. Integration, position limits, terrain/world contact, affine preparation,
the first animation pass, new model rates, gravity/control forces and the actual
80-record history ring execute in source order. The animation pass consumes the
preceding rates. Class event descriptors are shared across prepared Delphi actors;
reference buffers remain actor-owned. The common ANM implementation is reused by
Mecc through its existing adapter, preserving its selection and admission rules.

Sixty-four continuous steps match 510,144 normalized words over all 22 actor
headers/reference buffers, the complete owner payload and all fifteen ordered
lists. The native control applies real class animation bounds before execution.
Both sides build their own state and receive the same commands, timestep and
flat terrain; native velocity, contacts and trajectories are outputs only.
The comparison executes the entered primary prefix, jump and physical body,
including the first ANM pass. It does not execute the full primary mode/clip
selector, second ANM pass, other actor callbacks or a complete world frame.

A separate silent source-data control runs two owners against shared class
resources, checks falling/contact/jump, duplicate preparation refusal, failure
quarantine, retained payload/effect membership and Session cleanup. These backend
guards do not claim native error-path parity. Nonordinary controls, immersion,
turbo and unacquired context producers are refused. Prototype PS2 lookups retain
unknown attribution; no prototype ABI/layout is imported.

Full primary/secondary composition, copy updates, water/turbo, selected-mission
admission, camera/HUD/audio and combat remain prerequisites. This CPU milestone
does not make Delphi playable or establish visual fidelity. Earlier standalone
results below retain their original, narrower scopes.

Combined source `50472dc895370f9b8ec6a0d552fcc92fd3530187` preserves the latest
Kabuto cold group/physical refresh and shared subjective-reference work. The
64-step native/Rust comparison passes again with stable source/runner hashes,
as does the two-owner source-data control. Its isolated workspace passes 728
Rust tests (20 ignored), 35 Python tests, Rustfmt, Clippy, site/JavaScript checks
and source-history review (145 commits, 1,693 unique blobs, zero findings).

Its exact MinGW build has a private ten-member validation draft with complete
dependency/toolchain notices and matching receipt/review, with zero technical
package blockers. Executable SHA256:
`a405ed6f6908570c32186f128bad88f644b8bd86aa7de4039a6bc8854844bd0f`;
draft ZIP SHA256:
`84bf09999ddbeef66027408a9891f3d090307959b505f31e767f8a4bf6a94863`.
This draft does not acknowledge live Delphi control or change the public download.

The exact executable also passes both silent common-runtime lifecycle endpoints
and three selected-opening cancellation cycles with no window or audio device.
They exercise the common runtime, not this Delphi API, manual controls, complete
cinematic playback or fidelity. The later research-method tooling merge retains
identical runtime source and passes the 36 current Python/site controls; its
source identity is separate from the original build receipt.

`delphi_jump::step` adds the acquired standalone jump/hold/release service. It
preserves the distinct impulse and launch-latch gates, hold acceleration and
countdown, release flags and stored height. The inline physical request chooses
the smaller value unless its current value is zero.
Object-local time uses the shared ordered-area clock, with no fixed timestep.

Eight hundred sixty-four scalar cases and 64 persistent sequences of 32 steps
match 3,543,904 normalized header/payload words. Original jump, request and time
instructions execute. Both implementations carry their own state between steps;
native output snapshots are never fed into Rust. Supplied boundary inputs cover
commands, launch blocking, mode/contact, rates/orientation, position/vertical
speed, two context words, declarations and ordered time areas. The controls
include owner exclusion, area ordering and zero/varied timesteps. Unrelated seeded
header/payload words are compared too. Original helper calls are retained in the
private acquisition trace; no behavior return is substituted.

This is not a gravity/trajectory, terrain or immersion producer, complete primary
turn or Session/live-controller integration. Those consumers remain prerequisites
before admitting gameplay. Finite-profile and clock guards are owned refusals,
not claims of native NaN/error-path parity. Prototype PS2 lookup was performed;
routine attribution remains unknown pending independent prototype algorithm
review. The PC debug behavior, not a prototype layout, defines this service.

Private jump REA evidence:
`ev_81edd7f516b102ff9f375ac159cd56604a7513dce54e9ec99e66d6fb4059e124`;
request helper:
`ev_bd13a1f19cef8f0c3769c135a72b17fb3b32bd885f86008246de9198f0ca116f`.

The combined source at `0c0139d6212b7bdf5e3312596734b4f3e20492a2` repeats all
2,912 jump controls with stable source/runner hashes and passes 725 Rust tests
(17 ignored), 35 Python tests, Rustfmt and Clippy. The common build retains the
latest Kabuto auxiliary geometry, Mecc and cinematic work. Session consumption
of the jump kernel remains a prerequisite, not an acknowledged player frame.

The exact GNU build for that source passes both silent common-runtime lifecycle
endpoints and three selected-opening cancellation cycles without a window or
audio device. Its asset-free private ten-member draft passes receipt, notices and
package verification without technical blockers. Executable SHA256:
`df0a96c8f8d833a6cd00355b6d4fcd733285daec278a05a16ed3dc3a743fd34c`;
draft ZIP SHA256:
`64efdf4516892d86270fb8444cb983978a8f77f38c06d7b2fabbe5fd144f88c0`.
These common-runtime checks do not execute the standalone jump service.
The subsequent Kabuto-auxiliary merge at
`60756cedcd4584974cdfad9486cc7979dea55c02` retains unchanged jump/request/time
source and passes 725 Rust tests (18 ignored), 35 Python tests, Rustfmt and
Clippy; its source identity does not replace the earlier build receipt.

`Session::initialize_delphi_physics` now owns the first-primary physical setup as
a distinct stage after special construction. `delphi_initial` loads the actual
seven scalar declarations and signed limit, restores inline defaults through
`physical_initial` (also used by Mecc), preserves the opaque physical word, stores
the current scale and clears controls. The native payload/physical link is an
owned presence flag, never an emulated pointer. Replay and unrelated/live-profile
inputs are refused; clearing the Session releases the group and physics together.

Two hundred fifty-six native/Rust initializer controls match 311,552 words across
the complete seeded headers and payloads. They vary all declared parameters,
scale, arbitrary dirty payload/header words and signed limits, including negative
values and the upper clamp. Original initialization instructions execute without
behavior substitutes; only the asserted payload/inline links are normalized.
The seeded payloads are supplied inputs, not native-output replay. Separate
source-data checks exercise real resource-backed Session ownership and replay
refusal. This initializer does not execute movement, frame scheduling or a
complete primary callback. Native refusal/error-path equivalence is not claimed.

REA initializer evidence:
`ev_26038f778a560c7c81c4b8271dbafcb03bfc74182e23adbf114d9b47a76db515`.
Prototype PS2 lookup remains an unknown attribution; no prototype ABI/layout is
imported into this PC contract.

Combined source `ea575f0d5500d5739a396203c07a02305e47b16f` repeats all 256
initializer cases with stable source/executable hashes and the real parameter
reader, and passes 719 Rust tests (17 ignored), 35 Python tests, Rustfmt, Clippy
and the resource-backed physics ownership/replay/clear control. Mecc camera and
dispatch plus Kabuto size work are retained in this shared build.

That exact GNU build also passes both silent common-runtime lifecycle endpoints
and three selected-opening cancellations. Its ten-member private draft verifies
complete notices and matching receipt/review with no technical package blockers.
Executable SHA256:
`62c658b7e8c9f276b6e8ef9864ea2b7e3ca0df75d5b814c92b3558130226e7bc`;
draft ZIP SHA256:
`e915e5038d95cc0158bf1aacb35ca05ae40364d68afa607d9ba0bef01509d177`.
These regressions do not execute Delphi control or establish visual fidelity.
The subsequent cinematic-FX merge at
`01ef47e9c4caf0bd01c829a658c1e6f04c0aab78` retains unchanged Delphi physical and
shared actor source and passes 720 Rust tests (17 ignored), 35 Python tests,
Rustfmt and Clippy; its later source is not substituted into the earlier receipt.

Thirty-two exact-reference CPU cases compare 127,692 normalized words: every
actor header, the full owner payload and all fifteen ordered registration lists.
Cases vary pose, scale, finite copy parameters, group size, local-player identity
and the supplied charge declaration. Original constructor, special, held-object
and declaration-reader instructions execute in the acquisition. The Rust side
constructs its own actors and lists; native state snapshots are not replayed as
inputs. Private reference inputs and reports remain outside the public source.

An additional 32 cold generic-to-Delphi native/Rust cases match 123,792 normalized
words across complete actor headers, owner payload and all fifteen ordered
lists. They vary supplied position/current/previous/angles, discarded generic
shell scale, enabled/disabled animation blending, copy counts/parameters, initial
charges and viewport. Original generic construction, type change, projection
math, history, special phase and animation selection execute. Only external
projection submission is intercepted; its finite matrix is retained privately
but not compared to a Rust renderer. Rust receives the supplied inputs, not
native actor snapshots. These controls do not cover arbitrary dynamic switches,
UID-managed multiplayer, mission producers, live ticking or graphics fidelity.

Six additional original-instruction declaration-reader controls distinguish the
charge field from its neighbor, last-write behavior and declaration reset. The
initial slot reads the effective declaration rather than assuming unlimited
charges. Synthetic tests cover this decoding and the bounded finite copy
parameter contract. Singular/nonfinite copy profiles remain explicit refusals.

The source-data control also checks typed incoming references, Mecc-profile
exclusion, rejected reinitialization and complete owned-registry clearing. An
unbound held-list member exercises a late backend failure: partial work is
retained, the owner is quarantined and a second attempt cannot allocate again.
These are owned-backend refusal checks, not native error-path parity claims.

The special-only phase does not execute surrounding player field selection;
the new cold player entry covers that field subset, not a complete live type
change. Neither executes copy or held-object updates, animation/physics ticks,
camera/HUD/audio integration, water behavior,
combat or campaign admission. The main selected-player path still prepares a
Mecc controller only. Delphi campaign and movement remain `todo` in the runtime
roadmap. No playable feature or visual fidelity is claimed by CPU comparisons.

Private REA evidence identities: special dispatch
`ev_888c71a02f9092e68ca24d197226e6c376c6c0044b233de7e9bf1b3fd25a35c4`;
auxiliary creation
`ev_0b9057a2ff107be3d4940c18c81414b827a23212df79322fe60e85473d79cd69`;
equipment reset
`ev_23e32a60600f13b8989e666992f8537a5e56cbd1a90cbd57b6a288977342f5ad`;
slot initialization
`ev_97ee354f10731048b1f728301084c4dfa31384ede145bd1cc27654cb9eb4bb7c`;
default getter
`ev_1707de0530ba5a8dfd93affa5b6ae7e65fc12aa2949f18ef789d4252ad83bc83`;
declaration reader
`ev_c91022ce991a3c2aad05c61700b742b707099a4a70e080c8b1ec3687ea42e2a6`.
Static inspection and controlled execution are distinct evidence.

Cold-player wrapper evidence:
`ev_7c2221184c98a112a75e98586915d666730afe83184441c413781af0539517d2`;
initial channel selection:
`ev_e35d0b1a855c48844a3f24c0a7fd0daf40e0590f8d6c1035a5ec5be162674d38`;
history transfer:
`ev_4590785fe8aba76794f5d596f8774b8a7cfd49210f36854d366b5d5a4e0a2e20`.
The combined camera/Kabuto/Delphi source at
`facf988e7b868ed1225ee45da8ec825ea678d20f` passes the repeated 32-case comparison,
714 Rust tests (17 ignored), 35 Python tests, Rustfmt, Clippy and the source-data
refusal/cleanup control. These cold-field results do not enlarge runtime admission.

The receipt-bound GNU build for that same source also passes both silent common
runtime lifecycle endpoints and three selected-opening cancellations. Its
private ten-member draft passes receipt/notices/package verification without
technical blockers. Executable SHA256:
`8e4f4f161b1e51021a5d40d7980065c8eebc96090f3621ddeb5f2acf58652c83`;
draft ZIP SHA256:
`5c90d0a864e21872950fe7d9ef12f3160c1739a26b9b6c6e4e3050bae649e9a5`.
These regressions do not call the Delphi factory or validate manual controls,
complete playback or visuals. No window or audio device is created.

Integrated source `6ead6a0088ef01d0aef700ec45a6fc2e7b03cb05` retains the
parallel Mecc/Kabuto/cinematic changes. Its isolated workspace passes 707 Rust
tests (17 ignored), 20 Python tests, Rustfmt, Clippy and the source-data control.
The normalized comparison is rechecked against this combined source, with
source and executable hashes captured before and after the run.

Source `5187904951cd049d4db3532c5082d891a4b01765` additionally retains the
cinematic inventory/diagnostic changes. The game-crate source is unchanged from
the compared runtime above. The combined workspace again passes 707 Rust tests
(17 ignored), Rustfmt and Clippy, with all 30 current Python/site tests passing.

The receipt-bound Windows build at
`6012206836aae5c594dd5250e632db604b5fa274` passes both silent development
lifecycle endpoints and three selected-opening cancellation cycles. These
regressions exercise the common menu build, not the Delphi special API, manual
controls, natural cinematic endings or visual fidelity. No window or audio
device is created by these controls.

An asset-free private draft has ten allowlisted members, complete notices,
matching build/review identities and no package-verifier blockers. Executable
SHA256: `c227099f295063bc6827573884c41da7a0cc33c41977d87e56d5e547fcfaf3dc`.
ZIP SHA256: `37880c59eb20426b8c5bdb7c1f6dfaa44d3d862a5efea883644cc3351880fd42`.
It remains a private validation draft; the public download is unchanged by this
CPU milestone.
