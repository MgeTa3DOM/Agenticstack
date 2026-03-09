#!/usr/bin/env bash
# =============================================================================
# Apophy Sovereign — Auto-Deploy Pipeline
# Self-deploying, self-maintaining sovereign AI stack
# =============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
LOG_DIR="$PROJECT_ROOT/data/logs"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
LOG_FILE="$LOG_DIR/deploy_$TIMESTAMP.log"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Ensure log directory exists
mkdir -p "$LOG_DIR"

log() { echo -e "${BLUE}[$(date +%H:%M:%S)]${NC} $1" | tee -a "$LOG_FILE"; }
ok()  { echo -e "${GREEN}[OK]${NC} $1" | tee -a "$LOG_FILE"; }
warn(){ echo -e "${YELLOW}[WARN]${NC} $1" | tee -a "$LOG_FILE"; }
fail(){ echo -e "${RED}[FAIL]${NC} $1" | tee -a "$LOG_FILE"; }

# =============================================================================
# Stages
# =============================================================================

stage_lint() {
    log "Stage: LINT"
    cd "$PROJECT_ROOT"
    if cargo fmt --check 2>> "$LOG_FILE"; then
        ok "Format check passed"
    else
        warn "Format issues found — auto-fixing"
        cargo fmt 2>> "$LOG_FILE"
    fi

    if cargo clippy --workspace -- -D warnings 2>> "$LOG_FILE"; then
        ok "Clippy passed"
    else
        warn "Clippy warnings found"
        return 1
    fi
}

stage_build() {
    log "Stage: BUILD"
    cd "$PROJECT_ROOT"
    if cargo build --workspace --release 2>> "$LOG_FILE"; then
        ok "Build succeeded"
    else
        fail "Build failed"
        return 1
    fi
}

stage_test() {
    log "Stage: TEST"
    cd "$PROJECT_ROOT"
    local test_output
    test_output=$(cargo test --workspace 2>&1) || {
        fail "Tests failed"
        echo "$test_output" >> "$LOG_FILE"
        return 1
    }
    local test_count
    test_count=$(echo "$test_output" | grep -c "test .* ok" || true)
    ok "Tests passed ($test_count suites)"
    echo "$test_output" >> "$LOG_FILE"
}

stage_doc() {
    log "Stage: DOC"
    cd "$PROJECT_ROOT"
    if [ -f "prime.js" ] && command -v node &> /dev/null; then
        node prime.js --doctor 2>> "$LOG_FILE" && ok "Documentation updated"
    else
        warn "Skipping doc generation (prime.js or node not found)"
    fi
}

stage_deploy_docker() {
    log "Stage: DEPLOY (Docker)"
    cd "$PROJECT_ROOT"
    if command -v docker &> /dev/null && [ -f "docker-compose.yml" ]; then
        docker compose build 2>> "$LOG_FILE" && \
        docker compose up -d 2>> "$LOG_FILE" && \
        ok "Docker deployment succeeded"
    else
        warn "Docker not available — skipping container deploy"
    fi
}

stage_health() {
    log "Stage: HEALTH CHECK"
    local health_url="${HEALTH_URL:-http://localhost:8080/health}"
    local max_attempts=5
    local attempt=1

    while [ $attempt -le $max_attempts ]; do
        if curl -sf "$health_url" > /dev/null 2>&1; then
            ok "Health check passed (attempt $attempt)"
            return 0
        fi
        log "  Attempt $attempt/$max_attempts — waiting..."
        sleep 2
        attempt=$((attempt + 1))
    done

    fail "Health check failed after $max_attempts attempts"
    return 1
}

stage_recover() {
    log "Stage: AUTO-RECOVER"
    cd "$PROJECT_ROOT"

    # Try restart
    if command -v docker &> /dev/null && [ -f "docker-compose.yml" ]; then
        log "  Restarting containers..."
        docker compose restart 2>> "$LOG_FILE"
        sleep 5

        if stage_health 2>/dev/null; then
            ok "Recovery succeeded via container restart"
            return 0
        fi
    fi

    # Try rebuild
    log "  Attempting full rebuild..."
    if stage_build && stage_deploy_docker; then
        sleep 5
        if stage_health 2>/dev/null; then
            ok "Recovery succeeded via rebuild + deploy"
            return 0
        fi
    fi

    fail "Auto-recovery exhausted"
    return 1
}

stage_dataset_sync() {
    log "Stage: DATASET SYNC"
    cd "$PROJECT_ROOT"
    local dataset_dir="$PROJECT_ROOT/data/datasets"
    mkdir -p "$dataset_dir"

    # Check for AZR self-play data to export
    if [ -f "$PROJECT_ROOT/target/release/apophy-sovereign" ]; then
        log "  Collecting self-play episodes..."
        ok "Dataset sync ready (use API: POST /api/v1/brain/self-play)"
    else
        warn "Binary not built — skip dataset sync"
    fi
}

# =============================================================================
# Pipeline modes
# =============================================================================

pipeline_full() {
    log "=== FULL PIPELINE ==="
    log "Project: $PROJECT_ROOT"
    log "Log: $LOG_FILE"

    local failed=0

    stage_lint    || failed=1
    stage_build   || failed=1
    stage_test    || failed=1
    stage_doc     || true  # Non-critical
    stage_deploy_docker || failed=1

    if [ $failed -eq 0 ]; then
        stage_health || {
            warn "Health check failed — attempting recovery"
            stage_recover || failed=1
        }
    fi

    stage_dataset_sync || true  # Non-critical

    if [ $failed -eq 0 ]; then
        ok "=== PIPELINE SUCCEEDED ==="
    else
        fail "=== PIPELINE FAILED ==="
        return 1
    fi
}

pipeline_quick() {
    log "=== QUICK PIPELINE ==="
    stage_build || return 1
    stage_test  || return 1
    ok "=== QUICK PIPELINE SUCCEEDED ==="
}

pipeline_deploy() {
    log "=== DEPLOY ONLY ==="
    stage_deploy_docker || return 1
    stage_health        || stage_recover || return 1
    ok "=== DEPLOY SUCCEEDED ==="
}

pipeline_maintain() {
    log "=== MAINTENANCE CYCLE ==="
    stage_health 2>/dev/null || {
        warn "Service unhealthy — triggering recovery"
        stage_recover || return 1
    }
    stage_dataset_sync || true
    ok "=== MAINTENANCE COMPLETE ==="
}

# =============================================================================
# Main
# =============================================================================

usage() {
    cat << 'EOF'
Apophy Sovereign — Auto-Deploy Pipeline

Usage: autodeploy.sh [MODE]

Modes:
  full       Full pipeline: lint → build → test → doc → deploy → health
  quick      Quick check: build → test
  deploy     Deploy only: docker compose → health check
  maintain   Maintenance: health check → recover if needed → dataset sync
  lint       Lint only
  build      Build only
  test       Test only
  health     Health check only
  recover    Force recovery

Environment:
  HEALTH_URL   Health check endpoint (default: http://localhost:8080/health)

Examples:
  ./tools/autodeploy.sh full
  ./tools/autodeploy.sh quick
  HEALTH_URL=http://prod:8080/health ./tools/autodeploy.sh maintain
EOF
}

case "${1:-full}" in
    full)     pipeline_full ;;
    quick)    pipeline_quick ;;
    deploy)   pipeline_deploy ;;
    maintain) pipeline_maintain ;;
    lint)     stage_lint ;;
    build)    stage_build ;;
    test)     stage_test ;;
    doc)      stage_doc ;;
    health)   stage_health ;;
    recover)  stage_recover ;;
    dataset)  stage_dataset_sync ;;
    help|-h|--help) usage ;;
    *) fail "Unknown mode: $1"; usage; exit 1 ;;
esac
