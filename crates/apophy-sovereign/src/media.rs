//! # Media Pipeline — FFmpeg-based native media processing
//!
//! Sovereign media pipeline for transcoding, streaming, and recording.
//! Uses FFmpeg as a subprocess — no cloud transcoding services.
//!
//! ```text
//! ┌──────────┐   ┌───────────┐   ┌──────────┐   ┌──────────┐
//! │  Input   │──▶│ Transcode │──▶│  Stream  │──▶│  Record  │
//! │ (file/   │   │ (FFmpeg)  │   │ (RTMP/   │   │ (segment │
//! │  stream) │   │           │   │  HLS)    │   │  archive)│
//! └──────────┘   └───────────┘   └──────────┘   └──────────┘
//! ```

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

// =============================================================================
// MEDIA PIPELINE TYPES
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MediaFormat {
    H264,
    H265,
    VP9,
    AV1,
    AAC,
    Opus,
    WebM,
    MP4,
    HLS,
    RTMP,
}

impl std::fmt::Display for MediaFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::H264 => write!(f, "h264"),
            Self::H265 => write!(f, "h265"),
            Self::VP9 => write!(f, "vp9"),
            Self::AV1 => write!(f, "av1"),
            Self::AAC => write!(f, "aac"),
            Self::Opus => write!(f, "opus"),
            Self::WebM => write!(f, "webm"),
            Self::MP4 => write!(f, "mp4"),
            Self::HLS => write!(f, "hls"),
            Self::RTMP => write!(f, "rtmp"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaProfile {
    pub video_codec: MediaFormat,
    pub audio_codec: MediaFormat,
    pub container: MediaFormat,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate_kbps: u32,
    pub audio_bitrate_kbps: u32,
}

impl MediaProfile {
    pub fn hd_stream() -> Self {
        Self {
            video_codec: MediaFormat::H264,
            audio_codec: MediaFormat::AAC,
            container: MediaFormat::MP4,
            width: 1920,
            height: 1080,
            fps: 30,
            bitrate_kbps: 4500,
            audio_bitrate_kbps: 128,
        }
    }

    pub fn sd_stream() -> Self {
        Self {
            video_codec: MediaFormat::H264,
            audio_codec: MediaFormat::AAC,
            container: MediaFormat::MP4,
            width: 1280,
            height: 720,
            fps: 30,
            bitrate_kbps: 2500,
            audio_bitrate_kbps: 96,
        }
    }

    pub fn hls_adaptive() -> Self {
        Self {
            video_codec: MediaFormat::H264,
            audio_codec: MediaFormat::AAC,
            container: MediaFormat::HLS,
            width: 1920,
            height: 1080,
            fps: 30,
            bitrate_kbps: 6000,
            audio_bitrate_kbps: 128,
        }
    }

    pub fn webm_vp9() -> Self {
        Self {
            video_codec: MediaFormat::VP9,
            audio_codec: MediaFormat::Opus,
            container: MediaFormat::WebM,
            width: 1920,
            height: 1080,
            fps: 30,
            bitrate_kbps: 3000,
            audio_bitrate_kbps: 96,
        }
    }
}

// =============================================================================
// TRANSCODE JOB
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Queued,
    Running,
    Complete,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscodeJob {
    pub id: Uuid,
    pub input: String,
    pub output: String,
    pub profile: MediaProfile,
    pub status: JobStatus,
    pub progress_pct: f32,
    pub ffmpeg_args: Vec<String>,
    pub error: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl TranscodeJob {
    pub fn new(input: &str, output: &str, profile: MediaProfile) -> Self {
        let ffmpeg_args = build_ffmpeg_args(input, output, &profile);
        Self {
            id: Uuid::new_v4(),
            input: input.to_string(),
            output: output.to_string(),
            profile,
            status: JobStatus::Queued,
            progress_pct: 0.0,
            ffmpeg_args,
            error: None,
            created_at: chrono::Utc::now(),
            finished_at: None,
        }
    }
}

fn build_ffmpeg_args(input: &str, output: &str, profile: &MediaProfile) -> Vec<String> {
    let vcodec = match profile.video_codec {
        MediaFormat::H264 => "libx264",
        MediaFormat::H265 => "libx265",
        MediaFormat::VP9 => "libvpx-vp9",
        MediaFormat::AV1 => "libaom-av1",
        _ => "copy",
    };
    let acodec = match profile.audio_codec {
        MediaFormat::AAC => "aac",
        MediaFormat::Opus => "libopus",
        _ => "copy",
    };
    vec![
        "-i".into(), input.into(),
        "-c:v".into(), vcodec.into(),
        "-c:a".into(), acodec.into(),
        "-b:v".into(), format!("{}k", profile.bitrate_kbps),
        "-b:a".into(), format!("{}k", profile.audio_bitrate_kbps),
        "-s".into(), format!("{}x{}", profile.width, profile.height),
        "-r".into(), profile.fps.to_string(),
        "-y".into(),
        output.into(),
    ]
}

// =============================================================================
// STREAM SESSION
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamProtocol {
    RTMP,
    HLS,
    WebRTC,
    SRT,
}

impl std::fmt::Display for StreamProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RTMP => write!(f, "RTMP"),
            Self::HLS => write!(f, "HLS"),
            Self::WebRTC => write!(f, "WebRTC"),
            Self::SRT => write!(f, "SRT"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamSession {
    pub id: Uuid,
    pub name: String,
    pub protocol: StreamProtocol,
    pub source: String,
    pub output_url: String,
    pub profile: MediaProfile,
    pub status: JobStatus,
    pub viewers: u32,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub duration_secs: u64,
}

impl StreamSession {
    pub fn new_rtmp(name: &str, source: &str, rtmp_url: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            protocol: StreamProtocol::RTMP,
            source: source.to_string(),
            output_url: rtmp_url.to_string(),
            profile: MediaProfile::hd_stream(),
            status: JobStatus::Queued,
            viewers: 0,
            started_at: None,
            duration_secs: 0,
        }
    }

    pub fn new_hls(name: &str, source: &str, hls_dir: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            protocol: StreamProtocol::HLS,
            source: source.to_string(),
            output_url: hls_dir.to_string(),
            profile: MediaProfile::hls_adaptive(),
            status: JobStatus::Queued,
            viewers: 0,
            started_at: None,
            duration_secs: 0,
        }
    }

    pub fn ffmpeg_stream_args(&self) -> Vec<String> {
        match self.protocol {
            StreamProtocol::RTMP => vec![
                "-i".into(), self.source.clone(),
                "-c:v".into(), "libx264".into(),
                "-preset".into(), "veryfast".into(),
                "-tune".into(), "zerolatency".into(),
                "-c:a".into(), "aac".into(),
                "-b:v".into(), format!("{}k", self.profile.bitrate_kbps),
                "-b:a".into(), format!("{}k", self.profile.audio_bitrate_kbps),
                "-f".into(), "flv".into(),
                self.output_url.clone(),
            ],
            StreamProtocol::HLS => vec![
                "-i".into(), self.source.clone(),
                "-c:v".into(), "libx264".into(),
                "-preset".into(), "veryfast".into(),
                "-c:a".into(), "aac".into(),
                "-f".into(), "hls".into(),
                "-hls_time".into(), "4".into(),
                "-hls_list_size".into(), "10".into(),
                "-hls_flags".into(), "delete_segments".into(),
                format!("{}/stream.m3u8", self.output_url),
            ],
            StreamProtocol::SRT => vec![
                "-i".into(), self.source.clone(),
                "-c:v".into(), "libx264".into(),
                "-preset".into(), "veryfast".into(),
                "-c:a".into(), "aac".into(),
                "-f".into(), "mpegts".into(),
                self.output_url.clone(),
            ],
            StreamProtocol::WebRTC => vec![
                "-i".into(), self.source.clone(),
                "-c:v".into(), "libvpx-vp9".into(),
                "-c:a".into(), "libopus".into(),
                "-f".into(), "webm".into(),
                self.output_url.clone(),
            ],
        }
    }
}

// =============================================================================
// RECORDING
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recording {
    pub id: Uuid,
    pub name: String,
    pub source: String,
    pub output_dir: PathBuf,
    pub segment_duration_secs: u32,
    pub profile: MediaProfile,
    pub status: JobStatus,
    pub segments: Vec<RecordingSegment>,
    pub total_duration_secs: u64,
    pub total_size_bytes: u64,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingSegment {
    pub index: u32,
    pub path: PathBuf,
    pub duration_secs: f64,
    pub size_bytes: u64,
}

impl Recording {
    pub fn new(name: &str, source: &str, output_dir: PathBuf, segment_secs: u32) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            source: source.to_string(),
            output_dir,
            segment_duration_secs: segment_secs,
            profile: MediaProfile::hd_stream(),
            status: JobStatus::Queued,
            segments: Vec::new(),
            total_duration_secs: 0,
            total_size_bytes: 0,
            started_at: None,
        }
    }

    pub fn ffmpeg_record_args(&self) -> Vec<String> {
        let output_pattern = self.output_dir.join(format!("{}_seg%03d.mp4", self.name));
        vec![
            "-i".into(), self.source.clone(),
            "-c:v".into(), "libx264".into(),
            "-preset".into(), "fast".into(),
            "-c:a".into(), "aac".into(),
            "-b:v".into(), format!("{}k", self.profile.bitrate_kbps),
            "-f".into(), "segment".into(),
            "-segment_time".into(), self.segment_duration_secs.to_string(),
            "-reset_timestamps".into(), "1".into(),
            output_pattern.to_string_lossy().into_owned(),
        ]
    }
}

// =============================================================================
// MEDIA PIPELINE ENGINE
// =============================================================================

pub struct MediaPipeline {
    pub ffmpeg_path: String,
    pub ffprobe_path: String,
    pub jobs: Vec<TranscodeJob>,
    pub streams: Vec<StreamSession>,
    pub recordings: Vec<Recording>,
    pub available: bool,
}

impl MediaPipeline {
    pub fn new() -> Self {
        let available = check_ffmpeg_available();
        Self {
            ffmpeg_path: "ffmpeg".to_string(),
            ffprobe_path: "ffprobe".to_string(),
            jobs: Vec::new(),
            streams: Vec::new(),
            recordings: Vec::new(),
            available,
        }
    }

    pub fn queue_transcode(&mut self, input: &str, output: &str, profile: MediaProfile) -> Uuid {
        let job = TranscodeJob::new(input, output, profile);
        let id = job.id;
        self.jobs.push(job);
        id
    }

    pub fn start_stream(&mut self, session: StreamSession) -> Uuid {
        let id = session.id;
        self.streams.push(session);
        id
    }

    pub fn start_recording(&mut self, recording: Recording) -> Uuid {
        let id = recording.id;
        self.recordings.push(recording);
        id
    }

    pub fn status(&self) -> MediaPipelineStatus {
        MediaPipelineStatus {
            ffmpeg_available: self.available,
            ffmpeg_path: self.ffmpeg_path.clone(),
            total_jobs: self.jobs.len(),
            active_jobs: self.jobs.iter().filter(|j| j.status == JobStatus::Running).count(),
            completed_jobs: self.jobs.iter().filter(|j| j.status == JobStatus::Complete).count(),
            failed_jobs: self.jobs.iter().filter(|j| j.status == JobStatus::Failed).count(),
            active_streams: self.streams.iter().filter(|s| s.status == JobStatus::Running).count(),
            active_recordings: self.recordings.iter().filter(|r| r.status == JobStatus::Running).count(),
            supported_codecs: vec![
                "h264".into(), "h265".into(), "vp9".into(), "av1".into(),
                "aac".into(), "opus".into(),
            ],
            supported_protocols: vec![
                "rtmp".into(), "hls".into(), "webrtc".into(), "srt".into(),
            ],
        }
    }

    pub fn report(&self) -> String {
        let s = self.status();
        format!(
            "=== Media Pipeline (FFmpeg) ===\n\
             FFmpeg:       {}\n\
             Path:         {}\n\
             Jobs:         {} total ({} active, {} done, {} failed)\n\
             Streams:      {} active\n\
             Recordings:   {} active\n\
             Codecs:       {}\n\
             Protocols:    {}",
            if s.ffmpeg_available { "AVAILABLE" } else { "NOT FOUND" },
            s.ffmpeg_path,
            s.total_jobs, s.active_jobs, s.completed_jobs, s.failed_jobs,
            s.active_streams,
            s.active_recordings,
            s.supported_codecs.join(", "),
            s.supported_protocols.join(", "),
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaPipelineStatus {
    pub ffmpeg_available: bool,
    pub ffmpeg_path: String,
    pub total_jobs: usize,
    pub active_jobs: usize,
    pub completed_jobs: usize,
    pub failed_jobs: usize,
    pub active_streams: usize,
    pub active_recordings: usize,
    pub supported_codecs: Vec<String>,
    pub supported_protocols: Vec<String>,
}

fn check_ffmpeg_available() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
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
    fn test_media_profile_hd() {
        let p = MediaProfile::hd_stream();
        assert_eq!(p.width, 1920);
        assert_eq!(p.height, 1080);
        assert_eq!(p.fps, 30);
    }

    #[test]
    fn test_transcode_job_creation() {
        let job = TranscodeJob::new("input.mp4", "output.mp4", MediaProfile::hd_stream());
        assert_eq!(job.status, JobStatus::Queued);
        assert!(!job.ffmpeg_args.is_empty());
        assert!(job.ffmpeg_args.contains(&"-c:v".to_string()));
        assert!(job.ffmpeg_args.contains(&"libx264".to_string()));
    }

    #[test]
    fn test_stream_session_rtmp() {
        let s = StreamSession::new_rtmp("test", "/dev/video0", "rtmp://localhost/live/stream");
        assert_eq!(s.protocol, StreamProtocol::RTMP);
        let args = s.ffmpeg_stream_args();
        assert!(args.contains(&"flv".to_string()));
    }

    #[test]
    fn test_stream_session_hls() {
        let s = StreamSession::new_hls("test", "/dev/video0", "/tmp/hls");
        assert_eq!(s.protocol, StreamProtocol::HLS);
        let args = s.ffmpeg_stream_args();
        assert!(args.contains(&"hls".to_string()));
    }

    #[test]
    fn test_recording_creation() {
        let r = Recording::new("session1", "/dev/video0", PathBuf::from("/tmp/rec"), 30);
        assert_eq!(r.segment_duration_secs, 30);
        let args = r.ffmpeg_record_args();
        assert!(args.contains(&"segment".to_string()));
    }

    #[test]
    fn test_pipeline_engine() {
        let mut pipeline = MediaPipeline::new();
        let id = pipeline.queue_transcode("in.mp4", "out.mp4", MediaProfile::sd_stream());
        assert_eq!(pipeline.jobs.len(), 1);
        assert_eq!(pipeline.jobs[0].id, id);

        let status = pipeline.status();
        assert_eq!(status.total_jobs, 1);
        assert_eq!(status.active_jobs, 0);
    }

    #[test]
    fn test_pipeline_report() {
        let pipeline = MediaPipeline::new();
        let report = pipeline.report();
        assert!(report.contains("Media Pipeline"));
        assert!(report.contains("h264"));
    }

    #[test]
    fn test_ffmpeg_args_vp9() {
        let args = build_ffmpeg_args("in.webm", "out.webm", &MediaProfile::webm_vp9());
        assert!(args.contains(&"libvpx-vp9".to_string()));
        assert!(args.contains(&"libopus".to_string()));
    }

    #[test]
    fn test_multiple_profiles() {
        let profiles = vec![
            MediaProfile::hd_stream(),
            MediaProfile::sd_stream(),
            MediaProfile::hls_adaptive(),
            MediaProfile::webm_vp9(),
        ];
        assert_eq!(profiles.len(), 4);
        assert_eq!(profiles[0].width, 1920);
        assert_eq!(profiles[1].width, 1280);
    }
}
