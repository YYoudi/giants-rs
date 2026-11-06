# Owned Mecc descent threshold correction

Native research follows [RECONSTRUCTION_METHOD.md](RECONSTRUCTION_METHOD.md).
The selector retains its historical PC alias `FUN_0050d8b0`; its prototype PS2
attribution remains **unknown**. Literal name candidates do not establish a match.

## Confirmed branch condition

The ordinary secondary previously refused fast downward motion when the actor's
height above the floor was below a configured threshold. The original PC branch
instead takes the unacquired transition when that height is **above** the threshold.
Both the downward velocity and height comparisons are strict. Height includes
the actor's scaled center offset; equality at either threshold continues normally.

An independent control executes the original branch instructions with supplied
finite actor/global values and Double53 arithmetic. All 1,049 cases agree with
this condition, including threshold boundaries and varying floor, center and
scale. The former Rust condition disagrees in 255 cases. The control excludes
preceding selector admission, the common transition tail and animation/effects.
Its private evidence is bound to the `debug-reference-1.1` PC binary identity
already recorded by the central method. No native listing or extracted table is
distributed here.

`mecc_secondary::ordinary` now uses the confirmed comparison. A synthetic regression
checks actual selector continuation below/at the height threshold and at/above
the velocity threshold, the explicit refusal above both strict thresholds,
unchanged caller payload and unchanged RNG. The refusal now names a high-descent
transition rather than assigning an unverified landing meaning.

## Limits

The taken native branch selects logical state 2 before its common tail. That
observation alone does not acquire the complete transition, animation, writes or
effect providers. Rust therefore continues to refuse that branch explicitly.
Impact reaction and contact landing-effect providers are separate prerequisites.
This correction does not establish full movement, normal campaign admission,
original GPU rendering or visual fidelity.

The earlier longer owned replay's step-152 failure remains a historical failed
control. It is distinct from the successful 128-step inspection/resource cycles
documented in [OWNED_MODEL_DRAW.md](OWNED_MODEL_DRAW.md). Receipt-bound reruns and
their next actual boundary are recorded in [RELEASES.md](RELEASES.md).
