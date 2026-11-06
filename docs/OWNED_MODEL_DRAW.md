# Owned Mecc render-model selection

Further native acquisition follows [RECONSTRUCTION_METHOD.md](RECONSTRUCTION_METHOD.md).
The selector and enclosing model-consumer attributions remain unknown. Recovered
names and CPU matches do not certify graphics submission or visual fidelity.

## Producer and renderer binding

`mecc_model_draw::prepare` requires the actual completed outer-camera publication
and the same source's completed ordinary Mecc frame. It derives current position,
scale, channels, animation stamp and the immutable specification from the Session.
World-frame identity is separate from the camera's invocation counter. The current
viewport origin and lens come from that camera publication, rather than a second
camera calculation in presentation code.

The native-checked `lod::Table` selects a slot from the actual specification table,
including missing entries and negative sentinels. The selected model is evaluated
through the existing shared position bank. No animation clock, reference pose or
completion marker is advanced. A missing selected model produces no body draw;
an invalid setting, foreign camera/source, incomplete or stale frame is refused.

The one menu-first executable's explicit `owned-inspection` backend consumes this
selection. Its body meshes use the same registered geometry identities and checked
unlit resources. Only the selected model receives current owned positions and is
made visible by the existing presentation policy. The fixed-model development
backend remains separately scoped. This does not replace that presentation
policy with the full native world renderer.

The earlier model-0 diagnostic pose is still retained by the presentation frame.
Its shared-bank preparation precedes this selected-model upload. Complete native
draw/cache ordering with other actors is therefore still unbound; migrate those
remaining diagnostic consumers before treating this as a whole-world renderer.

## Explicit inspection settings

`GIANTS_MISSION_MODEL_QUALITY` accepts indices 0 through 3; this inspection profile
defaults to 2. These are the acquired selector's quality-factor indices, not a claim
about the native game's default or a reconstructed settings menu.
`GIANTS_MISSION_MODEL_SLOT` defaults to -1 for automatic selection. Any negative
value uses that automatic path; 0 through 9 forces the corresponding table slot,
as in the native diagnostic override. An absent forced model stays absent.
The ordinary camera uses the existing normal 62-degree lens profile. Broader
camera modes, live settings/reset producers and full scheduling remain open.

## Evidence and remaining work

A controlled original-PC selector run loads the actual body specification. It
matches 1,280 selections over 32 source frames, four quality settings and automatic
or forced slots. The Rust route additionally matches 256 complete selected-model
position snapshots, for 309,472 compared words in total. Source poses and viewport
origins come from earlier native captures; quality, override and lens settings
are supplied. This does not execute original GPU callbacks or a complete world.

The installed-data Mecc lifecycle checks cover model/source/frame identity,
present and absent slots, unchanged channels/stamp, foreign sources and invalid
settings. Selecting body positions does not recertify body references after a
subjective post. The latter retains its separate shared-reference binding.

Native consumer inspection distinguishes model/position selection, camera-facing
matrices, normal preparation, node/part filters, material setup, immediate/deferred
parts and primitive submission. Reuse existing `object_culling`, `camera_facing`,
`render_parts`, `render_sort`, `body_pipeline` and `material_descriptor` services
while binding actual current providers. Model selection alone supplies none of
these missing admission decisions. Full world/list scheduling, subjective arms
and held rendering, owned firing, target contexts and positional audio remain
prerequisites in [BACKLOG.md](../BACKLOG.md).

## Recorded Bevy endpoints

Receipt-bound source `c798370` passes six actual menu-request/owned-inspection
runs of 128 supplied movement steps: automatic quality indices 2 and 0, forced
models 1, 4 and 7, and absent model 9. The selected model reaches the Bevy mesh
upload. At the recorded endpoint, model 0 has 736 positions/eight parts, model 4
has 351 positions/six parts, and model 7 has 57 positions/six parts. Absent model 9
uploads no body part. Automatic index 0 and forced model 7 produce identical
endpoint captures; the original PNG difference between indices 2 and 0 is confined
to the character region. Public WebP copies remain display evidence, not pixel
validation or an original-engine reference.

Staging counts the initial model's prepared resources separately from alternate
models. The upload check counts the selected model's actual parts, including zero
for an absent model. It no longer assumes the first model's count.

`owned-player-inspection-cycles` uses the same acquired 128-step profile for
repeated real cancellation/re-entry, with no readiness event or OS input injection.
It checks retirement of additional model assets. A separate longer general
movement replay is retained as a historical failure at step 152 under the previous
secondary guard. Independent native comparison then confirmed an inverted height
comparison. With that correction, the longer replay explicitly refuses the
then-unacquired high-descent transition at step 143. Its subsequent native-checked
implementation reaches the pending contact-landing effect branch at step 158; see
[MECC_DESCENT_GUARD.md](MECC_DESCENT_GUARD.md). Later reaction/effect
branches remain open; a successful short resource cycle does not establish longer
movement.
