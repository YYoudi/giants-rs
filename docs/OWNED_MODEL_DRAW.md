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
