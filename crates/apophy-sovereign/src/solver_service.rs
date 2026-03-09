//! Solver Service — REST API handlers for Enterprise Problem Solver

use axum::{extract::State, http::StatusCode, response::Json};
use serde::{Deserialize, Serialize};

use crate::AppState;

// ─── Status Handler ─────────────────────────────────────────

pub async fn solver_status_handler(
    State(state): State<AppState>,
) -> Json<apophy_solver::SolverStatus> {
    let solver = state.solver.lock().unwrap();
    Json(solver.status())
}

// ─── Submit Problem Handler ─────────────────────────────────

#[derive(Deserialize)]
pub struct SubmitProblemRequest {
    pub title: String,
    pub description: String,
    pub domain: String,
    pub severity: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub affected_systems: Vec<String>,
    #[serde(default = "default_reporter")]
    pub reported_by: String,
}

fn default_reporter() -> String {
    "api".into()
}

#[derive(Serialize)]
pub struct SubmitProblemResponse {
    pub problem_id: String,
    pub status: String,
    pub summary: String,
    pub report: apophy_solver::SolverReport,
}

pub async fn solver_submit_handler(
    State(state): State<AppState>,
    Json(req): Json<SubmitProblemRequest>,
) -> Result<Json<SubmitProblemResponse>, StatusCode> {
    let domain = parse_domain(&req.domain);
    let severity = parse_severity(&req.severity);

    let problem = apophy_solver::Problem::new(
        &req.title,
        &req.description,
        domain,
        severity,
        &req.reported_by,
    )
    .with_tags(req.tags)
    .with_affected_systems(req.affected_systems);

    let mut solver = state.solver.lock().unwrap();
    let report = solver.submit(problem);

    let problem_id = report.problem.id.to_string();
    let summary = report.summary();

    Ok(Json(SubmitProblemResponse {
        problem_id,
        status: report.problem.status.to_string(),
        summary,
        report,
    }))
}

// ─── Problems List Handler ──────────────────────────────────

#[derive(Serialize)]
pub struct ProblemSummary {
    pub id: String,
    pub title: String,
    pub domain: String,
    pub severity: String,
    pub status: String,
    pub created_at: String,
}

pub async fn solver_problems_handler(
    State(state): State<AppState>,
) -> Json<Vec<ProblemSummary>> {
    let solver = state.solver.lock().unwrap();
    let problems: Vec<ProblemSummary> = solver.problems.iter().map(|p| ProblemSummary {
        id: p.id.to_string(),
        title: p.title.clone(),
        domain: p.domain.to_string(),
        severity: p.severity.to_string(),
        status: p.status.to_string(),
        created_at: p.created_at.to_rfc3339(),
    }).collect();
    Json(problems)
}

// ─── Knowledge Base Handler ─────────────────────────────────

pub async fn solver_knowledge_handler(
    State(state): State<AppState>,
) -> Json<apophy_solver::KnowledgeStats> {
    let solver = state.solver.lock().unwrap();
    Json(solver.knowledge_base.stats())
}

#[derive(Deserialize)]
pub struct KnowledgeSearchRequest {
    pub query: String,
}

pub async fn solver_knowledge_search_handler(
    State(state): State<AppState>,
    Json(req): Json<KnowledgeSearchRequest>,
) -> Json<Vec<apophy_solver::KnowledgeEntry>> {
    let mut solver = state.solver.lock().unwrap();
    let results = solver.knowledge_base.search(&req.query);
    Json(results.into_iter().cloned().collect())
}

// ─── Helpers ────────────────────────────────────────────────

fn parse_domain(s: &str) -> apophy_solver::Domain {
    match s.to_lowercase().as_str() {
        "infrastructure" => apophy_solver::Domain::Infrastructure,
        "security" => apophy_solver::Domain::Security,
        "performance" => apophy_solver::Domain::Performance,
        "data" | "data-integrity" | "dataintegrity" => apophy_solver::Domain::DataIntegrity,
        "compliance" => apophy_solver::Domain::Compliance,
        "financial" | "finance" => apophy_solver::Domain::Financial,
        "operations" | "ops" => apophy_solver::Domain::Operations,
        "hr" | "human-resources" => apophy_solver::Domain::HumanResources,
        "cx" | "customer" | "customer-experience" => apophy_solver::Domain::CustomerExperience,
        "supply-chain" | "supply" => apophy_solver::Domain::SupplyChain,
        "legal" => apophy_solver::Domain::Legal,
        "strategy" => apophy_solver::Domain::Strategy,
        _ => apophy_solver::Domain::Infrastructure,
    }
}

fn parse_severity(s: &str) -> apophy_solver::Severity {
    match s.to_lowercase().as_str() {
        "critical" => apophy_solver::Severity::Critical,
        "high" => apophy_solver::Severity::High,
        "medium" => apophy_solver::Severity::Medium,
        "low" => apophy_solver::Severity::Low,
        "info" => apophy_solver::Severity::Info,
        _ => apophy_solver::Severity::Medium,
    }
}
