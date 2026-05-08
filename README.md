# TRUEOS RADS

TRUEOS RADS is a small userspace RAD IDE for building TRUEOS/UI2 apps without copying a hello-world template by hand.

It runs as a cross-platform Rust binary with a local browser UI:

- Delphi-like tool palette for UI2 windows.
- Object inspector for captions, geometry, events, and window decorations.
- TRUEOS blueprint and package manifest generation.
- Tokio job manager for parallel check/build/pack/full-auto workflows.
- Server-sent job updates for live build and pack feedback.

## Run

```sh
cargo run
```

Then open `http://127.0.0.1:7377`.

## Shape

Generated projects live under `rads-workspace/` and include:

```text
app.blueprint.json
rads.project.json
ui/main.ui2.json
src/main.rs
package/manifest.trueos.json
```

The goal is to make TRUEOS app development feel like a rapid application environment: draw a UI2 window, set properties, run it, pack it, and iterate.
