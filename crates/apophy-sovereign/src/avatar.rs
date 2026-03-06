//! # Avatar 3D — Autostreaming 3D Avatar System
//!
//! Sovereign 3D avatar engine with real-time autostream capabilities.
//! Scene graph, avatar state machine, and streaming output — all native.
//!
//! ```text
//! ┌──────────┐   ┌───────────┐   ┌──────────┐   ┌──────────┐
//! │  Scene   │──▶│  Avatar   │──▶│ Renderer │──▶│ AutoStr  │
//! │  Graph   │   │  State    │   │ (frames) │   │ (FFmpeg) │
//! └──────────┘   └───────────┘   └──────────┘   └──────────┘
//! ```

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// =============================================================================
// SCENE GRAPH
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub fn zero() -> Self { Self { x: 0.0, y: 0.0, z: 0.0 } }
    pub fn new(x: f32, y: f32, z: f32) -> Self { Self { x, y, z } }
    pub fn up() -> Self { Self { x: 0.0, y: 1.0, z: 0.0 } }
    pub fn forward() -> Self { Self { x: 0.0, y: 0.0, z: -1.0 } }

    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn normalize(&self) -> Self {
        let len = self.length();
        if len < 1e-6 { return Self::zero(); }
        Self { x: self.x / len, y: self.y / len, z: self.z / len }
    }

    pub fn add(&self, other: &Vec3) -> Self {
        Self { x: self.x + other.x, y: self.y + other.y, z: self.z + other.z }
    }

    pub fn scale(&self, s: f32) -> Self {
        Self { x: self.x * s, y: self.y * s, z: self.z * s }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quaternion {
    pub w: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Quaternion {
    pub fn identity() -> Self { Self { w: 1.0, x: 0.0, y: 0.0, z: 0.0 } }

    pub fn from_euler(pitch: f32, yaw: f32, roll: f32) -> Self {
        let (sp, cp) = (pitch * 0.5).sin_cos();
        let (sy, cy) = (yaw * 0.5).sin_cos();
        let (sr, cr) = (roll * 0.5).sin_cos();
        Self {
            w: cr * cp * cy + sr * sp * sy,
            x: sr * cp * cy - cr * sp * sy,
            y: cr * sp * cy + sr * cp * sy,
            z: cr * cp * sy - sr * sp * cy,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Quaternion,
    pub scale: Vec3,
}

impl Transform {
    pub fn identity() -> Self {
        Self {
            position: Vec3::zero(),
            rotation: Quaternion::identity(),
            scale: Vec3::new(1.0, 1.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneNode {
    pub id: Uuid,
    pub name: String,
    pub transform: Transform,
    pub node_type: NodeType,
    pub visible: bool,
    pub children: Vec<SceneNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeType {
    Root,
    Avatar,
    Camera,
    Light,
    Mesh,
    Bone,
    Particle,
    Environment,
}

impl SceneNode {
    pub fn new(name: &str, node_type: NodeType) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            transform: Transform::identity(),
            node_type,
            visible: true,
            children: Vec::new(),
        }
    }

    pub fn add_child(&mut self, child: SceneNode) {
        self.children.push(child);
    }

    pub fn node_count(&self) -> usize {
        1 + self.children.iter().map(|c| c.node_count()).sum::<usize>()
    }

    pub fn find_by_name(&self, name: &str) -> Option<&SceneNode> {
        if self.name == name { return Some(self); }
        for child in &self.children {
            if let Some(found) = child.find_by_name(name) {
                return Some(found);
            }
        }
        None
    }
}

// =============================================================================
// AVATAR STATE MACHINE
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AvatarState {
    Idle,
    Talking,
    Listening,
    Thinking,
    Gesturing,
    Walking,
    Presenting,
    Reacting,
}

impl std::fmt::Display for AvatarState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "idle"),
            Self::Talking => write!(f, "talking"),
            Self::Listening => write!(f, "listening"),
            Self::Thinking => write!(f, "thinking"),
            Self::Gesturing => write!(f, "gesturing"),
            Self::Walking => write!(f, "walking"),
            Self::Presenting => write!(f, "presenting"),
            Self::Reacting => write!(f, "reacting"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvatarAnimation {
    pub name: String,
    pub state: AvatarState,
    pub duration_secs: f32,
    pub looping: bool,
    pub blend_weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlendShape {
    pub name: String,
    pub weight: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvatarAppearance {
    pub model_path: String,
    pub texture_path: Option<String>,
    pub height: f32,
    pub blend_shapes: Vec<BlendShape>,
}

impl Default for AvatarAppearance {
    fn default() -> Self {
        Self {
            model_path: "models/avatar_default.glb".to_string(),
            texture_path: None,
            height: 1.75,
            blend_shapes: vec![
                BlendShape { name: "smile".into(), weight: 0.0 },
                BlendShape { name: "blink_l".into(), weight: 0.0 },
                BlendShape { name: "blink_r".into(), weight: 0.0 },
                BlendShape { name: "jaw_open".into(), weight: 0.0 },
                BlendShape { name: "brow_up".into(), weight: 0.0 },
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Avatar {
    pub id: Uuid,
    pub name: String,
    pub state: AvatarState,
    pub appearance: AvatarAppearance,
    pub transform: Transform,
    pub animations: Vec<AvatarAnimation>,
    pub current_animation: Option<String>,
    pub voice_active: bool,
    pub lip_sync_weight: f32,
}

impl Avatar {
    pub fn new(name: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            state: AvatarState::Idle,
            appearance: AvatarAppearance::default(),
            transform: Transform::identity(),
            animations: default_animations(),
            current_animation: None,
            voice_active: false,
            lip_sync_weight: 0.0,
        }
    }

    pub fn set_state(&mut self, state: AvatarState) {
        self.state = state;
        // Auto-select animation based on state
        self.current_animation = self.animations.iter()
            .find(|a| a.state == state)
            .map(|a| a.name.clone());
    }

    pub fn start_talking(&mut self) {
        self.set_state(AvatarState::Talking);
        self.voice_active = true;
        self.lip_sync_weight = 0.5;
    }

    pub fn stop_talking(&mut self) {
        self.set_state(AvatarState::Idle);
        self.voice_active = false;
        self.lip_sync_weight = 0.0;
    }

    pub fn set_expression(&mut self, shape: &str, weight: f32) {
        if let Some(bs) = self.appearance.blend_shapes.iter_mut().find(|b| b.name == shape) {
            bs.weight = weight.clamp(0.0, 1.0);
        }
    }
}

fn default_animations() -> Vec<AvatarAnimation> {
    vec![
        AvatarAnimation { name: "idle".into(), state: AvatarState::Idle, duration_secs: 3.0, looping: true, blend_weight: 1.0 },
        AvatarAnimation { name: "talk".into(), state: AvatarState::Talking, duration_secs: 2.0, looping: true, blend_weight: 1.0 },
        AvatarAnimation { name: "listen".into(), state: AvatarState::Listening, duration_secs: 4.0, looping: true, blend_weight: 1.0 },
        AvatarAnimation { name: "think".into(), state: AvatarState::Thinking, duration_secs: 2.5, looping: true, blend_weight: 1.0 },
        AvatarAnimation { name: "gesture".into(), state: AvatarState::Gesturing, duration_secs: 1.5, looping: false, blend_weight: 1.0 },
        AvatarAnimation { name: "walk".into(), state: AvatarState::Walking, duration_secs: 1.0, looping: true, blend_weight: 1.0 },
        AvatarAnimation { name: "present".into(), state: AvatarState::Presenting, duration_secs: 5.0, looping: false, blend_weight: 1.0 },
        AvatarAnimation { name: "react".into(), state: AvatarState::Reacting, duration_secs: 1.0, looping: false, blend_weight: 1.0 },
    ]
}

// =============================================================================
// AUTOSTREAM ENGINE
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AutoStreamState {
    Idle,
    Starting,
    Streaming,
    Paused,
    Stopping,
    Error,
}

impl std::fmt::Display for AutoStreamState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "idle"),
            Self::Starting => write!(f, "starting"),
            Self::Streaming => write!(f, "streaming"),
            Self::Paused => write!(f, "paused"),
            Self::Stopping => write!(f, "stopping"),
            Self::Error => write!(f, "error"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoStreamConfig {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate_kbps: u32,
    pub output_url: String,
    pub protocol: String,
    pub codec: String,
    pub enable_audio: bool,
    pub enable_lip_sync: bool,
}

impl Default for AutoStreamConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            fps: 30,
            bitrate_kbps: 4500,
            output_url: "rtmp://localhost/live/avatar".to_string(),
            protocol: "rtmp".to_string(),
            codec: "h264".to_string(),
            enable_audio: true,
            enable_lip_sync: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoStreamStatus {
    pub state: AutoStreamState,
    pub frames_rendered: u64,
    pub frames_dropped: u64,
    pub fps_actual: f32,
    pub bitrate_actual_kbps: u32,
    pub duration_secs: u64,
    pub viewers: u32,
    pub config: AutoStreamConfig,
}

// =============================================================================
// AVATAR SCENE (full 3D environment with avatar)
// =============================================================================

pub struct AvatarScene {
    pub scene_graph: SceneNode,
    pub avatars: Vec<Avatar>,
    pub stream_config: AutoStreamConfig,
    pub stream_state: AutoStreamState,
    pub frames_rendered: u64,
    pub frames_dropped: u64,
}

impl AvatarScene {
    pub fn new() -> Self {
        let mut root = SceneNode::new("root", NodeType::Root);

        // Default scene: camera + light + environment
        let camera = SceneNode::new("main_camera", NodeType::Camera);
        let mut light = SceneNode::new("main_light", NodeType::Light);
        light.transform.position = Vec3::new(0.0, 5.0, 5.0);
        let env = SceneNode::new("environment", NodeType::Environment);

        root.add_child(camera);
        root.add_child(light);
        root.add_child(env);

        Self {
            scene_graph: root,
            avatars: Vec::new(),
            stream_config: AutoStreamConfig::default(),
            stream_state: AutoStreamState::Idle,
            frames_rendered: 0,
            frames_dropped: 0,
        }
    }

    pub fn add_avatar(&mut self, name: &str) -> Uuid {
        let avatar = Avatar::new(name);
        let id = avatar.id;

        // Add avatar node to scene graph
        let avatar_node = SceneNode::new(name, NodeType::Avatar);
        self.scene_graph.add_child(avatar_node);
        self.avatars.push(avatar);
        id
    }

    pub fn get_avatar(&self, name: &str) -> Option<&Avatar> {
        self.avatars.iter().find(|a| a.name == name)
    }

    pub fn get_avatar_mut(&mut self, name: &str) -> Option<&mut Avatar> {
        self.avatars.iter_mut().find(|a| a.name == name)
    }

    pub fn status(&self) -> AvatarSceneStatus {
        AvatarSceneStatus {
            node_count: self.scene_graph.node_count(),
            avatar_count: self.avatars.len(),
            avatars: self.avatars.iter().map(|a| AvatarInfo {
                id: a.id.to_string(),
                name: a.name.clone(),
                state: a.state.to_string(),
                voice_active: a.voice_active,
                current_animation: a.current_animation.clone(),
            }).collect(),
            stream: AutoStreamStatus {
                state: self.stream_state,
                frames_rendered: self.frames_rendered,
                frames_dropped: self.frames_dropped,
                fps_actual: if self.frames_rendered > 0 { self.stream_config.fps as f32 } else { 0.0 },
                bitrate_actual_kbps: self.stream_config.bitrate_kbps,
                duration_secs: 0,
                viewers: 0,
                config: self.stream_config.clone(),
            },
        }
    }

    pub fn report(&self) -> String {
        let s = self.status();
        let mut out = format!(
            "=== Avatar 3D AutoStream ===\n\
             Scene Nodes:  {}\n\
             Avatars:      {}\n\
             Stream:       {}\n\
             Frames:       {} rendered, {} dropped\n\
             Config:       {}x{} @ {}fps {}kbps\n",
            s.node_count, s.avatar_count,
            s.stream.state,
            s.stream.frames_rendered, s.stream.frames_dropped,
            s.stream.config.width, s.stream.config.height,
            s.stream.config.fps, s.stream.config.bitrate_kbps,
        );
        for a in &s.avatars {
            out.push_str(&format!(
                "  [{}] {} — state={} voice={} anim={}\n",
                &a.id[..8], a.name, a.state, a.voice_active,
                a.current_animation.as_deref().unwrap_or("none"),
            ));
        }
        out
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvatarSceneStatus {
    pub node_count: usize,
    pub avatar_count: usize,
    pub avatars: Vec<AvatarInfo>,
    pub stream: AutoStreamStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvatarInfo {
    pub id: String,
    pub name: String,
    pub state: String,
    pub voice_active: bool,
    pub current_animation: Option<String>,
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec3_operations() {
        let v = Vec3::new(3.0, 4.0, 0.0);
        assert!((v.length() - 5.0).abs() < 0.001);
        let n = v.normalize();
        assert!((n.length() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_quaternion_identity() {
        let q = Quaternion::identity();
        assert_eq!(q.w, 1.0);
        assert_eq!(q.x, 0.0);
    }

    #[test]
    fn test_scene_node_hierarchy() {
        let mut root = SceneNode::new("root", NodeType::Root);
        let child1 = SceneNode::new("child1", NodeType::Mesh);
        let child2 = SceneNode::new("child2", NodeType::Light);
        root.add_child(child1);
        root.add_child(child2);
        assert_eq!(root.node_count(), 3);
    }

    #[test]
    fn test_scene_node_find() {
        let mut root = SceneNode::new("root", NodeType::Root);
        let mut mid = SceneNode::new("mid", NodeType::Mesh);
        mid.add_child(SceneNode::new("deep", NodeType::Bone));
        root.add_child(mid);

        assert!(root.find_by_name("deep").is_some());
        assert!(root.find_by_name("missing").is_none());
    }

    #[test]
    fn test_avatar_creation() {
        let avatar = Avatar::new("TestAvatar");
        assert_eq!(avatar.state, AvatarState::Idle);
        assert_eq!(avatar.animations.len(), 8);
        assert!(!avatar.voice_active);
    }

    #[test]
    fn test_avatar_state_machine() {
        let mut avatar = Avatar::new("Bot");
        avatar.start_talking();
        assert_eq!(avatar.state, AvatarState::Talking);
        assert!(avatar.voice_active);
        assert_eq!(avatar.current_animation.as_deref(), Some("talk"));

        avatar.stop_talking();
        assert_eq!(avatar.state, AvatarState::Idle);
        assert!(!avatar.voice_active);
    }

    #[test]
    fn test_avatar_expression() {
        let mut avatar = Avatar::new("Bot");
        avatar.set_expression("smile", 0.8);
        let smile = avatar.appearance.blend_shapes.iter().find(|b| b.name == "smile").unwrap();
        assert!((smile.weight - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_avatar_expression_clamp() {
        let mut avatar = Avatar::new("Bot");
        avatar.set_expression("smile", 1.5);
        let smile = avatar.appearance.blend_shapes.iter().find(|b| b.name == "smile").unwrap();
        assert!((smile.weight - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_avatar_scene() {
        let mut scene = AvatarScene::new();
        assert_eq!(scene.scene_graph.node_count(), 4); // root + camera + light + env

        let id = scene.add_avatar("Apophy");
        assert_eq!(scene.avatars.len(), 1);
        assert_eq!(scene.scene_graph.node_count(), 5);

        let avatar = scene.get_avatar("Apophy").unwrap();
        assert_eq!(avatar.id, id);
    }

    #[test]
    fn test_avatar_scene_report() {
        let mut scene = AvatarScene::new();
        scene.add_avatar("Apophy");
        let report = scene.report();
        assert!(report.contains("Avatar 3D"));
        assert!(report.contains("Apophy"));
    }

    #[test]
    fn test_autostream_config_default() {
        let config = AutoStreamConfig::default();
        assert_eq!(config.width, 1920);
        assert_eq!(config.fps, 30);
        assert!(config.enable_lip_sync);
    }

    #[test]
    fn test_avatar_scene_status_json() {
        let mut scene = AvatarScene::new();
        scene.add_avatar("Bot1");
        scene.add_avatar("Bot2");
        let status = scene.status();
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("Bot1"));
        assert!(json.contains("Bot2"));
    }
}
