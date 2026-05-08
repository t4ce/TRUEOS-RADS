use crate::generator;
use crate::model::RadsProject;
use anyhow::{Context, bail};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{Mutex, broadcast};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub kind: JobKind,
    pub status: JobStatus,
    pub current_stage: Option<JobStage>,
    pub stages: Vec<JobStageState>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub lines: Vec<String>,
    pub events: Vec<JobLogLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JobKind {
    Generate { project: PathBuf },
    Check { project: PathBuf },
    Build { project: PathBuf },
    Pack { project: PathBuf },
    Dist { project: PathBuf },
    FullAuto { project: PathBuf },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JobStatus {
    Queued,
    Running,
    Passed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JobStage {
    Generate,
    Check,
    Build,
    Pack,
    Dist,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStageState {
    pub stage: JobStage,
    pub status: JobStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JobEventType {
    Queued,
    Started,
    Output,
    Finished,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JobStream {
    System,
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobLogLine {
    pub event: JobEventType,
    pub stream: JobStream,
    pub status: JobStatus,
    pub stage: Option<JobStage>,
    pub stage_status: Option<JobStatus>,
    pub line: String,
    pub at: DateTime<Utc>,
    pub sequence: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobEvent {
    pub job_id: Uuid,
    pub kind: JobKind,
    pub event: JobEventType,
    pub stream: JobStream,
    pub status: JobStatus,
    pub stage: Option<JobStage>,
    pub stage_status: Option<JobStatus>,
    pub line: String,
    pub at: DateTime<Utc>,
    pub sequence: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobActivity {
    pub job_id: Uuid,
    pub kind: JobKind,
    pub status: JobStatus,
    pub current_stage: Option<JobStage>,
}

#[derive(Clone)]
pub struct JobManager {
    inner: Arc<Mutex<JobState>>,
    events: broadcast::Sender<JobEvent>,
}

#[derive(Debug)]
struct JobState {
    jobs: HashMap<Uuid, Job>,
    recent: VecDeque<Uuid>,
    next_sequence: u64,
}

#[derive(Debug, Clone)]
struct BlueprintTool {
    program: PathBuf,
    args: Vec<OsString>,
    cwd: PathBuf,
    envs: Vec<(String, OsString)>,
}

const TRUEOS_BLUEPRINT_BIN_ENV: &str = "TRUEOS_BLUEPRINT_BIN";
const TRUEOS_BLUEPRINT_ROOT_ENV: &str = "TRUEOS_BLUEPRINT_ROOT";
const TRUEOS_BLUEPRINT_KERNEL_ROOT_ENV: &str = "TRUEOS_BLUEPRINT_KERNEL_ROOT";
const TRUEOS_BLUEPRINT_CARGO_CACHE_DIR_ENV: &str = "TRUEOS_BLUEPRINT_CARGO_CACHE_DIR";
const TRUEOS_BLUEPRINT_TARGET_SPEC_ENV: &str = "TRUEOS_BLUEPRINT_TARGET_SPEC";

impl JobManager {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(512);
        Self {
            inner: Arc::new(Mutex::new(JobState {
                jobs: HashMap::new(),
                recent: VecDeque::new(),
                next_sequence: 1,
            })),
            events,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<JobEvent> {
        self.events.subscribe()
    }

    pub async fn list(&self) -> Vec<Job> {
        let state = self.inner.lock().await;
        state
            .recent
            .iter()
            .filter_map(|id| state.jobs.get(id).cloned())
            .collect()
    }

    pub async fn current_activity(&self) -> Option<JobActivity> {
        let state = self.inner.lock().await;
        let running = state
            .recent
            .iter()
            .filter_map(|id| state.jobs.get(id))
            .find(|job| matches!(job.status, JobStatus::Queued | JobStatus::Running));
        let latest = running.or_else(|| {
            state
                .recent
                .iter()
                .filter_map(|id| state.jobs.get(id))
                .next()
        })?;
        Some(JobActivity {
            job_id: latest.id,
            kind: latest.kind.clone(),
            status: latest.status,
            current_stage: latest.current_stage,
        })
    }

    pub async fn get(&self, id: Uuid) -> Option<Job> {
        let state = self.inner.lock().await;
        state.jobs.get(&id).cloned()
    }

    pub async fn spawn(&self, kind: JobKind) -> Uuid {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let job = Job {
            id,
            kind: kind.clone(),
            status: JobStatus::Queued,
            current_stage: None,
            stages: pipeline_for_kind(&kind)
                .into_iter()
                .map(JobStageState::queued)
                .collect(),
            created_at: now,
            updated_at: now,
            lines: Vec::new(),
            events: Vec::new(),
        };
        {
            let mut state = self.inner.lock().await;
            state.recent.push_front(id);
            while state.recent.len() > 100 {
                if let Some(old) = state.recent.pop_back() {
                    state.jobs.remove(&old);
                }
            }
            state.jobs.insert(id, job);
        }
        self.note(
            id,
            JobStatus::Queued,
            JobEventType::Queued,
            JobStream::System,
            None,
            None,
            "queued",
        )
        .await;

        let manager = self.clone();
        tokio::spawn(async move {
            manager.run(id, kind).await;
        });
        id
    }

    async fn run(&self, id: Uuid, kind: JobKind) {
        self.note(
            id,
            JobStatus::Running,
            JobEventType::Started,
            JobStream::System,
            None,
            None,
            "started",
        )
        .await;
        let result = self.run_pipeline(id, &kind).await;

        match result {
            Ok(()) => {
                self.note(
                    id,
                    JobStatus::Passed,
                    JobEventType::Finished,
                    JobStream::System,
                    None,
                    None,
                    "finished",
                )
                .await
            }
            Err(err) => {
                self.note(
                    id,
                    JobStatus::Failed,
                    JobEventType::Finished,
                    JobStream::System,
                    None,
                    None,
                    &format!("failed: {err}"),
                )
                .await
            }
        }
    }

    async fn run_pipeline(&self, id: Uuid, kind: &JobKind) -> anyhow::Result<()> {
        let project = project_for_kind(kind);
        for stage in pipeline_for_kind(kind) {
            self.note(
                id,
                JobStatus::Running,
                JobEventType::Output,
                JobStream::System,
                Some(stage),
                Some(JobStatus::Running),
                &format!("stage {} started", stage.as_str()),
            )
            .await;

            match self.run_stage(id, stage, project).await {
                Ok(()) => {
                    self.note(
                        id,
                        JobStatus::Running,
                        JobEventType::Output,
                        JobStream::System,
                        Some(stage),
                        Some(JobStatus::Passed),
                        &format!("stage {} passed", stage.as_str()),
                    )
                    .await;
                }
                Err(err) => {
                    let label = failure_label(stage, &err.to_string());
                    self.note(
                        id,
                        JobStatus::Failed,
                        JobEventType::Output,
                        JobStream::System,
                        Some(stage),
                        Some(JobStatus::Failed),
                        &format!("stage {} failed: {label}: {err}", stage.as_str()),
                    )
                    .await;
                    bail!("{} stage failed: {err}", stage.as_str());
                }
            }
        }
        Ok(())
    }

    async fn run_stage(&self, id: Uuid, stage: JobStage, project: &Path) -> anyhow::Result<()> {
        match stage {
            JobStage::Generate => self.regenerate_project(id, project).await,
            JobStage::Check => {
                self.run_command(id, stage, project, "cargo", &["check"])
                    .await
            }
            JobStage::Build => {
                self.run_command(id, stage, project, "cargo", &["build"])
                    .await
            }
            JobStage::Pack => self.run_pack(id, project).await,
            JobStage::Dist => self.run_dist(id, project).await,
        }
    }

    async fn run_command(
        &self,
        id: Uuid,
        stage: JobStage,
        project: &Path,
        program: &str,
        args: &[&str],
    ) -> anyhow::Result<()> {
        self.note(
            id,
            JobStatus::Running,
            JobEventType::Output,
            JobStream::System,
            Some(stage),
            Some(JobStatus::Running),
            &format!(
                "running `{program} {}` in {}",
                args.join(" "),
                project.display()
            ),
        )
        .await;
        let output = Command::new(program)
            .args(args)
            .current_dir(project)
            .output()
            .await?;
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            self.note(
                id,
                JobStatus::Running,
                JobEventType::Output,
                JobStream::Stdout,
                Some(stage),
                Some(JobStatus::Running),
                line,
            )
            .await;
        }
        for line in String::from_utf8_lossy(&output.stderr).lines() {
            self.note(
                id,
                JobStatus::Running,
                JobEventType::Output,
                JobStream::Stderr,
                Some(stage),
                Some(JobStatus::Running),
                line,
            )
            .await;
        }
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "command exited with {}; stderr tail: {}",
                output.status,
                diagnostic_tail(&stderr)
            )
        }
    }

    async fn regenerate_project(&self, id: Uuid, project: &Path) -> anyhow::Result<()> {
        self.note(
            id,
            JobStatus::Running,
            JobEventType::Output,
            JobStream::System,
            Some(JobStage::Generate),
            Some(JobStatus::Running),
            "loading rads.project.json",
        )
        .await;
        let mut project_model = read_project_model(project)?;
        project_model.root = project.to_path_buf();
        generator::write_project_files(&project_model)
            .with_context(|| format!("failed to regenerate {}", project.display()))?;
        self.note(
            id,
            JobStatus::Running,
            JobEventType::Output,
            JobStream::System,
            Some(JobStage::Generate),
            Some(JobStatus::Running),
            "generated RADS project files",
        )
        .await;
        Ok(())
    }

    async fn run_pack(&self, id: Uuid, project: &Path) -> anyhow::Result<()> {
        self.write_package_plan(id, project).await?;

        let Some(tool) = discover_blueprint_tool(project) else {
            self.note(
                id,
                JobStatus::Running,
                JobEventType::Output,
                JobStream::System,
                Some(JobStage::Pack),
                Some(JobStatus::Running),
                "trueos-blueprint not found; package plan is ready",
            )
            .await;
            return Ok(());
        };

        let cache = tool
            .envs
            .iter()
            .find(|(key, _)| key == TRUEOS_BLUEPRINT_CARGO_CACHE_DIR_ENV)
            .map(|(_, value)| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| "default".to_string());
        self.note(
            id,
            JobStatus::Running,
            JobEventType::Output,
            JobStream::System,
            Some(JobStage::Pack),
            Some(JobStatus::Running),
            &format!("running trueos-blueprint with cargo cache {cache}"),
        )
        .await;
        self.run_process_streamed(
            id,
            JobStage::Pack,
            &tool.cwd,
            &tool.program,
            &tool.args,
            &tool.envs,
        )
        .await
    }

    async fn write_package_plan(&self, id: Uuid, project: &Path) -> anyhow::Result<()> {
        require_project_file(project, "app.blueprint.json")?;
        require_project_file(project, "package/package.blueprint.json")?;
        let project_model = read_project_model(project)?;
        if project_model.app_kind.has_ui2() {
            require_project_file(project, "ui/main.ui2")?;
        }
        self.note(
            id,
            JobStatus::Running,
            JobEventType::Output,
            JobStream::System,
            Some(JobStage::Pack),
            Some(JobStatus::Running),
            "validating blueprint",
        )
        .await;
        tokio::time::sleep(std::time::Duration::from_millis(120)).await;
        self.note(
            id,
            JobStatus::Running,
            JobEventType::Output,
            JobStream::System,
            Some(JobStage::Pack),
            Some(JobStatus::Running),
            if project_model.app_kind.has_ui2() {
                "collecting UI2 layouts and assets"
            } else {
                "collecting app artifacts and assets"
            },
        )
        .await;
        tokio::time::sleep(std::time::Duration::from_millis(120)).await;
        let plan_path = project.join("target/rads/package-plan.json");
        if let Some(parent) = plan_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let plan = serde_json::json!({
            "schema": "trueos.rads.package-plan/v1",
            "app_id": project_model.blueprint.app_id,
            "package_id": project_model.package.package_id,
            "artifacts": project_model.package.artifacts,
            "layout": project_model.blueprint.ui_layout,
        });
        fs::write(&plan_path, serde_json::to_vec_pretty(&plan)?)
            .with_context(|| format!("failed to write {}", plan_path.display()))?;
        self.note(
            id,
            JobStatus::Running,
            JobEventType::Output,
            JobStream::System,
            Some(JobStage::Pack),
            Some(JobStatus::Running),
            &format!("writing package plan for {}", project.display()),
        )
        .await;
        Ok(())
    }

    async fn run_dist(&self, id: Uuid, project: &Path) -> anyhow::Result<()> {
        let project_model = read_project_model(project)?;
        let dist_dir = project.join("dist");
        let dist_path = dist_dir.join(format!("{}.bp", project_model.slug));
        if dist_path.is_file() {
            self.note(
                id,
                JobStatus::Running,
                JobEventType::Output,
                JobStream::System,
                Some(JobStage::Dist),
                Some(JobStatus::Running),
                &format!("verified dist artifact {}", dist_path.display()),
            )
            .await;
            return Ok(());
        }

        fs::create_dir_all(&dist_dir)
            .with_context(|| format!("failed to create {}", dist_dir.display()))?;
        let body = serde_json::to_vec_pretty(&serde_json::json!({
            "schema": "trueos.rads.dist-placeholder/v1",
            "app_id": project_model.blueprint.app_id,
            "package_id": project_model.package.package_id,
            "source": "trueos-rads",
            "package_plan": "target/rads/package-plan.json",
        }))?;
        fs::write(&dist_path, body)
            .with_context(|| format!("failed to write {}", dist_path.display()))?;
        self.note(
            id,
            JobStatus::Running,
            JobEventType::Output,
            JobStream::System,
            Some(JobStage::Dist),
            Some(JobStatus::Running),
            &format!("wrote dist artifact {}", dist_path.display()),
        )
        .await;
        Ok(())
    }

    async fn run_process_streamed(
        &self,
        id: Uuid,
        stage: JobStage,
        cwd: &Path,
        program: &Path,
        args: &[OsString],
        envs: &[(String, OsString)],
    ) -> anyhow::Result<()> {
        let rendered_args = args
            .iter()
            .map(|arg| arg.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
        self.note(
            id,
            JobStatus::Running,
            JobEventType::Output,
            JobStream::System,
            Some(stage),
            Some(JobStatus::Running),
            &format!(
                "running `{}` {} in {}",
                program.display(),
                rendered_args,
                cwd.display()
            ),
        )
        .await;

        let mut command = Command::new(program);
        command
            .args(args)
            .current_dir(cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (key, value) in envs {
            command.env(key, value);
        }

        let mut child = command.spawn().with_context(|| {
            format!("failed to start {} in {}", program.display(), cwd.display())
        })?;
        let stdout_task = child.stdout.take().map(|stdout| {
            let manager = self.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stdout).lines();
                while let Some(line) = lines.next_line().await? {
                    manager
                        .note(
                            id,
                            JobStatus::Running,
                            JobEventType::Output,
                            JobStream::Stdout,
                            Some(stage),
                            Some(JobStatus::Running),
                            &line,
                        )
                        .await;
                }
                Ok::<(), std::io::Error>(())
            })
        });
        let stderr_task = child.stderr.take().map(|stderr| {
            let manager = self.clone();
            tokio::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Some(line) = lines.next_line().await? {
                    manager
                        .note(
                            id,
                            JobStatus::Running,
                            JobEventType::Output,
                            JobStream::Stderr,
                            Some(stage),
                            Some(JobStatus::Running),
                            &line,
                        )
                        .await;
                }
                Ok::<(), std::io::Error>(())
            })
        });

        let status = child.wait().await?;
        if let Some(task) = stdout_task {
            task.await.context("stdout reader task failed")??;
        }
        if let Some(task) = stderr_task {
            task.await.context("stderr reader task failed")??;
        }

        if status.success() {
            Ok(())
        } else {
            bail!("command exited with {status}");
        }
    }

    async fn note(
        &self,
        id: Uuid,
        status: JobStatus,
        event_type: JobEventType,
        stream: JobStream,
        stage: Option<JobStage>,
        stage_status: Option<JobStatus>,
        line: &str,
    ) {
        let at = Utc::now();
        let event = {
            let mut state = self.inner.lock().await;
            let sequence = state.next_sequence;
            state.next_sequence += 1;
            if let Some(job) = state.jobs.get_mut(&id) {
                let log = JobLogLine {
                    event: event_type.clone(),
                    stream: stream.clone(),
                    status,
                    stage,
                    stage_status,
                    line: line.to_string(),
                    at,
                    sequence,
                };
                job.status = status;
                if let Some(stage) = stage {
                    job.current_stage = Some(stage);
                    if let Some(indicator) = job
                        .stages
                        .iter_mut()
                        .find(|indicator| indicator.stage == stage)
                    {
                        if matches!(stage_status, Some(JobStatus::Running))
                            && indicator.started_at.is_none()
                        {
                            indicator.started_at = Some(at);
                        }
                        if let Some(stage_status) = stage_status {
                            indicator.status = stage_status;
                        }
                        indicator.updated_at = Some(at);
                        indicator.message = Some(line.to_string());
                    }
                }
                job.updated_at = at;
                job.lines.push(line.to_string());
                job.events.push(log);
                Some(JobEvent {
                    job_id: id,
                    kind: job.kind.clone(),
                    event: event_type,
                    stream,
                    status,
                    stage,
                    stage_status,
                    line: line.to_string(),
                    at,
                    sequence,
                })
            } else {
                None
            }
        };
        if let Some(event) = event {
            let _ = self.events.send(event);
        }
    }
}

impl JobStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::Generate => "generate",
            Self::Check => "check",
            Self::Build => "build",
            Self::Pack => "pack",
            Self::Dist => "dist",
        }
    }
}

impl JobStageState {
    fn queued(stage: JobStage) -> Self {
        Self {
            stage,
            status: JobStatus::Queued,
            started_at: None,
            updated_at: None,
            message: None,
        }
    }
}

fn pipeline_for_kind(kind: &JobKind) -> Vec<JobStage> {
    match kind {
        JobKind::Generate { .. } => vec![JobStage::Generate],
        JobKind::Check { .. } => vec![JobStage::Generate, JobStage::Check],
        JobKind::Build { .. } => vec![JobStage::Generate, JobStage::Check, JobStage::Build],
        JobKind::Pack { .. } => vec![JobStage::Generate, JobStage::Pack],
        JobKind::FullAuto { .. } => vec![JobStage::Generate, JobStage::Check, JobStage::Pack],
        JobKind::Dist { .. } => vec![
            JobStage::Generate,
            JobStage::Check,
            JobStage::Build,
            JobStage::Pack,
            JobStage::Dist,
        ],
    }
}

fn project_for_kind(kind: &JobKind) -> &Path {
    match kind {
        JobKind::Generate { project }
        | JobKind::Check { project }
        | JobKind::Build { project }
        | JobKind::Pack { project }
        | JobKind::Dist { project }
        | JobKind::FullAuto { project } => project,
    }
}

fn read_project_model(project: &Path) -> anyhow::Result<RadsProject> {
    let project_file = project.join("rads.project.json");
    let body = fs::read_to_string(&project_file)
        .with_context(|| format!("failed to read {}", project_file.display()))?;
    let mut project_model: RadsProject = serde_json::from_str(&body)
        .with_context(|| format!("failed to parse {}", project_file.display()))?;
    project_model.root = project.to_path_buf();
    Ok(project_model)
}

fn require_project_file(project: &Path, relative: &str) -> anyhow::Result<()> {
    let path = project.join(relative);
    if path.is_file() {
        Ok(())
    } else {
        bail!("missing required project file {}", relative)
    }
}

fn failure_label(stage: JobStage, diagnostic: &str) -> &'static str {
    match stage {
        JobStage::Generate => "generation failure",
        JobStage::Check => "syntax/check failure",
        JobStage::Build if looks_like_link_failure(diagnostic) => "link failure",
        JobStage::Build => "build failure",
        JobStage::Pack => "package failure",
        JobStage::Dist => "dist failure",
    }
}

fn looks_like_link_failure(diagnostic: &str) -> bool {
    let lower = diagnostic.to_ascii_lowercase();
    lower.contains("linking with")
        || lower.contains("linker")
        || lower.contains("undefined reference")
        || lower.contains("ld returned")
}

fn diagnostic_tail(diagnostic: &str) -> String {
    let tail = diagnostic
        .lines()
        .rev()
        .filter(|line| !line.trim().is_empty())
        .take(8)
        .collect::<Vec<_>>();
    if tail.is_empty() {
        "no stderr".to_string()
    } else {
        tail.into_iter().rev().collect::<Vec<_>>().join(" | ")
    }
}

fn discover_blueprint_tool(project: &Path) -> Option<BlueprintTool> {
    let root = discover_blueprint_root(project);
    let mut envs = Vec::new();
    if let Some(root) = root.as_ref() {
        let target_spec = env_path(TRUEOS_BLUEPRINT_TARGET_SPEC_ENV)
            .or_else(|| Some(root.join("target.json")))
            .filter(|path| path.is_file())?;
        envs.push((
            TRUEOS_BLUEPRINT_TARGET_SPEC_ENV.to_string(),
            target_spec.into_os_string(),
        ));

        let cache_dir = env_path(TRUEOS_BLUEPRINT_CARGO_CACHE_DIR_ENV)
            .unwrap_or_else(|| root.join("target/trueos-blueprint/cargo-cache"));
        envs.push((
            TRUEOS_BLUEPRINT_CARGO_CACHE_DIR_ENV.to_string(),
            cache_dir.into_os_string(),
        ));

        if let Some(kernel_root) = discover_kernel_root(root) {
            envs.push((
                TRUEOS_BLUEPRINT_KERNEL_ROOT_ENV.to_string(),
                kernel_root.into_os_string(),
            ));
        }
    }

    if let Some(bin) = env_path(TRUEOS_BLUEPRINT_BIN_ENV).filter(|path| path.is_file()) {
        return Some(BlueprintTool {
            program: bin,
            args: vec![project.as_os_str().to_os_string()],
            cwd: project.to_path_buf(),
            envs,
        });
    }

    let root = root?;
    let bin = root.join("target/debug/trueos-blueprint");
    if bin.is_file() {
        return Some(BlueprintTool {
            program: bin,
            args: vec![project.as_os_str().to_os_string()],
            cwd: root,
            envs,
        });
    }

    Some(BlueprintTool {
        program: PathBuf::from("cargo"),
        args: vec![
            OsString::from("run"),
            OsString::from("--quiet"),
            OsString::from("--bin"),
            OsString::from("trueos-blueprint"),
            OsString::from("--"),
            project.as_os_str().to_os_string(),
        ],
        cwd: root,
        envs,
    })
}

fn discover_blueprint_root(project: &Path) -> Option<PathBuf> {
    if let Some(root) = env_path(TRUEOS_BLUEPRINT_ROOT_ENV).filter(|path| is_blueprint_root(path)) {
        return Some(root);
    }

    for ancestor in project.ancestors() {
        if is_blueprint_root(ancestor) {
            return Some(ancestor.to_path_buf());
        }

        let sibling = ancestor.join("TRUEOS Blueprints");
        if is_blueprint_root(&sibling) {
            return Some(sibling);
        }
    }

    None
}

fn is_blueprint_root(path: &Path) -> bool {
    path.join("Cargo.toml").is_file()
        && path.join("target.json").is_file()
        && path.join("src/main.rs").is_file()
}

fn discover_kernel_root(blueprint_root: &Path) -> Option<PathBuf> {
    if let Some(root) = env_path(TRUEOS_BLUEPRINT_KERNEL_ROOT_ENV)
        .filter(|path| path.join("Cargo.toml").is_file() && path.join("vendor").is_dir())
    {
        return Some(root);
    }

    let sibling = blueprint_root.parent()?.join("TRUEOS");
    (sibling.join("Cargo.toml").is_file() && sibling.join("vendor").is_dir()).then_some(sibling)
}

fn env_path(name: &str) -> Option<PathBuf> {
    let value = env::var_os(name)?;
    if value.is_empty() {
        return None;
    }
    let path = PathBuf::from(value);
    if path.is_absolute() {
        Some(path)
    } else {
        env::current_dir().ok().map(|cwd| cwd.join(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn generate_job_records_stage_status() {
        let dir = tempfile::tempdir().unwrap();
        let project = generator::create_project(dir.path(), "Stage Check").unwrap();
        let manager = JobManager::new();

        let id = manager
            .spawn(JobKind::Generate {
                project: project.root.clone(),
            })
            .await;
        let job = wait_for_terminal_job(&manager, id).await;

        assert_eq!(job.status, JobStatus::Passed);
        assert_eq!(job.current_stage, Some(JobStage::Generate));
        assert_eq!(job.stages.len(), 1);
        assert_eq!(job.stages[0].status, JobStatus::Passed);
    }

    async fn wait_for_terminal_job(manager: &JobManager, id: Uuid) -> Job {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let job = manager.get(id).await.expect("job should exist");
            if matches!(job.status, JobStatus::Passed | JobStatus::Failed) {
                return job;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "job did not finish in time"
            );
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    }
}

impl Default for JobManager {
    fn default() -> Self {
        Self::new()
    }
}
