//! # Workflow Orchestrator — AI Workflow Manager
//!
//! Ties together Media, Avatar, Sandbox, and Agent systems into
//! orchestrated workflows. Each workflow is a DAG of steps that
//! can spawn VMs, process media, animate avatars, and run agent tasks.
//!
//! ```text
//! ┌──────────────────────────────────────────────────┐
//! │              Workflow Orchestrator                │
//! │                                                  │
//! │  ┌────────┐   ┌─────────┐   ┌──────────┐       │
//! │  │ Media  │   │ Avatar  │   │ Sandbox  │       │
//! │  │Pipeline│   │ Scene   │   │ Manager  │       │
//! │  └───┬────┘   └────┬────┘   └────┬─────┘       │
//! │      │             │             │              │
//! │  ┌───┴─────────────┴─────────────┴───┐          │
//! │  │         Step Executor              │          │
//! │  │  (DAG scheduler + state machine)   │          │
//! │  └────────────────────────────────────┘          │
//! └──────────────────────────────────────────────────┘
//! ```

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// =============================================================================
// WORKFLOW STEP TYPES
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StepType {
    /// Transcode/process media file
    MediaTranscode { input: String, output: String, profile: String },
    /// Start a live stream
    MediaStream { source: String, output_url: String, protocol: String },
    /// Record a stream/source
    MediaRecord { source: String, output_dir: String, segment_secs: u32 },
    /// Spawn and animate a 3D avatar
    AvatarSpawn { name: String, initial_state: String },
    /// Change avatar state/expression
    AvatarAnimate { name: String, state: String, expression: Option<String> },
    /// Start avatar autostream
    AvatarStream { config_preset: String },
    /// Run a task in a sandboxed VM
    SandboxRun { name: String, task: String, config_preset: String },
    /// Capture vision output from a running VM
    SandboxVision { vm_name: String, frames: u32 },
    /// Run an agent task (from fleet)
    AgentTask { agent_id: String, prompt: String },
    /// Execute a shell command (on host, with restrictions)
    ShellExec { command: String, timeout_secs: u32 },
    /// Wait for a duration
    Wait { seconds: u32 },
    /// Send a webhook/notification
    Notify { target: String, message: String },
    /// Conditional branch
    Condition { check: String, on_true: Vec<String>, on_false: Vec<String> },
}

impl StepType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::MediaTranscode { .. } => "media:transcode",
            Self::MediaStream { .. } => "media:stream",
            Self::MediaRecord { .. } => "media:record",
            Self::AvatarSpawn { .. } => "avatar:spawn",
            Self::AvatarAnimate { .. } => "avatar:animate",
            Self::AvatarStream { .. } => "avatar:stream",
            Self::SandboxRun { .. } => "sandbox:run",
            Self::SandboxVision { .. } => "sandbox:vision",
            Self::AgentTask { .. } => "agent:task",
            Self::ShellExec { .. } => "shell:exec",
            Self::Wait { .. } => "wait",
            Self::Notify { .. } => "notify",
            Self::Condition { .. } => "condition",
        }
    }
}

// =============================================================================
// WORKFLOW STEP
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    Pending,
    Running,
    Success,
    Failed,
    Skipped,
    Cancelled,
}

impl std::fmt::Display for StepStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Success => write!(f, "success"),
            Self::Failed => write!(f, "failed"),
            Self::Skipped => write!(f, "skipped"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub name: String,
    pub step_type: StepType,
    pub status: StepStatus,
    pub depends_on: Vec<String>,
    pub output: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub retries: u32,
    pub max_retries: u32,
}

impl WorkflowStep {
    pub fn new(id: &str, name: &str, step_type: StepType) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            step_type,
            status: StepStatus::Pending,
            depends_on: Vec::new(),
            output: None,
            error: None,
            duration_ms: 0,
            retries: 0,
            max_retries: 2,
        }
    }

    pub fn depends(mut self, dep_ids: &[&str]) -> Self {
        self.depends_on = dep_ids.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn is_ready(&self, completed: &[String]) -> bool {
        self.status == StepStatus::Pending
            && self.depends_on.iter().all(|d| completed.contains(d))
    }

    pub fn can_retry(&self) -> bool {
        self.retries < self.max_retries && self.status == StepStatus::Failed
    }
}

// =============================================================================
// WORKFLOW DEFINITION
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowStatus {
    Draft,
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for WorkflowStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draft => write!(f, "draft"),
            Self::Queued => write!(f, "queued"),
            Self::Running => write!(f, "running"),
            Self::Paused => write!(f, "paused"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub steps: Vec<WorkflowStep>,
    pub status: WorkflowStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
    pub current_step_idx: usize,
    pub fail_fast: bool,
}

impl Workflow {
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            description: description.to_string(),
            steps: Vec::new(),
            status: WorkflowStatus::Draft,
            created_at: chrono::Utc::now(),
            started_at: None,
            finished_at: None,
            current_step_idx: 0,
            fail_fast: true,
        }
    }

    pub fn add_step(&mut self, step: WorkflowStep) {
        self.steps.push(step);
    }

    pub fn completed_step_ids(&self) -> Vec<String> {
        self.steps.iter()
            .filter(|s| s.status == StepStatus::Success)
            .map(|s| s.id.clone())
            .collect()
    }

    pub fn next_ready_steps(&self) -> Vec<usize> {
        let completed = self.completed_step_ids();
        self.steps.iter().enumerate()
            .filter(|(_, s)| s.is_ready(&completed))
            .map(|(i, _)| i)
            .collect()
    }

    pub fn start(&mut self) {
        self.status = WorkflowStatus::Running;
        self.started_at = Some(chrono::Utc::now());
    }

    pub fn complete_step(&mut self, step_idx: usize, output: Option<String>) {
        if let Some(step) = self.steps.get_mut(step_idx) {
            step.status = StepStatus::Success;
            step.output = output;
        }
        self.check_completion();
    }

    pub fn fail_step(&mut self, step_idx: usize, error: &str) {
        if let Some(step) = self.steps.get_mut(step_idx) {
            step.status = StepStatus::Failed;
            step.error = Some(error.to_string());
        }
        if self.fail_fast {
            self.status = WorkflowStatus::Failed;
            self.finished_at = Some(chrono::Utc::now());
        } else {
            self.check_completion();
        }
    }

    fn check_completion(&mut self) {
        let all_done = self.steps.iter()
            .all(|s| matches!(s.status, StepStatus::Success | StepStatus::Skipped | StepStatus::Failed));
        if all_done {
            let any_failed = self.steps.iter().any(|s| s.status == StepStatus::Failed);
            self.status = if any_failed { WorkflowStatus::Failed } else { WorkflowStatus::Completed };
            self.finished_at = Some(chrono::Utc::now());
        }
    }

    pub fn progress(&self) -> f32 {
        let done = self.steps.iter()
            .filter(|s| matches!(s.status, StepStatus::Success | StepStatus::Skipped))
            .count();
        if self.steps.is_empty() { return 0.0; }
        done as f32 / self.steps.len() as f32
    }

    pub fn duration_ms(&self) -> u64 {
        match (self.started_at, self.finished_at) {
            (Some(s), Some(e)) => (e - s).num_milliseconds().max(0) as u64,
            (Some(s), None) => (chrono::Utc::now() - s).num_milliseconds().max(0) as u64,
            _ => 0,
        }
    }
}

// =============================================================================
// WORKFLOW ORCHESTRATOR
// =============================================================================

pub struct WorkflowOrchestrator {
    pub workflows: Vec<Workflow>,
    pub max_concurrent: usize,
    pub history: Vec<Workflow>,
    pub max_history: usize,
}

impl WorkflowOrchestrator {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            workflows: Vec::new(),
            max_concurrent,
            history: Vec::new(),
            max_history: 100,
        }
    }

    pub fn create_workflow(&mut self, name: &str, description: &str) -> &mut Workflow {
        let wf = Workflow::new(name, description);
        self.workflows.push(wf);
        self.workflows.last_mut().unwrap()
    }

    pub fn queue_workflow(&mut self, id: Uuid) -> Result<(), String> {
        let running = self.workflows.iter()
            .filter(|w| w.status == WorkflowStatus::Running)
            .count();
        if running >= self.max_concurrent {
            return Err("Max concurrent workflows reached".to_string());
        }
        if let Some(wf) = self.workflows.iter_mut().find(|w| w.id == id) {
            wf.status = WorkflowStatus::Queued;
            Ok(())
        } else {
            Err("Workflow not found".to_string())
        }
    }

    pub fn start_workflow(&mut self, id: Uuid) -> Result<(), String> {
        if let Some(wf) = self.workflows.iter_mut().find(|w| w.id == id) {
            wf.start();
            Ok(())
        } else {
            Err("Workflow not found".to_string())
        }
    }

    pub fn archive_completed(&mut self) {
        let (done, active): (Vec<_>, Vec<_>) = self.workflows.drain(..)
            .partition(|w| matches!(w.status, WorkflowStatus::Completed | WorkflowStatus::Failed | WorkflowStatus::Cancelled));
        self.workflows = active;
        self.history.extend(done);
        while self.history.len() > self.max_history {
            self.history.remove(0);
        }
    }

    pub fn status(&self) -> OrchestratorStatus {
        let total = self.workflows.len();
        let running = self.workflows.iter().filter(|w| w.status == WorkflowStatus::Running).count();
        let queued = self.workflows.iter().filter(|w| w.status == WorkflowStatus::Queued).count();
        let drafts = self.workflows.iter().filter(|w| w.status == WorkflowStatus::Draft).count();
        let completed_history = self.history.iter().filter(|w| w.status == WorkflowStatus::Completed).count();
        let failed_history = self.history.iter().filter(|w| w.status == WorkflowStatus::Failed).count();

        let total_steps: usize = self.workflows.iter().map(|w| w.steps.len()).sum();
        let step_types: Vec<String> = self.workflows.iter()
            .flat_map(|w| w.steps.iter())
            .map(|s| s.step_type.label().to_string())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        OrchestratorStatus {
            max_concurrent: self.max_concurrent,
            active_workflows: total,
            running,
            queued,
            drafts,
            history_completed: completed_history,
            history_failed: failed_history,
            total_steps,
            step_types,
            workflows: self.workflows.iter().map(|w| WorkflowInfo {
                id: w.id.to_string(),
                name: w.name.clone(),
                status: w.status.to_string(),
                steps: w.steps.len(),
                progress: w.progress(),
                duration_ms: w.duration_ms(),
            }).collect(),
        }
    }

    pub fn report(&self) -> String {
        let s = self.status();
        let mut out = format!(
            "=== Workflow Orchestrator ===\n\
             Max Concurrent: {}\n\
             Active:         {} ({} running, {} queued, {} drafts)\n\
             History:        {} completed, {} failed\n\
             Total Steps:    {}\n\
             Step Types:     {}\n",
            s.max_concurrent,
            s.active_workflows, s.running, s.queued, s.drafts,
            s.history_completed, s.history_failed,
            s.total_steps,
            if s.step_types.is_empty() { "none".to_string() } else { s.step_types.join(", ") },
        );
        for wf in &s.workflows {
            out.push_str(&format!(
                "  [{}] {} — {} ({} steps, {:.0}% done, {}ms)\n",
                &wf.id[..8], wf.name, wf.status, wf.steps, wf.progress * 100.0, wf.duration_ms,
            ));
        }
        out
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorStatus {
    pub max_concurrent: usize,
    pub active_workflows: usize,
    pub running: usize,
    pub queued: usize,
    pub drafts: usize,
    pub history_completed: usize,
    pub history_failed: usize,
    pub total_steps: usize,
    pub step_types: Vec<String>,
    pub workflows: Vec<WorkflowInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub steps: usize,
    pub progress: f32,
    pub duration_ms: u64,
}

// =============================================================================
// PRESET WORKFLOWS (templates)
// =============================================================================

pub fn preset_avatar_stream_workflow() -> Workflow {
    let mut wf = Workflow::new(
        "avatar-autostream",
        "Spawn avatar, start autostream, record output",
    );
    wf.add_step(WorkflowStep::new("spawn", "Spawn Avatar", StepType::AvatarSpawn {
        name: "Apophy".into(),
        initial_state: "idle".into(),
    }));
    wf.add_step(WorkflowStep::new("stream", "Start AutoStream", StepType::AvatarStream {
        config_preset: "hd".into(),
    }).depends(&["spawn"]));
    wf.add_step(WorkflowStep::new("record", "Record Stream", StepType::MediaRecord {
        source: "rtmp://localhost/live/avatar".into(),
        output_dir: "/tmp/recordings".into(),
        segment_secs: 60,
    }).depends(&["stream"]));
    wf
}

pub fn preset_sandbox_test_workflow() -> Workflow {
    let mut wf = Workflow::new(
        "sandbox-test",
        "Run tests in isolated VM with vision capture",
    );
    wf.add_step(WorkflowStep::new("vm", "Spawn Test VM", StepType::SandboxRun {
        name: "test-runner".into(),
        task: "cargo test --workspace".into(),
        config_preset: "default".into(),
    }));
    wf.add_step(WorkflowStep::new("vision", "Capture Vision", StepType::SandboxVision {
        vm_name: "test-runner".into(),
        frames: 10,
    }).depends(&["vm"]));
    wf.add_step(WorkflowStep::new("notify", "Send Result", StepType::Notify {
        target: "agent@iagenticflow.org".into(),
        message: "Test run complete".into(),
    }).depends(&["vision"]));
    wf
}

pub fn preset_media_pipeline_workflow() -> Workflow {
    let mut wf = Workflow::new(
        "media-pipeline",
        "Transcode input, create HLS stream, record archive",
    );
    wf.add_step(WorkflowStep::new("transcode", "Transcode to H264", StepType::MediaTranscode {
        input: "input.mp4".into(),
        output: "output_h264.mp4".into(),
        profile: "hd".into(),
    }));
    wf.add_step(WorkflowStep::new("stream", "Start HLS Stream", StepType::MediaStream {
        source: "output_h264.mp4".into(),
        output_url: "/tmp/hls".into(),
        protocol: "hls".into(),
    }).depends(&["transcode"]));
    wf.add_step(WorkflowStep::new("archive", "Archive Recording", StepType::MediaRecord {
        source: "output_h264.mp4".into(),
        output_dir: "/tmp/archive".into(),
        segment_secs: 300,
    }).depends(&["transcode"]));
    wf
}

pub fn preset_full_pipeline_workflow() -> Workflow {
    let mut wf = Workflow::new(
        "full-sovereign-pipeline",
        "Complete: VM sandbox → media process → avatar stream → notify",
    );
    wf.fail_fast = false;
    wf.add_step(WorkflowStep::new("sandbox", "Run in Sandbox", StepType::SandboxRun {
        name: "sovereign-build".into(),
        task: "cargo build --release".into(),
        config_preset: "heavy".into(),
    }));
    wf.add_step(WorkflowStep::new("transcode", "Process Media", StepType::MediaTranscode {
        input: "raw_capture.mp4".into(),
        output: "processed.mp4".into(),
        profile: "hd".into(),
    }).depends(&["sandbox"]));
    wf.add_step(WorkflowStep::new("avatar", "Spawn Presenter", StepType::AvatarSpawn {
        name: "Presenter".into(),
        initial_state: "presenting".into(),
    }).depends(&["sandbox"]));
    wf.add_step(WorkflowStep::new("stream", "Avatar AutoStream", StepType::AvatarStream {
        config_preset: "hd".into(),
    }).depends(&["avatar"]));
    wf.add_step(WorkflowStep::new("notify", "Notify Complete", StepType::Notify {
        target: "agent@iagenticflow.org".into(),
        message: "Full pipeline complete".into(),
    }).depends(&["transcode", "stream"]));
    wf
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_creation() {
        let wf = Workflow::new("test", "test workflow");
        assert_eq!(wf.status, WorkflowStatus::Draft);
        assert!(wf.steps.is_empty());
    }

    #[test]
    fn test_workflow_step_dependencies() {
        let mut wf = Workflow::new("test", "dep test");
        wf.add_step(WorkflowStep::new("a", "Step A", StepType::Wait { seconds: 1 }));
        wf.add_step(WorkflowStep::new("b", "Step B", StepType::Wait { seconds: 1 }).depends(&["a"]));

        let ready = wf.next_ready_steps();
        assert_eq!(ready, vec![0]); // only step A is ready

        wf.complete_step(0, Some("done".into()));
        let ready = wf.next_ready_steps();
        assert_eq!(ready, vec![1]); // now step B is ready
    }

    #[test]
    fn test_workflow_lifecycle() {
        let mut wf = Workflow::new("lc", "lifecycle test");
        wf.add_step(WorkflowStep::new("s1", "Step 1", StepType::Wait { seconds: 1 }));
        wf.add_step(WorkflowStep::new("s2", "Step 2", StepType::Wait { seconds: 1 }).depends(&["s1"]));

        wf.start();
        assert_eq!(wf.status, WorkflowStatus::Running);
        assert!(wf.started_at.is_some());

        wf.complete_step(0, None);
        assert_eq!(wf.progress(), 0.5);

        wf.complete_step(1, None);
        assert_eq!(wf.status, WorkflowStatus::Completed);
        assert!(wf.finished_at.is_some());
    }

    #[test]
    fn test_workflow_fail_fast() {
        let mut wf = Workflow::new("ff", "fail fast");
        wf.fail_fast = true;
        wf.add_step(WorkflowStep::new("s1", "Step 1", StepType::Wait { seconds: 1 }));
        wf.add_step(WorkflowStep::new("s2", "Step 2", StepType::Wait { seconds: 1 }));

        wf.start();
        wf.fail_step(0, "boom");
        assert_eq!(wf.status, WorkflowStatus::Failed);
    }

    #[test]
    fn test_workflow_no_fail_fast() {
        let mut wf = Workflow::new("nff", "no fail fast");
        wf.fail_fast = false;
        wf.add_step(WorkflowStep::new("s1", "Step 1", StepType::Wait { seconds: 1 }));
        wf.add_step(WorkflowStep::new("s2", "Step 2", StepType::Wait { seconds: 1 }));

        wf.start();
        wf.fail_step(0, "boom");
        // still running because fail_fast is false and step 2 isn't done
        assert_eq!(wf.status, WorkflowStatus::Running);

        wf.complete_step(1, None);
        assert_eq!(wf.status, WorkflowStatus::Failed); // one step failed
    }

    #[test]
    fn test_step_types() {
        let step = WorkflowStep::new("t", "test", StepType::MediaTranscode {
            input: "a.mp4".into(), output: "b.mp4".into(), profile: "hd".into(),
        });
        assert_eq!(step.step_type.label(), "media:transcode");
    }

    #[test]
    fn test_orchestrator() {
        let mut orch = WorkflowOrchestrator::new(3);
        let wf = orch.create_workflow("test", "test wf");
        let id = wf.id;
        wf.add_step(WorkflowStep::new("s1", "Step", StepType::Wait { seconds: 1 }));

        orch.queue_workflow(id).unwrap();
        orch.start_workflow(id).unwrap();

        let status = orch.status();
        assert_eq!(status.running, 1);
        assert_eq!(status.total_steps, 1);
    }

    #[test]
    fn test_orchestrator_archive() {
        let mut orch = WorkflowOrchestrator::new(5);
        let wf = orch.create_workflow("done", "completed wf");
        let id = wf.id;
        wf.add_step(WorkflowStep::new("s1", "Step", StepType::Wait { seconds: 1 }));

        orch.start_workflow(id).unwrap();
        orch.workflows[0].complete_step(0, None);

        orch.archive_completed();
        assert!(orch.workflows.is_empty());
        assert_eq!(orch.history.len(), 1);
    }

    #[test]
    fn test_orchestrator_report() {
        let mut orch = WorkflowOrchestrator::new(5);
        orch.create_workflow("test", "test wf");
        let report = orch.report();
        assert!(report.contains("Workflow Orchestrator"));
    }

    #[test]
    fn test_preset_avatar_stream() {
        let wf = preset_avatar_stream_workflow();
        assert_eq!(wf.steps.len(), 3);
        assert_eq!(wf.steps[0].step_type.label(), "avatar:spawn");
        assert!(wf.steps[1].depends_on.contains(&"spawn".to_string()));
    }

    #[test]
    fn test_preset_sandbox_test() {
        let wf = preset_sandbox_test_workflow();
        assert_eq!(wf.steps.len(), 3);
        assert_eq!(wf.steps[0].step_type.label(), "sandbox:run");
    }

    #[test]
    fn test_preset_media_pipeline() {
        let wf = preset_media_pipeline_workflow();
        assert_eq!(wf.steps.len(), 3);
        // stream and archive both depend on transcode (parallel after transcode)
        assert!(wf.steps[1].depends_on.contains(&"transcode".to_string()));
        assert!(wf.steps[2].depends_on.contains(&"transcode".to_string()));
    }

    #[test]
    fn test_preset_full_pipeline() {
        let wf = preset_full_pipeline_workflow();
        assert_eq!(wf.steps.len(), 5);
        assert!(!wf.fail_fast);
        // notify depends on both transcode and stream
        assert!(wf.steps[4].depends_on.contains(&"transcode".to_string()));
        assert!(wf.steps[4].depends_on.contains(&"stream".to_string()));
    }

    #[test]
    fn test_step_can_retry() {
        let mut step = WorkflowStep::new("r", "retry", StepType::Wait { seconds: 1 });
        step.max_retries = 3;
        step.status = StepStatus::Failed;
        assert!(step.can_retry());

        step.retries = 3;
        assert!(!step.can_retry());
    }

    #[test]
    fn test_workflow_progress_empty() {
        let wf = Workflow::new("empty", "no steps");
        assert_eq!(wf.progress(), 0.0);
    }
}
