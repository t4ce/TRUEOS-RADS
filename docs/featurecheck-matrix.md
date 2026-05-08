# RADS Featurecheck Matrix A-G

Scope: immediate pass-G featurecheck for documentation, examples, scripts, and
tests. This matrix records what can be verified from the current tree without
editing `src/**` or `static/**`.

Status legend:

- Ready: implemented enough to smoke-test through public Rust APIs or fixtures.
- Partial: implemented in the current app shape, but not complete end to end.
- Spec: documented contract for a feature that still needs source/UI work.

| ID | Feature area | Status | Current evidence | Smoke check |
| --- | --- | --- | --- | --- |
| A | Template creation and project persistence | Ready | `generator::create_project`, built-in project templates, `RadsProject::starter`, and `templates::*` write the starter tree. `GET /api/projects` indexes saved `rads-workspace/*/rads.project.json` files and `POST /api/project/load` makes one active again. See `docs/template-creation.md`. | Generate a temp project and require `rads.project.json`, `app.blueprint.json`, `ui/main.ui2`, `ui/index.html`, `ui/styles.css`, Rust stubs, package manifests, and README. Smoke the project index/load flow through the server. |
| B | UI2 project model and designer APIs | Ready | `Ui2Window`, `Ui2Control`, palette categories, snap settings, inspector sections, event bindings, and UI2 snippets are public library data APIs. | Add controls through `designer::add_control`, inspect sections, and render snippets through `templates::control_to_ui2_snippet`. |
| C | HTML/CSS UI2 shell description | Partial | `static/index.html`, `static/app.js`, and `static/styles.css` define the current browser shell, design stage, window frame, controls, object tree, inspector, job console, and palette tabs. See `docs/html-css-ui2-description.md`. | Documentation asserts the DOM/data contract and maps UI2 control kinds to CSS classes. |
| D | Preview, editor, and code tabs | Spec | The editor surface exists as the active design stage. Explicit Preview, Editor, and Code tabs are a documented UI contract for the next source pass. Code-tab file APIs are present in the server surface. | Documentation distinguishes current editor behavior from planned Preview and Code surfaces and names the project file API hooks. |
| E | Local featurecheck APIs | Partial | Public APIs cover model validation, generation, template catalog data, designer mutation, template rendering, project file listing/reading/writing, and `JobManager` job/event flow. HTTP routes are present in `server.rs`, but route handlers are private to the server module. | Exercise public Rust APIs and the `JobManager` pack event stream. HTTP endpoint smoke remains a source-level follow-up. |
| F | BP DIST stages | Ready | `JobKind::Pack` emits blueprint validation, UI2 layout collection, and package-plan lines, then runs `trueos-blueprint` when it can discover the Blueprints checkout. `check`, `build`, `pack`, and `auto` map to BP DIST stages. See `docs/bp-dist-stages.md`. | Spawn a public pack job and require the BP DIST stage log lines. |
| G | Docs, examples, and tests | Ready | `examples/generated-project/**` is the checked-in specimen. `tests/**` covers project shape, generator output, public designer APIs, docs contracts, and example cross-references. | Parse example JSON and compare app/package/manifest/layout references. |

## Immediate Check Commands

```sh
cargo fmt --check
cargo test
```

`scripts/smoke.sh` runs the same local pass and is intended as the quick command
for this docs/tests lane.

## Open Follow-Ups For Source/UI Lanes

- Add a public router/app builder so HTTP routes can be tested without binding
  the fixed development port.
- Wire explicit Preview, Editor, and Code tabs in the browser UI.
- Expand BP DIST artifact validation once the kernel-side package/install
  tooling has a stable inspection path.
