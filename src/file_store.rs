use anyhow::{Context, bail};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::fs;
use std::path::{Component, Path, PathBuf};

const IGNORED_DIRS: &[&str] = &[".git", "target", "node_modules"];
const GENERATED_FILES: &[&str] = &[
    "rads.project.json",
    "app.blueprint.json",
    "package/package.blueprint.json",
    "package/manifest.trueos.json",
    "ui/main.ui2",
    "ui/main.ui2.json",
    "ui/index.html",
    "ui/styles.css",
    "Cargo.toml",
    "src/main.rs",
    "src/ui.rs",
    "src/events.rs",
    "README.md",
];

#[derive(Debug, Clone, Serialize)]
pub struct ProjectFileListResponse {
    pub root: PathBuf,
    pub files: Vec<ProjectFileEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectFileEntry {
    pub path: PathBuf,
    pub name: String,
    pub kind: ProjectFileKind,
    pub size: u64,
    pub modified_at: Option<DateTime<Utc>>,
    pub generated: bool,
    pub writable: bool,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectFileKind {
    File,
    Directory,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectFileReadResponse {
    pub path: PathBuf,
    pub absolute_path: PathBuf,
    pub contents: String,
    pub size: u64,
    pub modified_at: Option<DateTime<Utc>>,
    pub generated: bool,
    pub writable: bool,
}

pub async fn list_project_files(root: PathBuf) -> anyhow::Result<ProjectFileListResponse> {
    tokio::task::spawn_blocking(move || {
        let mut files = Vec::new();
        collect_entries(&root, &root, &mut files)?;
        files.sort_by(|left, right| {
            left.path
                .components()
                .count()
                .cmp(&right.path.components().count())
                .then_with(|| left.path.cmp(&right.path))
        });
        Ok(ProjectFileListResponse { root, files })
    })
    .await
    .context("file listing task failed")?
}

pub async fn read_project_file(
    root: &Path,
    requested_path: impl AsRef<Path>,
) -> anyhow::Result<ProjectFileReadResponse> {
    let relative = project_relative_path(requested_path.as_ref())?;
    let absolute_path = root.join(&relative);
    let metadata = tokio::fs::metadata(&absolute_path)
        .await
        .with_context(|| format!("failed to stat {}", relative.display()))?;
    if !metadata.is_file() {
        bail!("{} is not a file", relative.display());
    }

    let bytes = tokio::fs::read(&absolute_path)
        .await
        .with_context(|| format!("failed to read {}", relative.display()))?;
    let contents = String::from_utf8(bytes)
        .with_context(|| format!("{} is not valid UTF-8", relative.display()))?;

    Ok(ProjectFileReadResponse {
        path: relative.clone(),
        absolute_path,
        contents,
        size: metadata.len(),
        modified_at: modified_at(&metadata),
        generated: is_generated_file(&relative),
        writable: !metadata.permissions().readonly(),
    })
}

pub async fn write_project_file(
    root: &Path,
    requested_path: impl AsRef<Path>,
    contents: String,
    create_dirs: bool,
) -> anyhow::Result<ProjectFileReadResponse> {
    let relative = project_relative_path(requested_path.as_ref())?;
    let absolute_path = root.join(&relative);
    if create_dirs {
        if let Some(parent) = absolute_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
    }
    tokio::fs::write(&absolute_path, contents)
        .await
        .with_context(|| format!("failed to write {}", relative.display()))?;
    read_project_file(root, relative).await
}

fn collect_entries(
    root: &Path,
    directory: &Path,
    files: &mut Vec<ProjectFileEntry>,
) -> anyhow::Result<()> {
    for entry in fs::read_dir(directory)
        .with_context(|| format!("failed to read {}", directory.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        if is_ignored_name(&file_name) {
            continue;
        }

        let metadata = entry
            .metadata()
            .with_context(|| format!("failed to stat {}", path.display()))?;
        let relative = path
            .strip_prefix(root)
            .with_context(|| format!("failed to relativize {}", path.display()))?
            .to_path_buf();
        let kind = if metadata.is_dir() {
            ProjectFileKind::Directory
        } else {
            ProjectFileKind::File
        };
        let writable = !metadata.permissions().readonly();
        files.push(ProjectFileEntry {
            path: relative.clone(),
            name: file_name,
            kind,
            size: metadata.len(),
            modified_at: modified_at(&metadata),
            generated: is_generated_file(&relative),
            writable,
        });

        if metadata.is_dir() {
            collect_entries(root, &path, files)?;
        }
    }
    Ok(())
}

fn project_relative_path(path: &Path) -> anyhow::Result<PathBuf> {
    if path.as_os_str().is_empty() {
        bail!("path is required");
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                bail!("project file paths must stay inside the project")
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        bail!("path must name a project file");
    }
    Ok(normalized)
}

fn is_ignored_name(name: &str) -> bool {
    IGNORED_DIRS.iter().any(|ignored| ignored == &name)
}

fn is_generated_file(path: &Path) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/");
    GENERATED_FILES
        .iter()
        .any(|generated| generated == &normalized)
}

fn modified_at(metadata: &fs::Metadata) -> Option<DateTime<Utc>> {
    metadata.modified().ok().map(DateTime::<Utc>::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn write_and_read_project_relative_file() {
        let dir = tempfile::tempdir().unwrap();

        let written = write_project_file(
            dir.path(),
            "src/events.rs",
            "fn main() {}\n".to_string(),
            true,
        )
        .await
        .unwrap();
        let read = read_project_file(dir.path(), "src/events.rs")
            .await
            .unwrap();

        assert_eq!(written.path, PathBuf::from("src/events.rs"));
        assert_eq!(read.contents, "fn main() {}\n");
        assert!(read.generated);
    }

    #[tokio::test]
    async fn rejects_paths_outside_project() {
        let dir = tempfile::tempdir().unwrap();
        let err = read_project_file(dir.path(), "../Cargo.toml")
            .await
            .unwrap_err();

        assert!(err.to_string().contains("inside the project"));
    }
}
