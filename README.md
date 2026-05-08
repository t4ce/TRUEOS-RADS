# TRUEOS RADS

TRUEOS RADS is a small userspace RAD IDE for building TRUEOS/UI2 apps without copying a hello-world template by hand.

It runs as a cross-platform Rust binary with a local browser UI:

- Lazarus-like tool palette for UI2 windows.
- Object inspector for captions, geometry, events, and window decorations.
- TRUEOS blueprint and package manifest generation.
- Tokio job manager for parallel check/build/pack/full-auto workflows.
- Server-sent job updates for live build and pack feedback.

## Run

```sh
cargo run
```

Then open `http://127.0.0.1:7377`.

## Localcoder

RADS loads `.env.local` on startup. For OpenAI-backed localcoder testing, put the API key there:

```sh
OPENAI_API_KEY=sk-proj-your-test-key
```

The localcoder model settings live in `.localcoder-home/.localcoder/settings.json`. Both `.env.local` and `.localcoder-home/` are ignored by git; `.env.local.example` shows the expected shape.

When a RADS project is active, the Localcoder chat runs with that project root
as its working directory and receives a small TRUEOS RADS context prelude. The
CLI tools therefore see the active project through normal cwd-based operations:
`Bash` runs `bash -lc`, file/search tools resolve against the project root, and
RADS also exposes `TRUEOS_RADS_PROJECT_*` environment variables to the process.
The Localcoder Tools tab is backed by `/api/localcoder/status`; Git is currently
reported as available through the Bash tool rather than as a dedicated model
function.

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
