# AudioSource post_create not called on ECS deserialize

## Problem

`Ecs::deserialize` is a serde `Deserialize` impl and receives no `GameCtxMut`. It
rebuilds component stores directly from RON without invoking `post_create` hooks.
This means that when `game.ron` is loaded at startup, `AudioSource::post_create`
never fires, so:

- `IncrementRefs` is never pushed for the loaded sounds.
- The audio cache is not pre-warmed — sounds load on the first playback call instead.
- Ref counts start at zero, so a later `DecrementRefs` (on entity removal) silently
  skips eviction rather than cleaning up correctly.

## Workaround (current)

`game/src/engine/game_instance.rs :: GameInstance::new` sweeps all `AudioSource`
components after `game.initialize()` and manually pushes `IncrementRefs`:

```rust
for source in AudioSource::store(&game.ecs).data.values() {
    push_audio_command(AudioCommand::IncrementRefs(source.sounds.clone()));
}
```

This is a one-time startup sweep and is not repeated on room transitions.

## Limitations

- Only covers the initial load. If rooms carry their own ECS snapshots in future,
  each room transition will need the same sweep.
- The workaround calls `clone()` on each `sounds` vec, which is acceptable at load
  time but would be wasteful if called repeatedly.
- `post_create` hooks for other components (e.g. Sprite asset ref-counts) have the
  same latent issue — they are protected today only because sprites are re-resolved
  via `AssetManager` during `game.initialize()`. AudioSource does not have that path.

## Proper fix (Save/Load sprint)

The Save/Load sprint introduces runtime save files and a generalised load pipeline.
At that point, add a **post-deserialize callback** to `Ecs` that fires `post_create`
for every component in every deserialized store, passing the `GameCtxMut` that the
callback needs. This replaces the per-system workaround with a single, correct hook
invocation. Reference: `engine_core/src/ecs/ecs.rs :: Ecs::deserialize`.
