# Owned cold Delphi special phase

Further native acquisition and implementation follow the central
[reconstruction method](RECONSTRUCTION_METHOD.md), including prototype PS2 name
lookup and separate PC identity, aliases, platform differences and attribution.

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

## Evidence and scope

Thirty-two exact-reference CPU cases compare 127,692 normalized words: every
actor header, the full owner payload and all fifteen ordered registration lists.
Cases vary pose, scale, finite copy parameters, group size, local-player identity
and the supplied charge declaration. Original constructor, special, held-object
and declaration-reader instructions execute in the acquisition. The Rust side
constructs its own actors and lists; native state snapshots are not replayed as
inputs. Private reference inputs and reports remain outside the public source.

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

This phase does not execute player type change, shared player reset/activation,
copy or held-object updates, animation/physics, camera/HUD/audio, water behavior,
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

Integrated source `6ead6a0088ef01d0aef700ec45a6fc2e7b03cb05` retains the
parallel Mecc/Kabuto/cinematic changes. Its isolated workspace passes 707 Rust
tests (17 ignored), 20 Python tests, Rustfmt, Clippy and the source-data control.
The normalized comparison is rechecked against this combined source, with
source and executable hashes captured before and after the run.

Source `5187904951cd049d4db3532c5082d891a4b01765` additionally retains the
cinematic inventory/diagnostic changes. The game-crate source is unchanged from
the compared runtime above. The combined workspace again passes 707 Rust tests
(17 ignored), Rustfmt and Clippy, with all 30 current Python/site tests passing.
