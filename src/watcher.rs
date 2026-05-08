use crate::jobs::{JobKind, JobManager};
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

const WATCH_DEBOUNCE: Duration = Duration::from_millis(800);

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

fn should_run_for_event(event: &Event) -> bool {
    if !matches!(
        event.kind,
        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_)
    ) {
        return false;
    }

    event.paths.iter().any(|path| !is_ignored_path(path))
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
