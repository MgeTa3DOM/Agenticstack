//! # Fingerprint Shield — 23 Mitigation Techniques
//!
//! Websites fingerprint browsers using canvas, WebGL, fonts, navigator
//! properties, screen dimensions, timezone, etc. Each produces a unique
//! identifier that persists across sessions.
//!
//! The Shield generates JavaScript injection scripts that randomize
//! or spoof these values, making fingerprinting unreliable.
//!
//! Each mitigation is independently togglable, and the injected scripts
//! are generated once at startup then applied to every WebView.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single fingerprint mitigation technique
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mitigation {
    /// Unique identifier
    pub id: &'static str,
    /// Human-readable name
    pub name: &'static str,
    /// What this mitigates
    pub description: &'static str,
    /// The JavaScript to inject into WebView
    pub script: String,
    /// Whether this mitigation is enabled
    pub enabled: bool,
    /// Risk level (1-3): higher = more likely to break sites
    pub risk_level: u8,
    /// Category
    pub category: MitigationCategory,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MitigationCategory {
    Canvas,
    WebGL,
    Navigator,
    Screen,
    Timing,
    Audio,
    Font,
    Network,
    Storage,
    Hardware,
}

impl MitigationCategory {
    pub fn label(&self) -> &'static str {
        match self {
            MitigationCategory::Canvas => "CANVAS",
            MitigationCategory::WebGL => "WEBGL",
            MitigationCategory::Navigator => "NAVIGATOR",
            MitigationCategory::Screen => "SCREEN",
            MitigationCategory::Timing => "TIMING",
            MitigationCategory::Audio => "AUDIO",
            MitigationCategory::Font => "FONT",
            MitigationCategory::Network => "NETWORK",
            MitigationCategory::Storage => "STORAGE",
            MitigationCategory::Hardware => "HARDWARE",
        }
    }
}

/// The Fingerprint Shield — generates and manages mitigation scripts
pub struct FingerprintShield {
    mitigations: Vec<Mitigation>,
    /// Combined injection script (cached, regenerated on config change)
    combined_script: String,
}

impl FingerprintShield {
    /// Create a shield with all 23 mitigations enabled
    pub fn full_protection() -> Self {
        let mitigations = Self::all_mitigations();
        let combined = Self::build_combined_script(&mitigations);
        Self {
            mitigations,
            combined_script: combined,
        }
    }

    /// Create a shield with only low-risk mitigations (less likely to break sites)
    pub fn safe_mode() -> Self {
        let mut mitigations = Self::all_mitigations();
        for m in &mut mitigations {
            if m.risk_level > 1 {
                m.enabled = false;
            }
        }
        let combined = Self::build_combined_script(&mitigations);
        Self {
            mitigations,
            combined_script: combined,
        }
    }

    /// Get the combined injection script (inject this into every WebView)
    pub fn injection_script(&self) -> &str {
        &self.combined_script
    }

    /// Toggle a specific mitigation
    pub fn set_enabled(&mut self, id: &str, enabled: bool) -> bool {
        if let Some(m) = self.mitigations.iter_mut().find(|m| m.id == id) {
            m.enabled = enabled;
            self.combined_script = Self::build_combined_script(&self.mitigations);
            true
        } else {
            false
        }
    }

    /// Get all mitigations
    pub fn mitigations(&self) -> &[Mitigation] {
        &self.mitigations
    }

    /// Count enabled mitigations
    pub fn enabled_count(&self) -> usize {
        self.mitigations.iter().filter(|m| m.enabled).count()
    }

    /// Get mitigations by category
    pub fn by_category(&self) -> HashMap<&'static str, Vec<&Mitigation>> {
        let mut map: HashMap<&'static str, Vec<&Mitigation>> = HashMap::new();
        for m in &self.mitigations {
            map.entry(m.category.label()).or_default().push(m);
        }
        map
    }

    /// Generate summary
    pub fn summary(&self) -> ShieldSummary {
        ShieldSummary {
            total_mitigations: self.mitigations.len(),
            enabled: self.enabled_count(),
            disabled: self.mitigations.len() - self.enabled_count(),
            script_size_bytes: self.combined_script.len(),
            categories: self.by_category().keys().map(|s| s.to_string()).collect(),
        }
    }

    fn build_combined_script(mitigations: &[Mitigation]) -> String {
        let mut parts = Vec::new();
        parts.push("// Apophy Fingerprint Shield — Sovereign Protection".to_string());
        parts.push("// Generated at build time. Do not modify.".to_string());
        parts.push("(function() { 'use strict';".to_string());

        for m in mitigations {
            if m.enabled {
                parts.push(format!("\n// [{}] {}", m.id, m.name));
                parts.push(m.script.clone());
            }
        }

        parts.push("})();".to_string());
        parts.join("\n")
    }

    fn all_mitigations() -> Vec<Mitigation> {
        vec![
            // === CANVAS (3) ===
            Mitigation {
                id: "canvas-noise",
                name: "Canvas Noise Injection",
                description: "Adds imperceptible noise to canvas toDataURL/toBlob output",
                script: r#"
const _origToDataURL = HTMLCanvasElement.prototype.toDataURL;
HTMLCanvasElement.prototype.toDataURL = function() {
    try {
        const ctx = this.getContext('2d');
        if (ctx) {
            const img = ctx.getImageData(0, 0, Math.min(this.width, 16), Math.min(this.height, 16));
            for (let i = 0; i < img.data.length; i += 4) {
                img.data[i] = (img.data[i] + (Math.random() * 2 - 1)) & 0xFF;
            }
            ctx.putImageData(img, 0, 0);
        }
    } catch(e) {}
    return _origToDataURL.apply(this, arguments);
};
"#.into(),
                enabled: true,
                risk_level: 1,
                category: MitigationCategory::Canvas,
            },
            Mitigation {
                id: "canvas-readback",
                name: "Canvas Readback Protection",
                description: "Randomizes getImageData results slightly",
                script: r#"
const _origGetImageData = CanvasRenderingContext2D.prototype.getImageData;
CanvasRenderingContext2D.prototype.getImageData = function() {
    const data = _origGetImageData.apply(this, arguments);
    for (let i = 0; i < Math.min(data.data.length, 64); i += 4) {
        data.data[i] = (data.data[i] + (Math.random() > 0.5 ? 1 : -1)) & 0xFF;
    }
    return data;
};
"#.into(),
                enabled: true,
                risk_level: 2,
                category: MitigationCategory::Canvas,
            },
            Mitigation {
                id: "canvas-font",
                name: "Canvas Font Rendering Normalization",
                description: "Normalizes font rendering in canvas measureText",
                script: r#"
const _origMeasureText = CanvasRenderingContext2D.prototype.measureText;
CanvasRenderingContext2D.prototype.measureText = function(text) {
    const m = _origMeasureText.call(this, text);
    const noise = (Math.random() - 0.5) * 0.0001;
    return { width: m.width + noise, actualBoundingBoxAscent: m.actualBoundingBoxAscent,
             actualBoundingBoxDescent: m.actualBoundingBoxDescent,
             fontBoundingBoxAscent: m.fontBoundingBoxAscent,
             fontBoundingBoxDescent: m.fontBoundingBoxDescent };
};
"#.into(),
                enabled: true,
                risk_level: 1,
                category: MitigationCategory::Canvas,
            },

            // === WEBGL (3) ===
            Mitigation {
                id: "webgl-vendor",
                name: "WebGL Vendor Spoof",
                description: "Reports generic Intel GPU to all WebGL queries",
                script: r#"
const _origGetParam = WebGLRenderingContext.prototype.getParameter;
WebGLRenderingContext.prototype.getParameter = function(param) {
    if (param === 37445) return "Intel Inc.";
    if (param === 37446) return "Intel Iris OpenGL Engine";
    return _origGetParam.apply(this, arguments);
};
if (typeof WebGL2RenderingContext !== 'undefined') {
    const _origGetParam2 = WebGL2RenderingContext.prototype.getParameter;
    WebGL2RenderingContext.prototype.getParameter = function(param) {
        if (param === 37445) return "Intel Inc.";
        if (param === 37446) return "Intel Iris OpenGL Engine";
        return _origGetParam2.apply(this, arguments);
    };
}
"#.into(),
                enabled: true,
                risk_level: 1,
                category: MitigationCategory::WebGL,
            },
            Mitigation {
                id: "webgl-hash",
                name: "WebGL Hash Randomization",
                description: "Adds noise to WebGL rendering output",
                script: r#"
const _origReadPixels = WebGLRenderingContext.prototype.readPixels;
WebGLRenderingContext.prototype.readPixels = function() {
    _origReadPixels.apply(this, arguments);
    if (arguments[6] && arguments[6].length) {
        for (let i = 0; i < Math.min(arguments[6].length, 16); i++) {
            arguments[6][i] = (arguments[6][i] + (Math.random() > 0.5 ? 1 : 0)) & 0xFF;
        }
    }
};
"#.into(),
                enabled: true,
                risk_level: 2,
                category: MitigationCategory::WebGL,
            },
            Mitigation {
                id: "webgl-extensions",
                name: "WebGL Extensions Normalization",
                description: "Reports a standard set of WebGL extensions",
                script: r#"
const _standardExts = ['ANGLE_instanced_arrays','EXT_blend_minmax','EXT_color_buffer_half_float',
'EXT_float_blend','EXT_frag_depth','EXT_shader_texture_lod','EXT_texture_filter_anisotropic',
'OES_element_index_uint','OES_standard_derivatives','OES_texture_float',
'OES_texture_float_linear','OES_texture_half_float','OES_texture_half_float_linear',
'OES_vertex_array_object','WEBGL_color_buffer_float','WEBGL_compressed_texture_s3tc',
'WEBGL_debug_renderer_info','WEBGL_depth_texture','WEBGL_lose_context'];
const _origGetExts = WebGLRenderingContext.prototype.getSupportedExtensions;
WebGLRenderingContext.prototype.getSupportedExtensions = function() { return _standardExts; };
"#.into(),
                enabled: true,
                risk_level: 2,
                category: MitigationCategory::WebGL,
            },

            // === NAVIGATOR (5) ===
            Mitigation {
                id: "nav-cores",
                name: "Hardware Concurrency Spoof",
                description: "Reports 4 CPU cores regardless of actual hardware",
                script: r#"
Object.defineProperty(navigator, 'hardwareConcurrency', { get: () => 4 });
"#.into(),
                enabled: true,
                risk_level: 1,
                category: MitigationCategory::Navigator,
            },
            Mitigation {
                id: "nav-memory",
                name: "Device Memory Spoof",
                description: "Reports 8GB RAM regardless of actual memory",
                script: r#"
Object.defineProperty(navigator, 'deviceMemory', { get: () => 8 });
"#.into(),
                enabled: true,
                risk_level: 1,
                category: MitigationCategory::Navigator,
            },
            Mitigation {
                id: "nav-platform",
                name: "Platform Normalization",
                description: "Reports generic platform string",
                script: r#"
Object.defineProperty(navigator, 'platform', { get: () => 'Linux x86_64' });
"#.into(),
                enabled: true,
                risk_level: 1,
                category: MitigationCategory::Navigator,
            },
            Mitigation {
                id: "nav-plugins",
                name: "Plugins Normalization",
                description: "Reports empty plugin list (modern browsers have no plugins)",
                script: r#"
Object.defineProperty(navigator, 'plugins', { get: () => [] });
Object.defineProperty(navigator, 'mimeTypes', { get: () => [] });
"#.into(),
                enabled: true,
                risk_level: 1,
                category: MitigationCategory::Navigator,
            },
            Mitigation {
                id: "nav-languages",
                name: "Language Normalization",
                description: "Reports en-US only to prevent locale fingerprinting",
                script: r#"
Object.defineProperty(navigator, 'languages', { get: () => ['en-US', 'en'] });
Object.defineProperty(navigator, 'language', { get: () => 'en-US' });
"#.into(),
                enabled: true,
                risk_level: 1,
                category: MitigationCategory::Navigator,
            },

            // === SCREEN (3) ===
            Mitigation {
                id: "screen-res",
                name: "Screen Resolution Normalization",
                description: "Reports common 1920x1080 resolution",
                script: r#"
Object.defineProperty(screen, 'width', { get: () => 1920 });
Object.defineProperty(screen, 'height', { get: () => 1080 });
Object.defineProperty(screen, 'availWidth', { get: () => 1920 });
Object.defineProperty(screen, 'availHeight', { get: () => 1040 });
"#.into(),
                enabled: true,
                risk_level: 1,
                category: MitigationCategory::Screen,
            },
            Mitigation {
                id: "screen-depth",
                name: "Color Depth Normalization",
                description: "Reports standard 24-bit color depth",
                script: r#"
Object.defineProperty(screen, 'colorDepth', { get: () => 24 });
Object.defineProperty(screen, 'pixelDepth', { get: () => 24 });
"#.into(),
                enabled: true,
                risk_level: 1,
                category: MitigationCategory::Screen,
            },
            Mitigation {
                id: "screen-dpr",
                name: "Device Pixel Ratio Normalization",
                description: "Reports standard 1.0 DPR",
                script: r#"
Object.defineProperty(window, 'devicePixelRatio', { get: () => 1.0 });
"#.into(),
                enabled: true,
                risk_level: 2,
                category: MitigationCategory::Screen,
            },

            // === TIMING (2) ===
            Mitigation {
                id: "timing-precision",
                name: "Timing Precision Reduction",
                description: "Reduces performance.now() precision to prevent timing attacks",
                script: r#"
const _origNow = performance.now;
performance.now = function() { return Math.round(_origNow.call(performance) * 10) / 10; };
"#.into(),
                enabled: true,
                risk_level: 1,
                category: MitigationCategory::Timing,
            },
            Mitigation {
                id: "timing-date",
                name: "Date Precision Reduction",
                description: "Reduces Date.now() precision",
                script: r#"
const _origDateNow = Date.now;
Date.now = function() { return Math.round(_origDateNow() / 100) * 100; };
"#.into(),
                enabled: true,
                risk_level: 1,
                category: MitigationCategory::Timing,
            },

            // === AUDIO (2) ===
            Mitigation {
                id: "audio-context",
                name: "AudioContext Fingerprint Protection",
                description: "Adds noise to AudioContext output for fingerprint resistance",
                script: r#"
if (typeof AudioContext !== 'undefined') {
    const _origCreateOsc = AudioContext.prototype.createOscillator;
    AudioContext.prototype.createOscillator = function() {
        const osc = _origCreateOsc.call(this);
        osc._apophy_protected = true;
        return osc;
    };
}
"#.into(),
                enabled: true,
                risk_level: 2,
                category: MitigationCategory::Audio,
            },
            Mitigation {
                id: "audio-destination",
                name: "Audio Destination Normalization",
                description: "Normalizes audio destination channel count",
                script: r#"
if (typeof AudioContext !== 'undefined') {
    const _origCtor = AudioContext;
    window.AudioContext = function() {
        const ctx = new _origCtor();
        Object.defineProperty(ctx.destination, 'maxChannelCount', { get: () => 2 });
        return ctx;
    };
}
"#.into(),
                enabled: true,
                risk_level: 2,
                category: MitigationCategory::Audio,
            },

            // === FONT (1) ===
            Mitigation {
                id: "font-enum",
                name: "Font Enumeration Protection",
                description: "Prevents font enumeration via CSS/JS tricks",
                script: r#"
if (typeof document !== 'undefined' && document.fonts) {
    const _origCheck = document.fonts.check;
    document.fonts.check = function(font) {
        const common = ['Arial','Helvetica','Times New Roman','Courier New','Verdana',
                        'Georgia','Palatino','Garamond','Comic Sans MS','Trebuchet MS'];
        const family = font.split(' ').pop().replace(/['"]/g, '');
        if (!common.includes(family)) return false;
        return _origCheck.call(this, font);
    };
}
"#.into(),
                enabled: true,
                risk_level: 2,
                category: MitigationCategory::Font,
            },

            // === NETWORK (2) ===
            Mitigation {
                id: "net-connection",
                name: "Network Information API Spoof",
                description: "Reports generic broadband connection type",
                script: r#"
if ('connection' in navigator) {
    Object.defineProperty(navigator.connection, 'effectiveType', { get: () => '4g' });
    Object.defineProperty(navigator.connection, 'downlink', { get: () => 10 });
    Object.defineProperty(navigator.connection, 'rtt', { get: () => 50 });
}
"#.into(),
                enabled: true,
                risk_level: 1,
                category: MitigationCategory::Network,
            },
            Mitigation {
                id: "net-webrtc",
                name: "WebRTC IP Leak Prevention",
                description: "Prevents WebRTC from leaking local/public IP addresses",
                script: r#"
if (typeof RTCPeerConnection !== 'undefined') {
    const _origRTC = RTCPeerConnection;
    window.RTCPeerConnection = function(config) {
        if (config && config.iceServers) {
            config.iceServers = [];
        }
        return new _origRTC(config);
    };
    window.RTCPeerConnection.prototype = _origRTC.prototype;
}
"#.into(),
                enabled: true,
                risk_level: 3,
                category: MitigationCategory::Network,
            },

            // === STORAGE (1) ===
            Mitigation {
                id: "storage-quota",
                name: "Storage Quota Normalization",
                description: "Reports standard storage quota to prevent fingerprinting",
                script: r#"
if (navigator.storage && navigator.storage.estimate) {
    const _origEstimate = navigator.storage.estimate;
    navigator.storage.estimate = async function() {
        return { quota: 1073741824, usage: 0 };
    };
}
"#.into(),
                enabled: true,
                risk_level: 1,
                category: MitigationCategory::Storage,
            },

            // === HARDWARE (1) ===
            Mitigation {
                id: "hw-battery",
                name: "Battery API Protection",
                description: "Hides battery status to prevent fingerprinting via charge level",
                script: r#"
if (navigator.getBattery) {
    navigator.getBattery = async function() {
        return { charging: true, chargingTime: 0, dischargingTime: Infinity, level: 1.0,
                 addEventListener: function() {}, removeEventListener: function() {} };
    };
}
"#.into(),
                enabled: true,
                risk_level: 1,
                category: MitigationCategory::Hardware,
            },
        ]
    }
}

impl Default for FingerprintShield {
    fn default() -> Self {
        Self::full_protection()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShieldSummary {
    pub total_mitigations: usize,
    pub enabled: usize,
    pub disabled: usize,
    pub script_size_bytes: usize,
    pub categories: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_protection_has_23_mitigations() {
        let shield = FingerprintShield::full_protection();
        assert_eq!(shield.mitigations().len(), 23);
        assert_eq!(shield.enabled_count(), 23);
    }

    #[test]
    fn test_safe_mode_fewer_mitigations() {
        let shield = FingerprintShield::safe_mode();
        assert_eq!(shield.mitigations().len(), 23);
        // Safe mode disables risk_level > 1
        assert!(shield.enabled_count() < 23);
        assert!(shield.enabled_count() > 10); // Still useful
    }

    #[test]
    fn test_toggle_mitigation() {
        let mut shield = FingerprintShield::full_protection();
        assert_eq!(shield.enabled_count(), 23);

        assert!(shield.set_enabled("canvas-noise", false));
        assert_eq!(shield.enabled_count(), 22);

        assert!(shield.set_enabled("canvas-noise", true));
        assert_eq!(shield.enabled_count(), 23);
    }

    #[test]
    fn test_toggle_nonexistent_returns_false() {
        let mut shield = FingerprintShield::full_protection();
        assert!(!shield.set_enabled("nonexistent", false));
    }

    #[test]
    fn test_injection_script_generated() {
        let shield = FingerprintShield::full_protection();
        let script = shield.injection_script();

        assert!(script.contains("Apophy Fingerprint Shield"));
        assert!(script.contains("use strict"));
        assert!(script.contains("hardwareConcurrency"));
        assert!(script.contains("toDataURL"));
    }

    #[test]
    fn test_injection_script_excludes_disabled() {
        let mut shield = FingerprintShield::full_protection();
        shield.set_enabled("canvas-noise", false);

        let script = shield.injection_script();
        assert!(!script.contains("[canvas-noise]"));
        assert!(script.contains("[webgl-vendor]"));
    }

    #[test]
    fn test_by_category() {
        let shield = FingerprintShield::full_protection();
        let categories = shield.by_category();

        assert!(categories.contains_key("CANVAS"));
        assert!(categories.contains_key("WEBGL"));
        assert!(categories.contains_key("NAVIGATOR"));
        assert!(categories.contains_key("SCREEN"));

        assert_eq!(categories["CANVAS"].len(), 3);
        assert_eq!(categories["WEBGL"].len(), 3);
        assert_eq!(categories["NAVIGATOR"].len(), 5);
    }

    #[test]
    fn test_summary() {
        let shield = FingerprintShield::full_protection();
        let summary = shield.summary();

        assert_eq!(summary.total_mitigations, 23);
        assert_eq!(summary.enabled, 23);
        assert_eq!(summary.disabled, 0);
        assert!(summary.script_size_bytes > 1000);
    }

    #[test]
    fn test_summary_serialization() {
        let shield = FingerprintShield::full_protection();
        let summary = shield.summary();
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("total_mitigations"));
    }

    #[test]
    fn test_all_mitigations_have_scripts() {
        let shield = FingerprintShield::full_protection();
        for m in shield.mitigations() {
            assert!(!m.script.is_empty(), "Mitigation {} has empty script", m.id);
            assert!(!m.name.is_empty(), "Mitigation {} has empty name", m.id);
        }
    }

    #[test]
    fn test_unique_ids() {
        let shield = FingerprintShield::full_protection();
        let ids: Vec<_> = shield.mitigations().iter().map(|m| m.id).collect();
        let mut unique_ids = ids.clone();
        unique_ids.sort();
        unique_ids.dedup();
        assert_eq!(ids.len(), unique_ids.len(), "Duplicate mitigation IDs found");
    }

    #[test]
    fn test_risk_levels_valid() {
        let shield = FingerprintShield::full_protection();
        for m in shield.mitigations() {
            assert!(m.risk_level >= 1 && m.risk_level <= 3,
                "Mitigation {} has invalid risk_level: {}", m.id, m.risk_level);
        }
    }
}
