# BP DIST Stages

BP DIST is the blueprint distribution path from a RADS design document to a
TRUEOS package artifact. RADS now writes the package plan and, when a sibling
`TRUEOS Blueprints` checkout is available, streams the real `trueos-blueprint`
packer into the job log.

## Stage Map

| Stage | Name | Input | Output | Current coverage |
| --- | --- | --- | --- | --- |
| BP DIST 0 | Model source | `rads.project.json` and active `RadsProject` memory state. | Canonical app, package, window, control, and event model. | Generated and parsed in tests. |
| BP DIST 1 | Blueprint validate | `app.blueprint.json` and `package/package.blueprint.json`. | Valid app/package identity, schema, entrypoint, layout path, capabilities, and artifacts. | Simulated by pack log `validating blueprint`; parsed in tests. |
| BP DIST 2 | Layout collect | `ui/main.ui2` and `ui/main.ui2.json`. | UI2 layout bundle with decorations, controls, properties, and events. | Simulated by pack log `collecting UI2 layouts and assets`; parsed in tests. |
| BP DIST 2A | HTML/CSS collect | `ui/index.html`, `ui/styles.css`, and window `ui_description` fields. | Preview/code assets bundled with the layout. | Documented and fixture-tested; final package validation is future work. |
| BP DIST 3 | Check | Generated Rust package. | `cargo check` result. | `JobKind::Check` and `JobKind::FullAuto`. |
| BP DIST 4 | Build | Generated Rust package. | `cargo build` result. | `JobKind::Build`. |
| BP DIST 5 | Package plan | Blueprints, layout bundle, build metadata. | Package manifest and artifact plan. | Pack log `writing package plan`; real BP packer runs when discovered. |
| BP DIST 6 | Dist artifact | Package plan and build output. | `.bp` artifact from `trueos-blueprint`, or placeholder if the packer is unavailable. | Real packer path plus fallback. |
| BP DIST 7 | Verify/publish | Dist artifact. | Installable package verification and optional publishing handoff. | Spec only. |

## Job Mapping

| Job kind | Stages |
| --- | --- |
| `check` | BP DIST 3 |
| `build` | BP DIST 4 |
| `pack` | BP DIST 1, BP DIST 2, BP DIST 2A, BP DIST 5 |
| `dist` | BP DIST 1 through BP DIST 6 |
| `auto` | BP DIST 3, then BP DIST 1, BP DIST 2, BP DIST 2A, BP DIST 5 if check passes |

The pack job always emits package-plan stage lines through `JobManager`. If
`trueos-blueprint` is present, RADS also streams stdout/stderr from the packer
and verifies the resulting `dist/<slug>.bp`; otherwise it leaves the package
plan ready for a later packer run.

## Completion Criteria

The stage should advance from simulated to ready when:

- BP DIST 1 rejects schema, app/package ID, entrypoint, and layout mismatches.
- BP DIST 2 verifies every referenced UI2 layout exists and parses.
- BP DIST 5 writes a deterministic package plan or manifest.
- BP DIST 6 writes or verifies the `.bp` artifact declared by the active
  Blueprints packer.
- BP DIST 7 verifies the artifact can be inspected or installed by TRUEOS
  tooling.
