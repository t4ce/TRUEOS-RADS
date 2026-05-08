# BP DIST Stages

BP DIST is the blueprint distribution path from a RADS design document to a
TRUEOS package artifact. The current implementation has enough structure to
validate the plan and simulate packaging, while the final artifact writer is
still future work.

## Stage Map

| Stage | Name | Input | Output | Current coverage |
| --- | --- | --- | --- | --- |
| BP DIST 0 | Model source | `rads.project.json` and active `RadsProject` memory state. | Canonical app, package, window, control, and event model. | Generated and parsed in tests. |
| BP DIST 1 | Blueprint validate | `app.blueprint.json` and `package/package.blueprint.json`. | Valid app/package identity, schema, entrypoint, layout path, capabilities, and artifacts. | Simulated by pack log `validating blueprint`; parsed in tests. |
| BP DIST 2 | Layout collect | `ui/main.ui2` and `ui/main.ui2.json`. | UI2 layout bundle with decorations, controls, properties, and events. | Simulated by pack log `collecting UI2 layouts and assets`; parsed in tests. |
| BP DIST 2A | HTML/CSS collect | `ui/index.html`, `ui/styles.css`, and window `ui_description` fields. | Preview/code assets bundled with the layout. | Documented and fixture-tested; final package validation is future work. |
| BP DIST 3 | Check | Generated Rust package. | `cargo check` result. | `JobKind::Check` and `JobKind::FullAuto`. |
| BP DIST 4 | Build | Generated Rust package. | `cargo build` result. | `JobKind::Build`. |
| BP DIST 5 | Package plan | Blueprints, layout bundle, build metadata. | Package manifest and artifact plan. | Simulated by pack log `writing package plan`. |
| BP DIST 6 | Dist artifact | Package plan and build output. | Future `.tapp` or equivalent TRUEOS package output. | Spec only. |
| BP DIST 7 | Verify/publish | Dist artifact. | Installable package verification and optional publishing handoff. | Spec only. |

## Job Mapping

| Job kind | Stages |
| --- | --- |
| `check` | BP DIST 3 |
| `build` | BP DIST 4 |
| `pack` | BP DIST 1, BP DIST 2, BP DIST 2A, BP DIST 5 |
| `dist` | BP DIST 1 through BP DIST 6 once artifact writing lands |
| `auto` | BP DIST 3, then BP DIST 1, BP DIST 2, BP DIST 2A, BP DIST 5 if check passes |

The current pack job is intentionally a package-plan smoke. It emits stage log
lines through `JobManager` so the UI and tests can observe the path without
requiring a final TRUEOS package writer.

## Completion Criteria

The stage should advance from simulated to ready when:

- BP DIST 1 rejects schema, app/package ID, entrypoint, and layout mismatches.
- BP DIST 2 verifies every referenced UI2 layout exists and parses.
- BP DIST 5 writes a deterministic package plan or manifest.
- BP DIST 6 writes the final package artifact declared by
  `package/package.blueprint.json`.
- BP DIST 7 verifies the artifact can be inspected or installed by TRUEOS
  tooling.
