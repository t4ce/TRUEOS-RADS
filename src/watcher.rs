use crate::jobs::{JobKind, JobManager};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc};

const WATCH_DEBOUNCE: Duration = Duration::from_millis(800);

#[derive(Debug, Clone, Serialize)]
pub struct ProjectFileEvent {
    pub project: PathBuf,
    pub paths: Vec<PathBuf>,
    pub kind: ProjectFileEventKind,
    pub at_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProjectFileEventKind {
    Create,
    Modify,
    Remove,
    Other,
}

pub async fn watch_project(
    project: PathBuf,
    jobs: JobManager,
    full_auto: Arc<AtomicBool>,
) -> notify::Result<RecommendedWatcher> {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut watcher = notify::recommended_watcher(move |result| {
        let _ = tx.send(result);
    })?;
    watcher.watch(&project, RecursiveMode::Recursive)?;

    tokio::spawn(async move {
        let mut last_run = Instant::now() - WATCH_DEBOUNCE;
        while let Some(result) = rx.recv().await {
            let Ok(event) = result else {
                continue;
            };
            if !should_run_for_event(&event) || last_run.elapsed() < WATCH_DEBOUNCE {
                continue;
            }
            last_run = Instant::now();

            let kind = if full_auto.load(Ordering::Relaxed) {
                JobKind::FullAuto {
                    project: project.clone(),
                }
            } else {
                JobKind::Check {
                    project: project.clone(),
                }
            };
            let _ = jobs.spawn(kind).await;
        }
    });

    Ok(watcher)
}

pub async fn watch_project_files(
    project: PathBuf,
    events: broadcast::Sender<ProjectFileEvent>,
) -> notify::Result<RecommendedWatcher> {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut watcher = notify::recommended_watcher(move |result| {
        let _ = tx.send(result);
    })?;
    watcher.watch(&project, RecursiveMode::Recursive)?;

    tokio::spawn(async move {
        while let Some(result) = rx.recv().await {
            let Ok(event) = result else {
                continue;
            };
            if !should_run_for_event(&event) {
                continue;
            }

            let paths = event
                .paths
                .iter()
                .filter(|path| !is_ignored_path(path))
                .map(|path| path.strip_prefix(&project).unwrap_or(path).to_path_buf())
                .collect::<Vec<_>>();
            if paths.is_empty() {
                continue;
            }

            let _ = events.send(ProjectFileEvent {
                project: project.clone(),
                paths,
                kind: project_file_event_kind(&event),
                at_unix_ms: now_unix_ms(),
            });
        }
    });

    Ok(watcher)
}

fn should_run_for_event(event: &Event) -> bool {
    if !matches!(
        event.kind,
        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
    ) {
        return false;
    }

    event.paths.iter().any(|path| !is_ignored_path(path))
}

fn project_file_event_kind(event: &Event) -> ProjectFileEventKind {
    match event.kind {
        EventKind::Create(_) => ProjectFileEventKind::Create,
        EventKind::Modify(_) => ProjectFileEventKind::Modify,
        EventKind::Remove(_) => ProjectFileEventKind::Remove,
        _ => ProjectFileEventKind::Other,
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        .unwrap_or_default()
}

fn is_ignored_path(path: &std::path::Path) -> bool {
    path.components().any(|component| {
        let name = component.as_os_str().to_string_lossy();
        matches!(
            name.as_ref(),
            ".git" | "target" | "dist" | "package" | "node_modules" | ".DS_Store"
        )
    })
}
