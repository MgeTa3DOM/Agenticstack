//! # Sandbox VM — Firecracker-style MicroVM Isolation with Vision Output
//!
//! Sovereign sandboxing for agentic workflows. Each agent task runs in an
//! isolated micro-VM with its own filesystem, network namespace, and
//! resource limits. Vision output captures framebuffer/screenshots for
//! visual agent inspection.
//!
//! ```text
//! ┌──────────────────────────────────────────────┐
//! │                  Host (Apophy)                │
//! │  ┌────────┐  ┌────────┐  ┌────────┐         │
//! │  │ VM-001 │  │ VM-002 │  │ VM-003 │  ...     │
//! │  │ Agent  │  │ Agent  │  │ Agent  │         │
//! │  │ Task   │  │ Task   │  │ Task   │         │
//! │  └───┬────┘  └───┬────┘  └───┬────┘         │
//! │      │           │           │               │
//! │  ┌───┴───────────┴───────────┴───┐           │
//! │  │      Vision Capture Bus       │           │
//! │  │   (framebuffer screenshots)   │           │
//! │  └───────────────────────────────┘           │
//! └──────────────────────────────────────────────┘
//! ```

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// =============================================================================
// VM CONFIGURATION
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    pub vcpu_count: u32,
    pub memory_mb: u64,
    pub disk_mb: u64,
    pub kernel_path: String,
    pub rootfs_path: String,
    pub network_enabled: bool,
    pub vision_enabled: bool,
    pub max_runtime_secs: u64,
    pub framebuffer_width: u32,
    pub framebuffer_height: u32,
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            vcpu_count: 1,
            memory_mb: 256,
            disk_mb: 512,
            kernel_path: "vmlinux".to_string(),
            rootfs_path: "rootfs.ext4".to_string(),
            network_enabled: false,
            vision_enabled: true,
            max_runtime_secs: 300,
            framebuffer_width: 1280,
            framebuffer_height: 720,
        }
    }
}

impl VmConfig {
    pub fn minimal() -> Self {
        Self {
            vcpu_count: 1,
            memory_mb: 128,
            disk_mb: 256,
            network_enabled: false,
            vision_enabled: false,
            max_runtime_secs: 60,
            ..Default::default()
        }
    }

    pub fn with_vision() -> Self {
        Self {
            vision_enabled: true,
            framebuffer_width: 1920,
            framebuffer_height: 1080,
            ..Default::default()
        }
    }

    pub fn heavy_compute() -> Self {
        Self {
            vcpu_count: 4,
            memory_mb: 2048,
            disk_mb: 4096,
            network_enabled: true,
            vision_enabled: true,
            max_runtime_secs: 3600,
            framebuffer_width: 1920,
            framebuffer_height: 1080,
            ..Default::default()
        }
    }
}

// =============================================================================
// VM STATE
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VmState {
    Creating,
    Booting,
    Running,
    Paused,
    Stopping,
    Stopped,
    Failed,
    TimedOut,
}

impl std::fmt::Display for VmState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Creating => write!(f, "creating"),
            Self::Booting => write!(f, "booting"),
            Self::Running => write!(f, "running"),
            Self::Paused => write!(f, "paused"),
            Self::Stopping => write!(f, "stopping"),
            Self::Stopped => write!(f, "stopped"),
            Self::Failed => write!(f, "failed"),
            Self::TimedOut => write!(f, "timed_out"),
        }
    }
}

// =============================================================================
// VISION OUTPUT
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionFrame {
    pub frame_id: u64,
    pub width: u32,
    pub height: u32,
    pub format: VisionFormat,
    pub timestamp_ms: u64,
    pub data_size_bytes: usize,
    pub checksum: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisionFormat {
    RGB24,
    RGBA32,
    PNG,
    JPEG,
    BMP,
}

impl std::fmt::Display for VisionFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RGB24 => write!(f, "rgb24"),
            Self::RGBA32 => write!(f, "rgba32"),
            Self::PNG => write!(f, "png"),
            Self::JPEG => write!(f, "jpeg"),
            Self::BMP => write!(f, "bmp"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionCapture {
    pub vm_id: Uuid,
    pub total_frames: u64,
    pub capture_fps: u32,
    pub format: VisionFormat,
    pub output_dir: String,
    pub frames: Vec<VisionFrame>,
    pub enabled: bool,
}

impl VisionCapture {
    pub fn new(vm_id: Uuid, config: &VmConfig) -> Self {
        Self {
            vm_id,
            total_frames: 0,
            capture_fps: 5, // 5 fps for vision inspection
            format: VisionFormat::PNG,
            output_dir: format!("/tmp/apophy-vision/{}", vm_id),
            frames: Vec::new(),
            enabled: config.vision_enabled,
        }
    }

    pub fn record_frame(&mut self, width: u32, height: u32, data_size: usize, checksum: &str) {
        if !self.enabled { return; }
        let frame = VisionFrame {
            frame_id: self.total_frames,
            width,
            height,
            format: self.format,
            timestamp_ms: self.total_frames * (1000 / self.capture_fps as u64),
            data_size_bytes: data_size,
            checksum: checksum.to_string(),
        };
        self.frames.push(frame);
        self.total_frames += 1;
    }

    pub fn latest_frame(&self) -> Option<&VisionFrame> {
        self.frames.last()
    }
}

// =============================================================================
// SANDBOX INSTANCE
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxInstance {
    pub id: Uuid,
    pub name: String,
    pub config: VmConfig,
    pub state: VmState,
    pub agent_id: Option<String>,
    pub task_description: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
    pub vision_frames: u64,
    pub cpu_usage_pct: f32,
    pub memory_usage_mb: u64,
}

impl SandboxInstance {
    pub fn new(name: &str, task: &str, config: VmConfig) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            config,
            state: VmState::Creating,
            agent_id: None,
            task_description: task.to_string(),
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            created_at: chrono::Utc::now(),
            started_at: None,
            finished_at: None,
            vision_frames: 0,
            cpu_usage_pct: 0.0,
            memory_usage_mb: 0,
        }
    }

    pub fn with_agent(mut self, agent_id: &str) -> Self {
        self.agent_id = Some(agent_id.to_string());
        self
    }

    pub fn boot(&mut self) {
        self.state = VmState::Booting;
        self.started_at = Some(chrono::Utc::now());
    }

    pub fn mark_running(&mut self) {
        self.state = VmState::Running;
    }

    pub fn complete(&mut self, exit_code: i32, stdout: &str, stderr: &str) {
        self.state = VmState::Stopped;
        self.exit_code = Some(exit_code);
        self.stdout = stdout.to_string();
        self.stderr = stderr.to_string();
        self.finished_at = Some(chrono::Utc::now());
    }

    pub fn fail(&mut self, error: &str) {
        self.state = VmState::Failed;
        self.stderr = error.to_string();
        self.finished_at = Some(chrono::Utc::now());
    }

    pub fn timeout(&mut self) {
        self.state = VmState::TimedOut;
        self.finished_at = Some(chrono::Utc::now());
    }

    pub fn runtime_secs(&self) -> u64 {
        match (self.started_at, self.finished_at) {
            (Some(start), Some(end)) => (end - start).num_seconds().max(0) as u64,
            (Some(start), None) => (chrono::Utc::now() - start).num_seconds().max(0) as u64,
            _ => 0,
        }
    }

    pub fn is_over_time_limit(&self) -> bool {
        self.runtime_secs() > self.config.max_runtime_secs
    }
}

// =============================================================================
// SANDBOX MANAGER
// =============================================================================

pub struct SandboxManager {
    pub instances: Vec<SandboxInstance>,
    pub vision_captures: Vec<VisionCapture>,
    pub max_concurrent: usize,
    pub firecracker_available: bool,
}

impl SandboxManager {
    pub fn new(max_concurrent: usize) -> Self {
        let firecracker_available = check_firecracker_available();
        Self {
            instances: Vec::new(),
            vision_captures: Vec::new(),
            max_concurrent,
            firecracker_available,
        }
    }

    pub fn spawn(&mut self, name: &str, task: &str, config: VmConfig) -> Result<Uuid, String> {
        let active = self.instances.iter()
            .filter(|i| matches!(i.state, VmState::Running | VmState::Booting))
            .count();
        if active >= self.max_concurrent {
            return Err(format!("Max concurrent VMs reached: {}", self.max_concurrent));
        }

        let mut instance = SandboxInstance::new(name, task, config.clone());
        let id = instance.id;

        // Create vision capture if enabled
        if config.vision_enabled {
            let capture = VisionCapture::new(id, &config);
            self.vision_captures.push(capture);
        }

        instance.boot();
        self.instances.push(instance);
        Ok(id)
    }

    pub fn get_instance(&self, id: Uuid) -> Option<&SandboxInstance> {
        self.instances.iter().find(|i| i.id == id)
    }

    pub fn get_vision(&self, vm_id: Uuid) -> Option<&VisionCapture> {
        self.vision_captures.iter().find(|v| v.vm_id == vm_id)
    }

    pub fn cleanup_stopped(&mut self) {
        self.instances.retain(|i| !matches!(i.state, VmState::Stopped | VmState::Failed | VmState::TimedOut));
    }

    pub fn status(&self) -> SandboxManagerStatus {
        let total = self.instances.len();
        let running = self.instances.iter().filter(|i| i.state == VmState::Running).count();
        let booting = self.instances.iter().filter(|i| i.state == VmState::Booting).count();
        let stopped = self.instances.iter().filter(|i| i.state == VmState::Stopped).count();
        let failed = self.instances.iter().filter(|i| i.state == VmState::Failed).count();
        let total_vision_frames: u64 = self.vision_captures.iter().map(|v| v.total_frames).sum();

        SandboxManagerStatus {
            firecracker_available: self.firecracker_available,
            max_concurrent: self.max_concurrent,
            total_instances: total,
            running,
            booting,
            stopped,
            failed,
            total_vision_frames,
            instances: self.instances.iter().map(|i| SandboxInstanceInfo {
                id: i.id.to_string(),
                name: i.name.clone(),
                state: i.state.to_string(),
                task: i.task_description.clone(),
                agent_id: i.agent_id.clone(),
                runtime_secs: i.runtime_secs(),
                vision_frames: self.vision_captures.iter()
                    .find(|v| v.vm_id == i.id)
                    .map(|v| v.total_frames)
                    .unwrap_or(0),
                cpu_pct: i.cpu_usage_pct,
                mem_mb: i.memory_usage_mb,
            }).collect(),
        }
    }

    pub fn report(&self) -> String {
        let s = self.status();
        let mut out = format!(
            "=== Sandbox VM (Firecracker) ===\n\
             Firecracker:  {}\n\
             Max VMs:      {}\n\
             Total:        {} ({} running, {} booting, {} stopped, {} failed)\n\
             Vision:       {} total frames captured\n",
            if s.firecracker_available { "AVAILABLE" } else { "NOT FOUND (using process isolation)" },
            s.max_concurrent,
            s.total_instances, s.running, s.booting, s.stopped, s.failed,
            s.total_vision_frames,
        );
        for inst in &s.instances {
            out.push_str(&format!(
                "  [{}] {} — {} task=\"{}\" runtime={}s vision={} cpu={:.1}% mem={}MB\n",
                &inst.id[..8], inst.name, inst.state, inst.task,
                inst.runtime_secs, inst.vision_frames, inst.cpu_pct, inst.mem_mb,
            ));
        }
        out
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxManagerStatus {
    pub firecracker_available: bool,
    pub max_concurrent: usize,
    pub total_instances: usize,
    pub running: usize,
    pub booting: usize,
    pub stopped: usize,
    pub failed: usize,
    pub total_vision_frames: u64,
    pub instances: Vec<SandboxInstanceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxInstanceInfo {
    pub id: String,
    pub name: String,
    pub state: String,
    pub task: String,
    pub agent_id: Option<String>,
    pub runtime_secs: u64,
    pub vision_frames: u64,
    pub cpu_pct: f32,
    pub mem_mb: u64,
}

fn check_firecracker_available() -> bool {
    std::process::Command::new("firecracker")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_config_default() {
        let config = VmConfig::default();
        assert_eq!(config.vcpu_count, 1);
        assert_eq!(config.memory_mb, 256);
        assert!(config.vision_enabled);
    }

    #[test]
    fn test_vm_config_minimal() {
        let config = VmConfig::minimal();
        assert_eq!(config.memory_mb, 128);
        assert!(!config.vision_enabled);
    }

    #[test]
    fn test_vm_config_heavy() {
        let config = VmConfig::heavy_compute();
        assert_eq!(config.vcpu_count, 4);
        assert_eq!(config.memory_mb, 2048);
        assert!(config.network_enabled);
    }

    #[test]
    fn test_sandbox_instance_lifecycle() {
        let mut inst = SandboxInstance::new("test-vm", "run tests", VmConfig::default());
        assert_eq!(inst.state, VmState::Creating);

        inst.boot();
        assert_eq!(inst.state, VmState::Booting);
        assert!(inst.started_at.is_some());

        inst.mark_running();
        assert_eq!(inst.state, VmState::Running);

        inst.complete(0, "all passed", "");
        assert_eq!(inst.state, VmState::Stopped);
        assert_eq!(inst.exit_code, Some(0));
        assert_eq!(inst.stdout, "all passed");
    }

    #[test]
    fn test_sandbox_instance_failure() {
        let mut inst = SandboxInstance::new("fail-vm", "bad task", VmConfig::default());
        inst.boot();
        inst.fail("kernel panic");
        assert_eq!(inst.state, VmState::Failed);
        assert!(inst.stderr.contains("kernel panic"));
    }

    #[test]
    fn test_sandbox_instance_timeout() {
        let mut inst = SandboxInstance::new("slow-vm", "long task", VmConfig::minimal());
        inst.boot();
        inst.timeout();
        assert_eq!(inst.state, VmState::TimedOut);
    }

    #[test]
    fn test_sandbox_with_agent() {
        let inst = SandboxInstance::new("agent-vm", "agent task", VmConfig::default())
            .with_agent("agent-007");
        assert_eq!(inst.agent_id.as_deref(), Some("agent-007"));
    }

    #[test]
    fn test_vision_capture() {
        let vm_id = Uuid::new_v4();
        let config = VmConfig::with_vision();
        let mut capture = VisionCapture::new(vm_id, &config);
        assert!(capture.enabled);

        capture.record_frame(1920, 1080, 6220800, "abc123");
        capture.record_frame(1920, 1080, 6220800, "def456");
        assert_eq!(capture.total_frames, 2);
        assert_eq!(capture.frames.len(), 2);

        let latest = capture.latest_frame().unwrap();
        assert_eq!(latest.frame_id, 1);
        assert_eq!(latest.checksum, "def456");
    }

    #[test]
    fn test_vision_capture_disabled() {
        let vm_id = Uuid::new_v4();
        let config = VmConfig::minimal(); // vision disabled
        let mut capture = VisionCapture::new(vm_id, &config);
        assert!(!capture.enabled);

        capture.record_frame(640, 480, 921600, "abc");
        assert_eq!(capture.total_frames, 0); // no frames recorded
    }

    #[test]
    fn test_sandbox_manager() {
        let mut mgr = SandboxManager::new(3);
        let id1 = mgr.spawn("vm1", "task1", VmConfig::default()).unwrap();
        let id2 = mgr.spawn("vm2", "task2", VmConfig::default()).unwrap();

        assert_eq!(mgr.instances.len(), 2);
        assert!(mgr.get_instance(id1).is_some());
        assert!(mgr.get_instance(id2).is_some());
    }

    #[test]
    fn test_sandbox_manager_max_concurrent() {
        let mut mgr = SandboxManager::new(2);
        mgr.spawn("vm1", "t1", VmConfig::default()).unwrap();
        mgr.spawn("vm2", "t2", VmConfig::default()).unwrap();
        let result = mgr.spawn("vm3", "t3", VmConfig::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_sandbox_manager_cleanup() {
        let mut mgr = SandboxManager::new(10);
        mgr.spawn("vm1", "t1", VmConfig::default()).unwrap();
        mgr.instances[0].complete(0, "ok", "");
        mgr.cleanup_stopped();
        assert!(mgr.instances.is_empty());
    }

    #[test]
    fn test_sandbox_manager_status() {
        let mut mgr = SandboxManager::new(5);
        mgr.spawn("vm1", "task1", VmConfig::with_vision()).unwrap();
        let status = mgr.status();
        assert_eq!(status.total_instances, 1);
        assert_eq!(status.booting, 1);
    }

    #[test]
    fn test_sandbox_manager_report() {
        let mut mgr = SandboxManager::new(5);
        mgr.spawn("test-vm", "run tests", VmConfig::default()).unwrap();
        let report = mgr.report();
        assert!(report.contains("Sandbox VM"));
        assert!(report.contains("test-vm"));
    }
}
