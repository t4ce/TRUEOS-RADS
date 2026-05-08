use crate::model::{RadsProject, validate_project_name};
use crate::project_templates::{self, ProjectTemplate};
use crate::templates;
use anyhow::{Context, anyhow, bail};
use std::fs;
use std::path::{Path, PathBuf};

pub fn create_project(root: &Path, name: &str) -> anyhow::Result<RadsProject> {
    create_project_from_template(root, name, project_templates::DEFAULT_PROJECT_TEMPLATE_ID)
}

pub fn create_project_from_template(
    root: &Path,
    name: &str,
    template_id: &str,
) -> anyhow::Result<RadsProject> {
    let template = project_templates::find_project_template(template_id)
        .ok_or_else(|| unknown_template_error(template_id))?;
    create_project_with_template(root, name, template)
}

pub fn create_project_with_template(
    root: &Path,
    name: &str,
    template: &ProjectTemplate,
) -> anyhow::Result<RadsProject> {
    let validated = validate_project_name(name).context("invalid project name")?;
    let target = root.join(&validated.slug);
    if target.exists() {
        bail!("project target already exists: {}", target.display());
    }

    create_project_dirs(&target)?;

    let project = template.build_project(validated, &target);
    write_project_files_with_template(&project, template)?;
    Ok(project)
}

pub fn write_project_files(project: &RadsProject) -> anyhow::Result<()> {
    write_project_files_with_template(
        project,
        project_templates::default_project_template_for_kind(project.app_kind),
    )
}

pub fn write_project_files_with_template(
    project: &RadsProject,
    template: &ProjectTemplate,
) -> anyhow::Result<()> {
    write_json(project.root.join("rads.project.json"), project)?;
    write_json(project.root.join("app.blueprint.json"), &project.blueprint)?;
    write_json(
        project.root.join("package/package.blueprint.json"),
        &project.package,
    )?;
    if project.app_kind.has_ui2() && !project.windows.is_empty() {
        write_ui_files(project, template)?;
    }
    write_text(
        project.root.join("Cargo.toml"),
        templates::cargo_toml(project),
    )?;
    write_text(
        project.root.join("src/main.rs"),
        templates::main_rs_for_template(project, template),
    )?;
    if project.app_kind.has_ui2() && !project.windows.is_empty() {
        write_text(project.root.join("src/ui.rs"), templates::ui_rs(project))?;
    }
    write_text(
        project.root.join("src/events.rs"),
        templates::events_rs_for_template(project, template),
    )?;
    write_text(
        project.root.join("package/manifest.trueos.json"),
        templates::package_manifest(project),
    )?;
    write_text(
        project.root.join("README.md"),
        templates::readme_for_template(project, template),
    )?;
    Ok(())
}

fn write_ui_files(project: &RadsProject, template: &ProjectTemplate) -> anyhow::Result<()> {
    fs::create_dir_all(project.root.join("ui/windows"))
        .context("failed to create ui/windows directory")?;
    write_json(project.root.join("ui/main.ui2.json"), &project.windows[0])?;
    write_text(
        project.root.join("ui/main.ui2"),
        templates::ui2_layout(project),
    )?;
    write_text(
        project.root.join("ui/index.html"),
        templates::html_for_window(project, template, &project.windows[0]),
    )?;
    write_text(
        project.root.join("ui/styles.css"),
        templates::css_for_window(project, template, &project.windows[0]),
    )?;

    for (index, window) in project.windows.iter().enumerate() {
        let stem = window_file_stem(window, index);
        write_json(
            project.root.join(format!("ui/windows/{stem}.ui2.json")),
            window,
        )?;
        write_text(
            project.root.join(format!("ui/windows/{stem}.html")),
            templates::html_for_window(project, template, window),
        )?;
        write_text(
            project.root.join(format!("ui/windows/{stem}.css")),
            templates::css_for_window(project, template, window),
        )?;
    }
    Ok(())
}

fn create_project_dirs(target: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(target.join("src")).context("failed to create src directory")?;
    fs::create_dir_all(target.join("ui")).context("failed to create ui directory")?;
    fs::create_dir_all(target.join("assets")).context("failed to create assets directory")?;
    fs::create_dir_all(target.join("package")).context("failed to create package directory")?;
    Ok(())
}

fn window_file_stem(window: &crate::model::Ui2Window, index: usize) -> String {
    if index == 0 {
        return "main".to_string();
    }
    let stem = crate::model::slugify(&window.name);
    if stem.is_empty() {
        format!("window-{}", index + 1)
    } else {
        stem
    }
}

fn unknown_template_error(template_id: &str) -> anyhow::Error {
    let available = project_templates::available_project_templates()
        .iter()
        .map(|template| template.id)
        .collect::<Vec<_>>()
        .join(", ");
    anyhow!("unknown project template '{template_id}'. available templates: {available}")
}

fn write_json(path: PathBuf, value: &impl serde::Serialize) -> anyhow::Result<()> {
    let body = serde_json::to_string_pretty(value)?;
    write_text(path, body)
}

fn write_text(path: PathBuf, body: String) -> anyhow::Result<()> {
    fs::write(&path, body).with_context(|| format!("failed to write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_starter_project() {
        let dir = tempfile::tempdir().unwrap();
        let project = create_project(dir.path(), "Hello UI2").unwrap();
        assert!(project.root.join("app.blueprint.json").exists());
        assert!(project.root.join("ui/main.ui2.json").exists());
        assert_eq!(project.windows.len(), 1);
    }
}
