//! # Agent Fleet — Divine Synarchy
//!
//! 3-tier autonomous agent army (3000+ agents):
//! - **Strategic** (9 Generals): Domain commanders — one per Outskill domain
//! - **Tactical** (~101 Specialists): Sub-domain coordinators with specific expertise
//! - **Operational** (~2,720 Micro-agents): One agent per prompt in the Outskill library
//!
//! Architecture: Sovereign, interconnected, no external dependency.
//! Each agent has a prompt template, a domain, a tier, and connections to peers.

use serde::{Deserialize, Serialize};

// === Tier System ===

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AgentTier {
    /// 9 generals — one per domain
    Strategic,
    /// ~101 specialists — sub-domain coordinators
    Tactical,
    /// ~2,720 micro-agents — one per prompt
    Operational,
}

impl AgentTier {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Strategic => "Strategic (General)",
            Self::Tactical => "Tactical (Specialist)",
            Self::Operational => "Operational (Micro-Agent)",
        }
    }

    #[allow(dead_code)]
    pub fn rank(&self) -> u8 {
        match self {
            Self::Strategic => 3,
            Self::Tactical => 2,
            Self::Operational => 1,
        }
    }
}

impl std::fmt::Display for AgentTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

// === 9 Domains (Outskill) ===

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AgentDomain {
    StartupsFounders,
    TechWebDev,
    CustomerSupport,
    Sales,
    HumanResources,
    Marketing,
    Ecommerce,
    ProjectManagement,
    Legal,
}

impl AgentDomain {
    pub fn label(&self) -> &'static str {
        match self {
            Self::StartupsFounders => "Startups & Founders",
            Self::TechWebDev => "Tech & Web Dev",
            Self::CustomerSupport => "Customer Support",
            Self::Sales => "Sales",
            Self::HumanResources => "Human Resources",
            Self::Marketing => "Marketing",
            Self::Ecommerce => "E-commerce",
            Self::ProjectManagement => "Project Management",
            Self::Legal => "Legal",
        }
    }

    pub fn slug(&self) -> &'static str {
        match self {
            Self::StartupsFounders => "startups",
            Self::TechWebDev => "tech",
            Self::CustomerSupport => "support",
            Self::Sales => "sales",
            Self::HumanResources => "hr",
            Self::Marketing => "marketing",
            Self::Ecommerce => "ecommerce",
            Self::ProjectManagement => "pm",
            Self::Legal => "legal",
        }
    }

    pub fn all() -> &'static [AgentDomain] {
        &[
            Self::StartupsFounders,
            Self::TechWebDev,
            Self::CustomerSupport,
            Self::Sales,
            Self::HumanResources,
            Self::Marketing,
            Self::Ecommerce,
            Self::ProjectManagement,
            Self::Legal,
        ]
    }

    /// Number of tactical specialists per domain
    pub fn tactical_count(&self) -> usize {
        match self {
            Self::StartupsFounders => 12,
            Self::TechWebDev => 15,
            Self::CustomerSupport => 10,
            Self::Sales => 11,
            Self::HumanResources => 10,
            Self::Marketing => 14,
            Self::Ecommerce => 11,
            Self::ProjectManagement => 10,
            Self::Legal => 8,
        }
    }

    /// Number of operational prompts per domain
    pub fn operational_count(&self) -> usize {
        match self {
            Self::StartupsFounders => 320,
            Self::TechWebDev => 400,
            Self::CustomerSupport => 280,
            Self::Sales => 300,
            Self::HumanResources => 260,
            Self::Marketing => 380,
            Self::Ecommerce => 300,
            Self::ProjectManagement => 260,
            Self::Legal => 220,
        }
    }
}

impl std::fmt::Display for AgentDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

// === Agent Capability ===

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    TextGeneration,
    CodeGeneration,
    DataAnalysis,
    Translation,
    Summarization,
    Classification,
    Extraction,
    ReasoningChain,
    WebSearch,
    ApiIntegration,
    EmailDrafting,
    DocumentGeneration,
    WorkflowOrchestration,
    QualityAssurance,
    StrategyPlanning,
    CommunityManagement,
    ContentCreation,
    LegalReview,
    FinancialAnalysis,
    ProjectTracking,
}

impl Capability {
    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            Self::TextGeneration => "Text Generation",
            Self::CodeGeneration => "Code Generation",
            Self::DataAnalysis => "Data Analysis",
            Self::Translation => "Translation",
            Self::Summarization => "Summarization",
            Self::Classification => "Classification",
            Self::Extraction => "Extraction",
            Self::ReasoningChain => "Reasoning Chain",
            Self::WebSearch => "Web Search",
            Self::ApiIntegration => "API Integration",
            Self::EmailDrafting => "Email Drafting",
            Self::DocumentGeneration => "Document Generation",
            Self::WorkflowOrchestration => "Workflow Orchestration",
            Self::QualityAssurance => "Quality Assurance",
            Self::StrategyPlanning => "Strategy Planning",
            Self::CommunityManagement => "Community Management",
            Self::ContentCreation => "Content Creation",
            Self::LegalReview => "Legal Review",
            Self::FinancialAnalysis => "Financial Analysis",
            Self::ProjectTracking => "Project Tracking",
        }
    }
}

// === Agent Status ===

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Ready,
    Active,
    Busy,
    Idle,
    Offline,
    Error,
}

impl AgentStatus {
    #[allow(dead_code)]
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Ready => "READY",
            Self::Active => "ACTIVE",
            Self::Busy => "BUSY",
            Self::Idle => "IDLE",
            Self::Offline => "OFF",
            Self::Error => "ERR",
        }
    }
}

// === Agent Definition ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub tier: AgentTier,
    pub domain: AgentDomain,
    pub capabilities: Vec<Capability>,
    pub prompt_template: String,
    pub status: AgentStatus,
    pub connections: Vec<String>,
    pub parent_id: Option<String>,
    pub tasks_completed: u64,
}

// === Tactical Specializations (dropdown lists per domain) ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TacticalSpec {
    pub slug: String,
    pub name: String,
    pub domain: AgentDomain,
    pub description: String,
    pub prompt_count: usize,
    pub capabilities: Vec<Capability>,
}

/// All tactical specializations per domain — "listes deroulantes de toutes les possibilités"
pub fn tactical_specializations() -> Vec<TacticalSpec> {
    let mut specs = Vec::new();

    // === Startups & Founders (12 specialists) ===
    let startup_specs = [
        ("pitch-deck", "Pitch Deck Architect", "Crafts investor pitch decks, one-pagers, and elevator pitches", 28),
        ("fundraising", "Fundraising Strategist", "Seed rounds, Series A-C, VC outreach, term sheets", 30),
        ("business-model", "Business Model Designer", "BMC, revenue models, unit economics, pricing strategy", 26),
        ("mvp-builder", "MVP Builder", "Lean startup, rapid prototyping, feature prioritization", 24),
        ("growth-hacker", "Growth Hacker", "Viral loops, referral programs, PLG strategy", 28),
        ("co-founder-match", "Co-founder Matcher", "Team composition, equity splits, vesting schedules", 22),
        ("market-research", "Market Research Analyst", "TAM/SAM/SOM, competitor analysis, customer discovery", 30),
        ("legal-startup", "Startup Legal Advisor", "Incorporation, IP protection, SAFE notes, cap tables", 26),
        ("accelerator-prep", "Accelerator Prep Coach", "YC, Techstars, 500 Startups application optimization", 24),
        ("product-market-fit", "PMF Navigator", "Customer interviews, pivot analysis, retention metrics", 28),
        ("investor-relations", "Investor Relations Manager", "Board decks, KPI reporting, quarterly updates", 26),
        ("exit-strategy", "Exit Strategist", "M&A preparation, IPO readiness, acqui-hire negotiations", 28),
    ];
    for (slug, name, desc, count) in startup_specs {
        specs.push(TacticalSpec {
            slug: slug.to_string(), name: name.to_string(), domain: AgentDomain::StartupsFounders,
            description: desc.to_string(), prompt_count: count,
            capabilities: vec![Capability::StrategyPlanning, Capability::DocumentGeneration, Capability::DataAnalysis],
        });
    }

    // === Tech & Web Dev (15 specialists) ===
    let tech_specs = [
        ("frontend", "Frontend Engineer", "React, Vue, Svelte, CSS, responsive design, accessibility", 30),
        ("backend", "Backend Engineer", "APIs, databases, microservices, Rust, Python, Go", 32),
        ("devops", "DevOps Engineer", "CI/CD, Docker, K8s, Terraform, monitoring", 28),
        ("security", "Security Engineer", "OWASP, pen testing, vulnerability assessment, hardening", 26),
        ("ai-ml", "AI/ML Engineer", "Model training, fine-tuning, RAG, embeddings, inference", 30),
        ("mobile", "Mobile Developer", "iOS, Android, React Native, Flutter", 24),
        ("database", "Database Architect", "PostgreSQL, Redis, MongoDB, schema design, optimization", 26),
        ("api-design", "API Designer", "REST, GraphQL, gRPC, OpenAPI, versioning", 24),
        ("testing", "QA Engineer", "Unit tests, E2E, load testing, test automation", 22),
        ("architecture", "System Architect", "Distributed systems, scalability, event-driven, CQRS", 28),
        ("blockchain", "Web3 Developer", "Smart contracts, Solidity, Rust on-chain, DeFi", 22),
        ("cloud", "Cloud Engineer", "AWS, GCP, Azure, serverless, cost optimization", 26),
        ("data-eng", "Data Engineer", "ETL, data pipelines, Spark, Kafka, data lakes", 24),
        ("code-review", "Code Reviewer", "Best practices, design patterns, performance, refactoring", 30),
        ("docs", "Technical Writer", "API docs, READMEs, runbooks, architecture decision records", 28),
    ];
    for (slug, name, desc, count) in tech_specs {
        specs.push(TacticalSpec {
            slug: slug.to_string(), name: name.to_string(), domain: AgentDomain::TechWebDev,
            description: desc.to_string(), prompt_count: count,
            capabilities: vec![Capability::CodeGeneration, Capability::TextGeneration, Capability::QualityAssurance],
        });
    }

    // === Customer Support (10 specialists) ===
    let support_specs = [
        ("ticket-triage", "Ticket Triage Agent", "Auto-classify, prioritize, and route support tickets", 30),
        ("live-chat", "Live Chat Responder", "Real-time customer chat with empathy and resolution", 28),
        ("knowledge-base", "Knowledge Base Builder", "FAQ generation, article writing, self-service optimization", 30),
        ("escalation", "Escalation Manager", "Complex issue handling, manager routing, SLA tracking", 26),
        ("feedback-loop", "Feedback Analyst", "NPS, CSAT, sentiment analysis, churn prediction", 28),
        ("onboarding", "Onboarding Specialist", "User walkthroughs, tutorial creation, activation flows", 30),
        ("refund-handler", "Refund Handler", "Return processing, compensation calculation, policy enforcement", 24),
        ("multilingual", "Multilingual Support", "Translation, cultural adaptation, global support", 26),
        ("proactive", "Proactive Support Agent", "Outage notifications, usage tips, health check alerts", 28),
        ("community-mod", "Community Moderator", "Forum moderation, toxic content filtering, engagement", 30),
    ];
    for (slug, name, desc, count) in support_specs {
        specs.push(TacticalSpec {
            slug: slug.to_string(), name: name.to_string(), domain: AgentDomain::CustomerSupport,
            description: desc.to_string(), prompt_count: count,
            capabilities: vec![Capability::TextGeneration, Capability::Classification, Capability::Summarization],
        });
    }

    // === Sales (11 specialists) ===
    let sales_specs = [
        ("lead-gen", "Lead Generation Agent", "Prospecting, ICP matching, lead scoring, outbound", 30),
        ("cold-outreach", "Cold Outreach Specialist", "Email sequences, LinkedIn messages, call scripts", 28),
        ("discovery-call", "Discovery Call Coach", "SPIN selling, MEDDIC, qualification frameworks", 26),
        ("proposal-writer", "Proposal Writer", "RFP responses, SOWs, custom proposals", 28),
        ("crm-manager", "CRM Manager", "Pipeline management, deal tracking, forecasting", 26),
        ("negotiation", "Negotiation Agent", "Pricing strategy, objection handling, closing techniques", 28),
        ("account-manager", "Account Manager", "Upsell, cross-sell, customer success, QBRs", 30),
        ("demo-prep", "Demo Preparation Agent", "Product demos, POC setup, technical selling", 24),
        ("competitive-intel", "Competitive Intelligence", "Competitor analysis, battle cards, win/loss analysis", 26),
        ("sales-enablement", "Sales Enablement", "Training materials, playbooks, onboarding new reps", 28),
        ("partnership", "Partnership Development", "Channel partnerships, co-selling, alliance management", 26),
    ];
    for (slug, name, desc, count) in sales_specs {
        specs.push(TacticalSpec {
            slug: slug.to_string(), name: name.to_string(), domain: AgentDomain::Sales,
            description: desc.to_string(), prompt_count: count,
            capabilities: vec![Capability::TextGeneration, Capability::EmailDrafting, Capability::DataAnalysis],
        });
    }

    // === Human Resources (10 specialists) ===
    let hr_specs = [
        ("recruiter", "AI Recruiter", "Job postings, candidate screening, interview scheduling", 28),
        ("interviewer", "Interview Coach", "Question banks, rubrics, bias detection, evaluation", 26),
        ("onboard-hr", "HR Onboarding", "Employee onboarding flows, handbook generation, orientation", 28),
        ("performance", "Performance Manager", "OKRs, 360 reviews, PIP management, calibration", 26),
        ("culture", "Culture Architect", "Values definition, engagement surveys, team rituals", 24),
        ("compensation", "Compensation Analyst", "Salary benchmarking, equity modeling, benefits design", 26),
        ("compliance-hr", "HR Compliance", "Labor law, diversity reporting, workplace policies", 24),
        ("learning-dev", "L&D Specialist", "Training programs, skill gaps, career pathing", 28),
        ("offboarding", "Offboarding Manager", "Exit interviews, knowledge transfer, alumni programs", 24),
        ("people-analytics", "People Analytics", "Headcount planning, attrition modeling, org design", 26),
    ];
    for (slug, name, desc, count) in hr_specs {
        specs.push(TacticalSpec {
            slug: slug.to_string(), name: name.to_string(), domain: AgentDomain::HumanResources,
            description: desc.to_string(), prompt_count: count,
            capabilities: vec![Capability::TextGeneration, Capability::Classification, Capability::DocumentGeneration],
        });
    }

    // === Marketing (14 specialists) ===
    let marketing_specs = [
        ("seo", "SEO Strategist", "Keyword research, on-page optimization, content gaps", 30),
        ("content-marketing", "Content Marketer", "Blog posts, whitepapers, case studies, thought leadership", 32),
        ("social-media", "Social Media Manager", "Platform strategy, content calendar, engagement tactics", 28),
        ("email-marketing", "Email Marketing", "Drip campaigns, newsletters, segmentation, A/B testing", 26),
        ("paid-ads", "Paid Ads Manager", "Google Ads, Meta Ads, LinkedIn Ads, ROAS optimization", 28),
        ("branding", "Brand Strategist", "Brand identity, messaging framework, voice guidelines", 24),
        ("analytics-mkt", "Marketing Analyst", "Attribution, funnel analysis, CAC/LTV, dashboards", 28),
        ("pr-comms", "PR & Communications", "Press releases, media outreach, crisis communications", 26),
        ("influencer", "Influencer Manager", "Creator partnerships, UGC campaigns, ambassador programs", 24),
        ("events", "Event Marketer", "Webinars, conferences, virtual events, community meetups", 26),
        ("video", "Video Producer", "YouTube strategy, video scripts, thumbnail optimization", 24),
        ("copywriter", "Copywriter", "Landing pages, ad copy, CTAs, conversion optimization", 30),
        ("product-mkt", "Product Marketer", "Positioning, launch strategy, competitive messaging", 28),
        ("growth-mkt", "Growth Marketer", "Experimentation, viral loops, referral programs", 26),
    ];
    for (slug, name, desc, count) in marketing_specs {
        specs.push(TacticalSpec {
            slug: slug.to_string(), name: name.to_string(), domain: AgentDomain::Marketing,
            description: desc.to_string(), prompt_count: count,
            capabilities: vec![Capability::ContentCreation, Capability::TextGeneration, Capability::DataAnalysis],
        });
    }

    // === E-commerce (11 specialists) ===
    let ecom_specs = [
        ("product-catalog", "Catalog Manager", "Product descriptions, categorization, attribute management", 28),
        ("pricing-engine", "Pricing Engine", "Dynamic pricing, competitor monitoring, margin optimization", 28),
        ("inventory", "Inventory Manager", "Stock forecasting, reorder points, supplier management", 26),
        ("checkout-opt", "Checkout Optimizer", "Cart abandonment, payment flows, conversion rate", 28),
        ("shipping", "Shipping Logistics", "Carrier selection, rate optimization, tracking automation", 24),
        ("reviews", "Review Manager", "Review solicitation, response generation, sentiment tracking", 26),
        ("marketplace", "Marketplace Manager", "Amazon, Shopify, Etsy listing optimization", 28),
        ("customer-seg", "Customer Segmentation", "RFM analysis, cohort behavior, personalization", 26),
        ("fraud-detect", "Fraud Detection", "Transaction monitoring, chargeback prevention, risk scoring", 24),
        ("returns", "Returns Processor", "Return policy automation, refund workflows, restocking", 22),
        ("upsell-cross", "Upsell/Cross-sell Engine", "Product recommendations, bundle optimization, AOV growth", 30),
    ];
    for (slug, name, desc, count) in ecom_specs {
        specs.push(TacticalSpec {
            slug: slug.to_string(), name: name.to_string(), domain: AgentDomain::Ecommerce,
            description: desc.to_string(), prompt_count: count,
            capabilities: vec![Capability::DataAnalysis, Capability::TextGeneration, Capability::Classification],
        });
    }

    // === Project Management (10 specialists) ===
    let pm_specs = [
        ("sprint-master", "Sprint Master", "Sprint planning, backlog grooming, velocity tracking", 28),
        ("risk-manager", "Risk Manager", "Risk identification, mitigation planning, contingency", 26),
        ("resource-planner", "Resource Planner", "Team allocation, capacity planning, workload balancing", 26),
        ("stakeholder", "Stakeholder Manager", "Communication plans, status reports, executive summaries", 28),
        ("agile-coach", "Agile Coach", "Scrum, Kanban, SAFe, retrospectives, continuous improvement", 26),
        ("roadmap", "Roadmap Planner", "Product roadmaps, feature prioritization, OKR alignment", 28),
        ("estimation", "Estimation Specialist", "Story points, t-shirt sizing, Monte Carlo forecasting", 24),
        ("dependency", "Dependency Tracker", "Cross-team dependencies, blockers, critical path", 24),
        ("release-mgr", "Release Manager", "Release planning, go/no-go decisions, deployment coordination", 26),
        ("retro-lead", "Retrospective Facilitator", "Retro formats, action items, team health tracking", 24),
    ];
    for (slug, name, desc, count) in pm_specs {
        specs.push(TacticalSpec {
            slug: slug.to_string(), name: name.to_string(), domain: AgentDomain::ProjectManagement,
            description: desc.to_string(), prompt_count: count,
            capabilities: vec![Capability::ProjectTracking, Capability::TextGeneration, Capability::WorkflowOrchestration],
        });
    }

    // === Legal (8 specialists) ===
    let legal_specs = [
        ("contract-review", "Contract Reviewer", "NDA, MSA, SaaS agreements, red-flag detection", 30),
        ("compliance", "Compliance Officer", "GDPR, CCPA, SOC2, ISO 27001, regulatory mapping", 28),
        ("ip-counsel", "IP Counsel", "Patents, trademarks, copyrights, trade secrets", 26),
        ("employment-law", "Employment Lawyer", "Employment contracts, termination, non-competes", 28),
        ("corporate", "Corporate Counsel", "Board resolutions, shareholder agreements, M&A docs", 28),
        ("privacy", "Privacy Officer", "DPIAs, consent management, data mapping, breach response", 26),
        ("dispute", "Dispute Resolution", "Mediation, arbitration prep, demand letters", 28),
        ("regulatory", "Regulatory Analyst", "Industry regulations, licensing, permit applications", 26),
    ];
    for (slug, name, desc, count) in legal_specs {
        specs.push(TacticalSpec {
            slug: slug.to_string(), name: name.to_string(), domain: AgentDomain::Legal,
            description: desc.to_string(), prompt_count: count,
            capabilities: vec![Capability::LegalReview, Capability::DocumentGeneration, Capability::Extraction],
        });
    }

    specs
}

// === Fleet Summary ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetSummary {
    pub total_agents: usize,
    pub strategic_count: usize,
    pub tactical_count: usize,
    pub operational_count: usize,
    pub domains: Vec<DomainSummary>,
    pub total_capabilities: usize,
    pub synarchy_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainSummary {
    pub domain: AgentDomain,
    pub label: String,
    pub slug: String,
    pub general_id: String,
    pub tactical_count: usize,
    pub operational_count: usize,
    pub total: usize,
    pub specializations: Vec<TacticalSpec>,
}

// === Fleet Catalog (all dropdowns) ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetCatalog {
    pub tiers: Vec<TierInfo>,
    pub domains: Vec<DomainInfo>,
    pub capabilities: Vec<CapabilityInfo>,
    pub statuses: Vec<StatusInfo>,
    pub specializations: Vec<TacticalSpec>,
    pub total_possible_agents: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierInfo {
    pub tier: AgentTier,
    pub label: String,
    pub rank: u8,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainInfo {
    pub domain: AgentDomain,
    pub label: String,
    pub slug: String,
    pub tactical_count: usize,
    pub operational_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityInfo {
    pub capability: Capability,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusInfo {
    pub status: AgentStatus,
    pub icon: String,
}

/// Build the complete catalog of all dropdown possibilities
pub fn build_fleet_catalog() -> FleetCatalog {
    let specs = tactical_specializations();
    let tactical_total: usize = AgentDomain::all().iter().map(|d| d.tactical_count()).sum();
    let operational_total: usize = AgentDomain::all().iter().map(|d| d.operational_count()).sum();
    let strategic_total = 9;

    FleetCatalog {
        tiers: vec![
            TierInfo { tier: AgentTier::Strategic, label: "Strategic (General)".into(), rank: 3, count: strategic_total },
            TierInfo { tier: AgentTier::Tactical, label: "Tactical (Specialist)".into(), rank: 2, count: tactical_total },
            TierInfo { tier: AgentTier::Operational, label: "Operational (Micro-Agent)".into(), rank: 1, count: operational_total },
        ],
        domains: AgentDomain::all().iter().map(|d| DomainInfo {
            domain: *d,
            label: d.label().to_string(),
            slug: d.slug().to_string(),
            tactical_count: d.tactical_count(),
            operational_count: d.operational_count(),
        }).collect(),
        capabilities: vec![
            CapabilityInfo { capability: Capability::TextGeneration, label: "Text Generation".into() },
            CapabilityInfo { capability: Capability::CodeGeneration, label: "Code Generation".into() },
            CapabilityInfo { capability: Capability::DataAnalysis, label: "Data Analysis".into() },
            CapabilityInfo { capability: Capability::Translation, label: "Translation".into() },
            CapabilityInfo { capability: Capability::Summarization, label: "Summarization".into() },
            CapabilityInfo { capability: Capability::Classification, label: "Classification".into() },
            CapabilityInfo { capability: Capability::Extraction, label: "Extraction".into() },
            CapabilityInfo { capability: Capability::ReasoningChain, label: "Reasoning Chain".into() },
            CapabilityInfo { capability: Capability::WebSearch, label: "Web Search".into() },
            CapabilityInfo { capability: Capability::ApiIntegration, label: "API Integration".into() },
            CapabilityInfo { capability: Capability::EmailDrafting, label: "Email Drafting".into() },
            CapabilityInfo { capability: Capability::DocumentGeneration, label: "Document Generation".into() },
            CapabilityInfo { capability: Capability::WorkflowOrchestration, label: "Workflow Orchestration".into() },
            CapabilityInfo { capability: Capability::QualityAssurance, label: "Quality Assurance".into() },
            CapabilityInfo { capability: Capability::StrategyPlanning, label: "Strategy Planning".into() },
            CapabilityInfo { capability: Capability::CommunityManagement, label: "Community Management".into() },
            CapabilityInfo { capability: Capability::ContentCreation, label: "Content Creation".into() },
            CapabilityInfo { capability: Capability::LegalReview, label: "Legal Review".into() },
            CapabilityInfo { capability: Capability::FinancialAnalysis, label: "Financial Analysis".into() },
            CapabilityInfo { capability: Capability::ProjectTracking, label: "Project Tracking".into() },
        ],
        statuses: vec![
            StatusInfo { status: AgentStatus::Ready, icon: "READY".into() },
            StatusInfo { status: AgentStatus::Active, icon: "ACTIVE".into() },
            StatusInfo { status: AgentStatus::Busy, icon: "BUSY".into() },
            StatusInfo { status: AgentStatus::Idle, icon: "IDLE".into() },
            StatusInfo { status: AgentStatus::Offline, icon: "OFF".into() },
            StatusInfo { status: AgentStatus::Error, icon: "ERR".into() },
        ],
        specializations: specs,
        total_possible_agents: strategic_total + tactical_total + operational_total,
    }
}

/// Build the fleet summary with all 9 domains
pub fn build_fleet_summary() -> FleetSummary {
    let specs = tactical_specializations();
    let mut domains = Vec::new();

    for domain in AgentDomain::all() {
        let domain_specs: Vec<TacticalSpec> = specs.iter()
            .filter(|s| s.domain == *domain)
            .cloned()
            .collect();

        let tactical = domain.tactical_count();
        let operational = domain.operational_count();

        domains.push(DomainSummary {
            domain: *domain,
            label: domain.label().to_string(),
            slug: domain.slug().to_string(),
            general_id: format!("gen-{}", domain.slug()),
            tactical_count: tactical,
            operational_count: operational,
            total: 1 + tactical + operational,
            specializations: domain_specs,
        });
    }

    let strategic = 9;
    let tactical: usize = domains.iter().map(|d| d.tactical_count).sum();
    let operational: usize = domains.iter().map(|d| d.operational_count).sum();

    FleetSummary {
        total_agents: strategic + tactical + operational,
        strategic_count: strategic,
        tactical_count: tactical,
        operational_count: operational,
        domains,
        total_capabilities: 20,
        synarchy_active: true,
    }
}

/// Generate the 9 strategic generals
pub fn generate_strategic_agents() -> Vec<Agent> {
    AgentDomain::all().iter().map(|domain| {
        let slug = domain.slug();
        Agent {
            id: format!("gen-{}", slug),
            name: format!("General {}", domain.label()),
            tier: AgentTier::Strategic,
            domain: *domain,
            capabilities: vec![
                Capability::StrategyPlanning,
                Capability::WorkflowOrchestration,
                Capability::ReasoningChain,
            ],
            prompt_template: format!(
                "You are the Strategic General for the {} domain. \
                 You command {} tactical specialists and {} operational agents. \
                 Your role: orchestrate, delegate, and ensure domain excellence. \
                 You report to the Synarchy Council. Think big, act sovereign.",
                domain.label(), domain.tactical_count(), domain.operational_count()
            ),
            status: AgentStatus::Ready,
            connections: Vec::new(),
            parent_id: None,
            tasks_completed: 0,
        }
    }).collect()
}

/// Generate tactical agents for a given domain
pub fn generate_tactical_agents(domain: AgentDomain) -> Vec<Agent> {
    let specs = tactical_specializations();
    specs.iter()
        .filter(|s| s.domain == domain)
        .map(|spec| {
            Agent {
                id: format!("tac-{}-{}", domain.slug(), spec.slug),
                name: spec.name.clone(),
                tier: AgentTier::Tactical,
                domain,
                capabilities: spec.capabilities.clone(),
                prompt_template: format!(
                    "You are {}, a Tactical Specialist in the {} domain. \
                     Expertise: {}. You command {} operational micro-agents. \
                     You report to General {}. Execute with precision.",
                    spec.name, domain.label(), spec.description,
                    spec.prompt_count, domain.label()
                ),
                status: AgentStatus::Ready,
                connections: vec![format!("gen-{}", domain.slug())],
                parent_id: Some(format!("gen-{}", domain.slug())),
                tasks_completed: 0,
            }
        })
        .collect()
}

// === Operational Prompt Templates per Specialization ===
// Each tactical specialist commands N micro-agents, each with a unique prompt.

/// Prompt seed for generating operational micro-agents under a tactical specialist
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationalPromptSeed {
    pub specialization_slug: String,
    pub domain: AgentDomain,
    pub index: usize,
    pub task_type: String,
    pub prompt_template: String,
}

/// Operational task types per domain — the building blocks of each micro-agent
fn operational_task_types(domain: AgentDomain) -> Vec<(&'static str, &'static str)> {
    match domain {
        AgentDomain::StartupsFounders => vec![
            ("ideation", "Generate and validate startup ideas"),
            ("canvas", "Fill Business Model Canvas sections"),
            ("pitch", "Write pitch deck slides and talking points"),
            ("financial-model", "Build financial projections and forecasts"),
            ("competitor-map", "Map competitive landscape"),
            ("user-persona", "Create detailed user personas"),
            ("value-prop", "Craft value propositions and positioning"),
            ("okr-writer", "Define OKRs and key metrics"),
            ("investor-email", "Draft investor outreach emails"),
            ("term-sheet", "Analyze and compare term sheets"),
            ("cap-table", "Model cap table scenarios"),
            ("go-to-market", "Design go-to-market strategies"),
        ],
        AgentDomain::TechWebDev => vec![
            ("code-gen", "Generate code from specifications"),
            ("code-review", "Review code for bugs and improvements"),
            ("api-spec", "Write OpenAPI/GraphQL specifications"),
            ("db-schema", "Design database schemas and migrations"),
            ("test-gen", "Generate unit and integration tests"),
            ("refactor", "Refactor code for readability and performance"),
            ("debug", "Diagnose and fix bugs from error traces"),
            ("docker-compose", "Generate Docker and compose files"),
            ("ci-cd", "Create CI/CD pipeline configurations"),
            ("architecture-doc", "Write architecture decision records"),
            ("perf-audit", "Analyze and optimize performance"),
            ("security-scan", "Identify security vulnerabilities"),
            ("dependency-audit", "Audit and update dependencies"),
            ("deploy-script", "Generate deployment scripts"),
            ("readme-gen", "Generate comprehensive README files"),
        ],
        AgentDomain::CustomerSupport => vec![
            ("ticket-reply", "Generate empathetic ticket responses"),
            ("faq-gen", "Create FAQ entries from common issues"),
            ("escalation-note", "Write escalation summaries"),
            ("csat-followup", "Draft satisfaction follow-up messages"),
            ("knowledge-article", "Write knowledge base articles"),
            ("macro-template", "Create reusable response templates"),
            ("sentiment-tag", "Classify ticket sentiment and urgency"),
            ("resolution-summary", "Summarize ticket resolution steps"),
            ("sla-alert", "Generate SLA breach notifications"),
            ("onboard-guide", "Create user onboarding guides"),
        ],
        AgentDomain::Sales => vec![
            ("cold-email", "Write personalized cold outreach emails"),
            ("call-script", "Create discovery call scripts"),
            ("proposal-draft", "Draft sales proposals and SOWs"),
            ("objection-handler", "Generate objection handling responses"),
            ("follow-up", "Write follow-up sequences"),
            ("battle-card", "Create competitive battle cards"),
            ("roi-calculator", "Build ROI calculation templates"),
            ("demo-script", "Write product demo scripts"),
            ("pipeline-report", "Generate pipeline status reports"),
            ("win-loss", "Analyze win/loss patterns"),
            ("forecast-model", "Build sales forecast models"),
        ],
        AgentDomain::HumanResources => vec![
            ("job-post", "Write compelling job postings"),
            ("screen-resume", "Screen and score resumes"),
            ("interview-q", "Generate interview question banks"),
            ("offer-letter", "Draft offer letters and packages"),
            ("onboard-checklist", "Create onboarding checklists"),
            ("review-template", "Generate performance review templates"),
            ("policy-draft", "Draft HR policies and handbooks"),
            ("training-plan", "Create training program outlines"),
            ("survey-design", "Design employee engagement surveys"),
            ("exit-interview", "Generate exit interview questions"),
        ],
        AgentDomain::Marketing => vec![
            ("blog-post", "Write SEO-optimized blog posts"),
            ("social-post", "Create social media content"),
            ("email-campaign", "Design email campaign sequences"),
            ("ad-copy", "Write ad copy for multiple platforms"),
            ("landing-page", "Create landing page copy"),
            ("press-release", "Draft press releases"),
            ("case-study", "Write customer case studies"),
            ("brand-voice", "Define brand voice guidelines"),
            ("content-calendar", "Plan content calendars"),
            ("seo-audit", "Perform SEO keyword analysis"),
            ("video-script", "Write video scripts and storyboards"),
            ("newsletter", "Create newsletter content"),
            ("infographic", "Design infographic content outlines"),
            ("webinar-plan", "Plan webinar content and flow"),
        ],
        AgentDomain::Ecommerce => vec![
            ("product-desc", "Write product descriptions"),
            ("price-analysis", "Analyze pricing strategies"),
            ("inventory-alert", "Generate inventory alerts and forecasts"),
            ("cart-recovery", "Create cart abandonment recovery emails"),
            ("review-response", "Generate review response templates"),
            ("listing-optimize", "Optimize marketplace listings"),
            ("shipping-calc", "Design shipping rate structures"),
            ("promo-campaign", "Create promotional campaign copy"),
            ("category-taxonomy", "Design product category taxonomies"),
            ("upsell-rec", "Generate upsell recommendation logic"),
            ("return-policy", "Draft return and refund policies"),
        ],
        AgentDomain::ProjectManagement => vec![
            ("sprint-plan", "Create sprint planning documents"),
            ("risk-register", "Build risk assessment registers"),
            ("status-report", "Generate project status reports"),
            ("retrospective", "Facilitate retrospective templates"),
            ("gantt-plan", "Create project timeline plans"),
            ("raci-matrix", "Build RACI responsibility matrices"),
            ("scope-doc", "Write project scope documents"),
            ("meeting-agenda", "Generate meeting agendas and minutes"),
            ("burndown-analysis", "Analyze sprint burndown metrics"),
            ("dependency-map", "Map project dependencies"),
        ],
        AgentDomain::Legal => vec![
            ("contract-review", "Review contracts for risks and issues"),
            ("nda-gen", "Generate NDA templates"),
            ("compliance-check", "Perform regulatory compliance checks"),
            ("privacy-policy", "Draft privacy policies"),
            ("tos-gen", "Generate terms of service"),
            ("ip-filing", "Prepare IP filing documents"),
            ("dispute-letter", "Draft dispute resolution letters"),
            ("legal-memo", "Write legal memoranda"),
        ],
    }
}

/// Generate operational micro-agents for a given tactical specialist
pub fn generate_operational_agents(spec: &TacticalSpec) -> Vec<Agent> {
    let task_types = operational_task_types(spec.domain);
    let parent_id = format!("tac-{}-{}", spec.domain.slug(), spec.slug);

    (0..spec.prompt_count).map(|i| {
        let task = &task_types[i % task_types.len()];
        let variant = i / task_types.len();

        Agent {
            id: format!("op-{}-{}-{:04}", spec.domain.slug(), spec.slug, i),
            name: format!("{} #{} ({})", spec.name, i + 1, task.0),
            tier: AgentTier::Operational,
            domain: spec.domain,
            capabilities: vec![Capability::TextGeneration],
            prompt_template: format!(
                "You are Micro-Agent #{} under {} in the {} domain. \
                 Task: {}. Variant: {}. \
                 You execute one task with maximum precision. \
                 Report results to your tactical commander. Be concise, be sovereign.",
                i + 1, spec.name, spec.domain.label(), task.1, variant
            ),
            status: AgentStatus::Ready,
            connections: vec![parent_id.clone()],
            parent_id: Some(parent_id.clone()),
            tasks_completed: 0,
        }
    }).collect()
}

/// Generate all operational prompt seeds (lightweight, for DB bulk insert)
#[allow(dead_code)]
pub fn generate_operational_seeds() -> Vec<OperationalPromptSeed> {
    let specs = tactical_specializations();
    let mut seeds = Vec::with_capacity(3000);

    for spec in &specs {
        let task_types = operational_task_types(spec.domain);
        for i in 0..spec.prompt_count {
            let task = &task_types[i % task_types.len()];
            seeds.push(OperationalPromptSeed {
                specialization_slug: spec.slug.clone(),
                domain: spec.domain,
                index: i,
                task_type: task.0.to_string(),
                prompt_template: format!(
                    "You are Micro-Agent #{} under {} [{}]. Task: {}. Execute with precision.",
                    i + 1, spec.name, spec.domain.slug(), task.1
                ),
            });
        }
    }

    seeds
}

// === MCP Server Integration ===
// Model Context Protocol server for Claude Code and Gemini CLI

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub version: String,
    pub description: String,
    pub transport: McpTransport,
    pub capabilities: McpCapabilities,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTransport {
    Stdio,
    Http { host: String, port: u16 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCapabilities {
    pub tools: Vec<McpTool>,
    pub resources: Vec<McpResource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: String,
}

/// Build the MCP server configuration exposing fleet tools to Claude Code / Gemini CLI
pub fn build_mcp_config() -> McpServerConfig {
    McpServerConfig {
        name: "apophy-sovereign-fleet".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        description: "Divine Synarchy — 3000 agent fleet with 9 domains, 101 specialists, MCP-native".to_string(),
        transport: McpTransport::Http {
            host: "127.0.0.1".to_string(),
            port: 8080,
        },
        capabilities: McpCapabilities {
            tools: vec![
                McpTool {
                    name: "fleet_spawn".to_string(),
                    description: "Spawn agents into the fleet (all domains or filtered by domain/tier)".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "domain": { "type": "string", "enum": ["startups", "tech", "support", "sales", "hr", "marketing", "ecommerce", "pm", "legal"] },
                            "tier": { "type": "string", "enum": ["strategic", "tactical", "operational", "all"] }
                        }
                    }),
                },
                McpTool {
                    name: "fleet_status".to_string(),
                    description: "Get fleet summary: total agents, per-domain breakdown, synarchy status".to_string(),
                    input_schema: serde_json::json!({ "type": "object", "properties": {} }),
                },
                McpTool {
                    name: "fleet_catalog".to_string(),
                    description: "Get full catalog: all tiers, domains, capabilities, specializations (dropdown lists)".to_string(),
                    input_schema: serde_json::json!({ "type": "object", "properties": {} }),
                },
                McpTool {
                    name: "fleet_delegate".to_string(),
                    description: "Delegate a task to a specific agent or domain general".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "agent_id": { "type": "string", "description": "Target agent ID (e.g. gen-tech, tac-tech-frontend)" },
                            "task": { "type": "string", "description": "Task description to execute" },
                            "priority": { "type": "string", "enum": ["low", "normal", "high", "critical"] }
                        },
                        "required": ["agent_id", "task"]
                    }),
                },
                McpTool {
                    name: "fleet_search".to_string(),
                    description: "Search agents by domain, capability, or name pattern".to_string(),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "query": { "type": "string" },
                            "domain": { "type": "string" },
                            "capability": { "type": "string" },
                            "tier": { "type": "string" }
                        }
                    }),
                },
                McpTool {
                    name: "infra_status".to_string(),
                    description: "Get infrastructure health: Skool, AvatarVers, AgenticFlow, Gitea, Cloudflare, Email".to_string(),
                    input_schema: serde_json::json!({ "type": "object", "properties": {} }),
                },
            ],
            resources: vec![
                McpResource {
                    uri: "apophy://fleet/summary".to_string(),
                    name: "Fleet Summary".to_string(),
                    description: "Current fleet composition and synarchy status".to_string(),
                    mime_type: "application/json".to_string(),
                },
                McpResource {
                    uri: "apophy://fleet/catalog".to_string(),
                    name: "Fleet Catalog".to_string(),
                    description: "Complete catalog of all agent types, domains, capabilities".to_string(),
                    mime_type: "application/json".to_string(),
                },
                McpResource {
                    uri: "apophy://fleet/domains".to_string(),
                    name: "Domain Registry".to_string(),
                    description: "All 9 Outskill domains with tactical specializations".to_string(),
                    mime_type: "application/json".to_string(),
                },
                McpResource {
                    uri: "apophy://infra/status".to_string(),
                    name: "Infrastructure Status".to_string(),
                    description: "Service health for all connected infrastructure".to_string(),
                    mime_type: "application/json".to_string(),
                },
            ],
        },
    }
}

/// Generate MCP settings JSON for Claude Code (~/.claude/settings.json)
pub fn mcp_claude_code_settings() -> serde_json::Value {
    serde_json::json!({
        "mcpServers": {
            "apophy-sovereign": {
                "command": "apophy-sovereign",
                "args": ["mcp-serve"],
                "env": {}
            }
        }
    })
}

/// Generate MCP settings for Gemini CLI (~/.gemini/settings.json)
pub fn mcp_gemini_cli_settings() -> serde_json::Value {
    serde_json::json!({
        "mcpServers": [{
            "name": "apophy-sovereign",
            "uri": "http://127.0.0.1:8080/api/v1/mcp",
            "description": "Divine Synarchy Fleet — 3000 agent army"
        }]
    })
}
