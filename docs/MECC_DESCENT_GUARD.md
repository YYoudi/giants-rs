# Owned Mecc descent threshold and transition

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
unchanged caller payload and unchanged RNG. The initial correction retained an
explicit high-descent refusal until the transition was acquired separately.

## Acquired ordinary transition

The ordinary pistol secondary now implements that high-descent branch through
the common animation-channel service. It selects logical state 2 and the dedicated
descent clip with its source-axis playback rate scaled by the actor. The common
tail clears the acquired selection fields, updates control flags and toggles the
payload alternation word only when an animation request actually occurs. An
already selected descent clip is retained, including when its channel has ended;
there is no unconditional restart.

The native branch precedes impact and contact-landing decisions and goes directly
to the common tail. Rust preserves that ordering, rather than asking those later
providers to execute on this path. Outside high descent, their existing explicit
refusals remain. Logical state 2 is accepted for subsequent ordinary frames.

Two independent original-selector controls cover 128 cases with real loaded
animation descriptors and blending disabled/enabled. They vary current clip,
prior logical state, control flags, scale, trigger state and channel-end state.
The actual original channel setter executes. All 41,920 compared header/payload
and request words match Rust. Deliberately supplied velocity-change and contact
inputs verify the high-descent path's precedence over the later branches.
This is a selector/channel control, not original physics, ANM or GPU execution.

## Limits

The acquired transition is limited to the living ordinary pistol profile.
Impact reaction and contact landing-effect providers remain separate prerequisites.
This correction does not establish full movement, normal campaign admission,
original GPU rendering or visual fidelity.

The earlier longer owned replay's step-152 failure remains a historical failed
control. It is distinct from the successful 128-step inspection/resource cycles
documented in [OWNED_MODEL_DRAW.md](OWNED_MODEL_DRAW.md). Receipt-bound reruns and
their next actual boundary are recorded in [RELEASES.md](RELEASES.md).
