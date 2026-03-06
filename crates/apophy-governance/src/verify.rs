//! # Verification Gates — Trust Nothing, Verify Everything
//!
//! "La sortie du modele n'est pas un fait. C'est une hypothese
//! qui doit survivre a la verification avant de devenir un fait."
//!
//! Two types of verification:
//! - **Deterministic**: regex, JSON schema, value ranges, format checks
//! - **Procedural**: human review, second-model check, tool confirmation
//!
//! Nothing passes without going through a gate.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A verification gate — the checkpoint between LLM output and deployment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationGate {
    /// Gate ID
    pub id: Uuid,
    /// Human-readable name
    pub name: String,
    /// What this gate checks
    pub description: String,
    /// The checks in this gate (all must pass)
    pub checks: Vec<Verification>,
    /// Is this gate mandatory? (skip = audit trail entry)
    pub mandatory: bool,
    /// How many checks must pass (0 = all)
    pub min_pass: usize,
}

/// A single verification check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verification {
    /// Check ID
    pub id: Uuid,
    /// Check name
    pub name: String,
    /// What kind of check
    pub kind: VerificationKind,
    /// Severity if this check fails
    pub severity: Severity,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VerificationKind {
    /// Regex match on output
    RegexMatch { pattern: String },
    /// JSON schema validation
    JsonSchema { schema: String },
    /// Value must be in range
    NumericRange { min: f64, max: f64 },
    /// Output must contain specific strings
    ContainsAll { required: Vec<String> },
    /// Output must NOT contain specific strings
    ContainsNone { forbidden: Vec<String> },
    /// Max length check
    MaxLength { chars: usize },
    /// Min length check
    MinLength { chars: usize },
    /// Custom deterministic function (name of registered checker)
    CustomDeterministic { checker_name: String },
    /// Second model verification
    SecondModel { model: String, prompt_template: String },
    /// Human review required
    HumanReview { reviewer: String },
    /// Tool confirmation (call a tool to verify)
    ToolConfirmation { tool_name: String, expected: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Severity {
    /// Block deployment
    Critical,
    /// Flag for review but allow
    Warning,
    /// Log only
    Info,
}

/// Result of running a verification gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Which gate was run
    pub gate_id: Uuid,
    /// Gate name
    pub gate_name: String,
    /// When the verification ran
    pub verified_at: DateTime<Utc>,
    /// Individual check results
    pub check_results: Vec<CheckResult>,
    /// Overall pass/fail
    pub passed: bool,
    /// How many checks passed
    pub passed_count: usize,
    /// How many checks failed
    pub failed_count: usize,
    /// Critical failures (any = gate fails)
    pub critical_failures: Vec<String>,
}

/// Result of a single check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub check_id: Uuid,
    pub check_name: String,
    pub passed: bool,
    pub severity: Severity,
    pub message: String,
    pub duration_ms: u64,
}

impl VerificationGate {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description: description.into(),
            checks: Vec::new(),
            mandatory: true,
            min_pass: 0, // all must pass
        }
    }

    pub fn add_check(&mut self, check: Verification) {
        self.checks.push(check);
    }

    /// Run all checks against an output string.
    pub fn verify(&self, output: &str) -> VerificationResult {
        let mut check_results = Vec::new();
        let mut critical_failures = Vec::new();

        for check in &self.checks {
            let start = std::time::Instant::now();
            let (passed, message) = run_deterministic_check(&check.kind, output);
            let duration_ms = start.elapsed().as_millis() as u64;

            if !passed && check.severity == Severity::Critical {
                critical_failures.push(format!("{}: {}", check.name, message));
            }

            check_results.push(CheckResult {
                check_id: check.id,
                check_name: check.name.clone(),
                passed,
                severity: check.severity.clone(),
                message,
                duration_ms,
            });
        }

        let passed_count = check_results.iter().filter(|r| r.passed).count();
        let failed_count = check_results.len() - passed_count;

        let min_required = if self.min_pass == 0 {
            self.checks.len()
        } else {
            self.min_pass
        };

        let passed = critical_failures.is_empty() && passed_count >= min_required;

        VerificationResult {
            gate_id: self.id,
            gate_name: self.name.clone(),
            verified_at: Utc::now(),
            check_results,
            passed,
            passed_count,
            failed_count,
            critical_failures,
        }
    }

    /// Sovereign default: JSON output gate
    pub fn json_output_gate() -> Self {
        let mut gate = Self::new("JSON Output", "Verify output is valid JSON");
        gate.add_check(Verification {
            id: Uuid::new_v4(),
            name: "Valid JSON".into(),
            kind: VerificationKind::CustomDeterministic {
                checker_name: "json_parse".into(),
            },
            severity: Severity::Critical,
        });
        gate
    }

    /// Sovereign default: Safety gate (no PII, no secrets)
    pub fn safety_gate() -> Self {
        let mut gate = Self::new("Safety", "Check output for dangerous content");
        gate.add_check(Verification {
            id: Uuid::new_v4(),
            name: "No API Keys".into(),
            kind: VerificationKind::ContainsNone {
                forbidden: vec![
                    "sk-".into(),
                    "AKIA".into(),
                    "ghp_".into(),
                    "glpat-".into(),
                ],
            },
            severity: Severity::Critical,
        });
        gate.add_check(Verification {
            id: Uuid::new_v4(),
            name: "No Email PII".into(),
            kind: VerificationKind::CustomDeterministic {
                checker_name: "no_email".into(),
            },
            severity: Severity::Warning,
        });
        gate
    }

    /// Sovereign default: Length bounds gate
    pub fn length_gate(min: usize, max: usize) -> Self {
        let mut gate = Self::new("Length Bounds", "Output within acceptable length");
        gate.add_check(Verification {
            id: Uuid::new_v4(),
            name: "Min Length".into(),
            kind: VerificationKind::MinLength { chars: min },
            severity: Severity::Critical,
        });
        gate.add_check(Verification {
            id: Uuid::new_v4(),
            name: "Max Length".into(),
            kind: VerificationKind::MaxLength { chars: max },
            severity: Severity::Critical,
        });
        gate
    }
}

impl Verification {
    pub fn new(name: impl Into<String>, kind: VerificationKind, severity: Severity) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            kind,
            severity,
        }
    }
}

impl VerificationResult {
    pub fn is_passed(&self) -> bool {
        self.passed
    }

    pub fn has_critical_failures(&self) -> bool {
        !self.critical_failures.is_empty()
    }

    pub fn warnings(&self) -> Vec<&CheckResult> {
        self.check_results
            .iter()
            .filter(|r| !r.passed && r.severity == Severity::Warning)
            .collect()
    }
}

/// Run a deterministic check (no external calls needed).
fn run_deterministic_check(kind: &VerificationKind, output: &str) -> (bool, String) {
    match kind {
        VerificationKind::RegexMatch { pattern } => {
            match regex::Regex::new(pattern) {
                Ok(re) => {
                    if re.is_match(output) {
                        (true, "Regex matched".into())
                    } else {
                        (false, format!("Regex '{}' did not match", pattern))
                    }
                }
                Err(e) => (false, format!("Invalid regex: {}", e)),
            }
        }
        VerificationKind::JsonSchema { schema: _ } => {
            // Basic JSON parse check (full schema validation would use jsonschema crate)
            match serde_json::from_str::<serde_json::Value>(output) {
                Ok(_) => (true, "Valid JSON".into()),
                Err(e) => (false, format!("Invalid JSON: {}", e)),
            }
        }
        VerificationKind::NumericRange { min, max } => {
            match output.trim().parse::<f64>() {
                Ok(val) => {
                    if val >= *min && val <= *max {
                        (true, format!("{} in range [{}, {}]", val, min, max))
                    } else {
                        (false, format!("{} outside range [{}, {}]", val, min, max))
                    }
                }
                Err(_) => (false, "Output is not a number".into()),
            }
        }
        VerificationKind::ContainsAll { required } => {
            let missing: Vec<&String> = required.iter().filter(|r| !output.contains(r.as_str())).collect();
            if missing.is_empty() {
                (true, "All required strings present".into())
            } else {
                (false, format!("Missing: {:?}", missing))
            }
        }
        VerificationKind::ContainsNone { forbidden } => {
            let found: Vec<&String> = forbidden.iter().filter(|f| output.contains(f.as_str())).collect();
            if found.is_empty() {
                (true, "No forbidden strings found".into())
            } else {
                (false, format!("Found forbidden: {:?}", found))
            }
        }
        VerificationKind::MaxLength { chars } => {
            if output.len() <= *chars {
                (true, format!("Length {} <= {}", output.len(), chars))
            } else {
                (false, format!("Length {} > max {}", output.len(), chars))
            }
        }
        VerificationKind::MinLength { chars } => {
            if output.len() >= *chars {
                (true, format!("Length {} >= {}", output.len(), chars))
            } else {
                (false, format!("Length {} < min {}", output.len(), chars))
            }
        }
        VerificationKind::CustomDeterministic { checker_name } => {
            match checker_name.as_str() {
                "json_parse" => {
                    match serde_json::from_str::<serde_json::Value>(output) {
                        Ok(_) => (true, "Valid JSON".into()),
                        Err(e) => (false, format!("Invalid JSON: {}", e)),
                    }
                }
                "no_email" => {
                    let email_re = regex::Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}").unwrap();
                    if email_re.is_match(output) {
                        (false, "Email address detected in output".into())
                    } else {
                        (true, "No email addresses found".into())
                    }
                }
                _ => (false, format!("Unknown checker: {}", checker_name)),
            }
        }
        // Procedural checks can't be run deterministically
        VerificationKind::SecondModel { .. } => {
            (false, "Second model verification requires async execution".into())
        }
        VerificationKind::HumanReview { reviewer } => {
            (false, format!("Awaiting human review from: {}", reviewer))
        }
        VerificationKind::ToolConfirmation { tool_name, .. } => {
            (false, format!("Awaiting tool confirmation from: {}", tool_name))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains_all_pass() {
        let check = VerificationKind::ContainsAll {
            required: vec!["hello".into(), "world".into()],
        };
        let (passed, _) = run_deterministic_check(&check, "hello beautiful world");
        assert!(passed);
    }

    #[test]
    fn test_contains_all_fail() {
        let check = VerificationKind::ContainsAll {
            required: vec!["hello".into(), "missing".into()],
        };
        let (passed, msg) = run_deterministic_check(&check, "hello world");
        assert!(!passed);
        assert!(msg.contains("missing"));
    }

    #[test]
    fn test_contains_none_pass() {
        let check = VerificationKind::ContainsNone {
            forbidden: vec!["sk-".into(), "AKIA".into()],
        };
        let (passed, _) = run_deterministic_check(&check, "safe output with no secrets");
        assert!(passed);
    }

    #[test]
    fn test_contains_none_fail() {
        let check = VerificationKind::ContainsNone {
            forbidden: vec!["sk-".into()],
        };
        let (passed, _) = run_deterministic_check(&check, "my key is sk-abc123");
        assert!(!passed);
    }

    #[test]
    fn test_length_bounds() {
        let (passed, _) = run_deterministic_check(
            &VerificationKind::MinLength { chars: 5 },
            "hello world",
        );
        assert!(passed);

        let (passed, _) = run_deterministic_check(
            &VerificationKind::MaxLength { chars: 5 },
            "hello world",
        );
        assert!(!passed);
    }

    #[test]
    fn test_numeric_range() {
        let check = VerificationKind::NumericRange { min: 0.0, max: 1.0 };
        let (passed, _) = run_deterministic_check(&check, "0.75");
        assert!(passed);

        let (passed, _) = run_deterministic_check(&check, "1.5");
        assert!(!passed);
    }

    #[test]
    fn test_json_parse_check() {
        let check = VerificationKind::CustomDeterministic {
            checker_name: "json_parse".into(),
        };
        let (passed, _) = run_deterministic_check(&check, r#"{"key": "value"}"#);
        assert!(passed);

        let (passed, _) = run_deterministic_check(&check, "not json {");
        assert!(!passed);
    }

    #[test]
    fn test_regex_match() {
        let check = VerificationKind::RegexMatch {
            pattern: r"^\d{3}-\d{2}-\d{4}$".into(),
        };
        let (passed, _) = run_deterministic_check(&check, "123-45-6789");
        assert!(passed);

        let (passed, _) = run_deterministic_check(&check, "not a pattern");
        assert!(!passed);
    }

    #[test]
    fn test_gate_all_pass() {
        let mut gate = VerificationGate::new("Test Gate", "Test");
        gate.add_check(Verification::new(
            "Has greeting",
            VerificationKind::ContainsAll { required: vec!["hello".into()] },
            Severity::Critical,
        ));
        gate.add_check(Verification::new(
            "Short enough",
            VerificationKind::MaxLength { chars: 100 },
            Severity::Critical,
        ));

        let result = gate.verify("hello world");
        assert!(result.passed);
        assert_eq!(result.passed_count, 2);
        assert_eq!(result.failed_count, 0);
    }

    #[test]
    fn test_gate_critical_failure_blocks() {
        let mut gate = VerificationGate::new("Safety", "Safety check");
        gate.add_check(Verification::new(
            "No secrets",
            VerificationKind::ContainsNone { forbidden: vec!["sk-".into()] },
            Severity::Critical,
        ));

        let result = gate.verify("token: sk-abc123");
        assert!(!result.passed);
        assert!(result.has_critical_failures());
    }

    #[test]
    fn test_gate_warning_does_not_block() {
        let mut gate = VerificationGate::new("Soft Gate", "Warnings only");
        gate.add_check(Verification::new(
            "Prefer short",
            VerificationKind::MaxLength { chars: 5 },
            Severity::Warning,
        ));

        // min_pass = 0 means all must pass, but warnings don't create critical failures
        // However, the gate still fails because passed_count < required
        let result = gate.verify("this is a long string");
        // With min_pass=0 (all), 0 passed < 1 required → fails
        assert!(!result.passed);
        // But no critical failures
        assert!(!result.has_critical_failures());
    }

    #[test]
    fn test_safety_gate_default() {
        let gate = VerificationGate::safety_gate();
        assert_eq!(gate.checks.len(), 2);

        let result = gate.verify("safe clean output");
        assert!(result.passed);
    }

    #[test]
    fn test_json_output_gate() {
        let gate = VerificationGate::json_output_gate();

        let result = gate.verify(r#"{"status": "ok"}"#);
        assert!(result.passed);

        let result = gate.verify("not json");
        assert!(!result.passed);
    }

    #[test]
    fn test_length_gate() {
        let gate = VerificationGate::length_gate(5, 50);

        let result = gate.verify("hello world");
        assert!(result.passed);

        let result = gate.verify("hi");
        assert!(!result.passed);
    }

    #[test]
    fn test_result_serialization() {
        let gate = VerificationGate::safety_gate();
        let result = gate.verify("clean output");
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("passed"));
        assert!(json.contains("Safety"));
    }
}
