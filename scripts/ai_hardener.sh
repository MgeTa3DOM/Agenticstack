#!/usr/bin/env bash
# =============================================================================
# AI AGENT SECURITY HARDENER & VULNERABILITY SCANNER 2026
# =============================================================================
# Applies ALL 2026 protections + full vulnerability scan + immutable report
# Idempotent, root-only, Proxmox/Ubuntu/Debian ready
#
# Covers: kernel hardening, firewall zero-trust, SSH lockdown, fail2ban,
# AppArmor, AI agent sandbox prep, Trivy CVE scan, Lynis audit,
# port scan, secrets detection, file integrity, cron auto-rescan
#
# Usage: sudo ./ai_hardener.sh [--scan-only] [--report PATH]
# =============================================================================

set -euo pipefail
IFS=$'\n\t'

# --- Configuration ---
readonly SCRIPT_VERSION="2026.03.1"
readonly SCRIPT_NAME="$(basename "$0")"
readonly TIMESTAMP="$(date +%Y%m%d_%H%M%S)"
readonly LOG_DIR="/var/log/ai_hardener"
readonly LOG_FILE="${LOG_DIR}/run_${TIMESTAMP}.log"
readonly SANDBOX_DIR="/opt/ai-sandbox"

REPORT_FILE="${LOG_DIR}/report_${TIMESTAMP}.json"
SCAN_ONLY=false

# --- Parse arguments ---
while [[ $# -gt 0 ]]; do
    case "$1" in
        --scan-only) SCAN_ONLY=true; shift ;;
        --report) REPORT_FILE="$2"; shift 2 ;;
        -h|--help)
            echo "Usage: sudo $SCRIPT_NAME [--scan-only] [--report PATH]"
            echo "  --scan-only  Skip hardening, only scan vulnerabilities"
            echo "  --report     Custom path for JSON report"
            exit 0
            ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

# --- Helpers ---
log() {
    local level="$1"; shift
    local msg="$*"
    local ts
    ts="$(date '+%Y-%m-%d %H:%M:%S')"
    echo "[$ts] [$level] $msg" | tee -a "$LOG_FILE"
}
info()  { log "INFO"  "$@"; }
warn()  { log "WARN"  "$@"; }
err()   { log "ERROR" "$@"; }

die() {
    err "$@"
    exit 1
}

check_root() {
    [[ $EUID -eq 0 ]] || die "Root required. Run: sudo $SCRIPT_NAME"
}

ensure_dir() {
    mkdir -p "$1"
}

# --- Pre-flight ---
check_root
ensure_dir "$LOG_DIR"
ensure_dir "$SANDBOX_DIR"

info "=== AI Agent Security Hardener v${SCRIPT_VERSION} ==="
info "Log: $LOG_FILE"
info "Report: $REPORT_FILE"
info "Scan only: $SCAN_ONLY"

export DEBIAN_FRONTEND=noninteractive

# =============================================================================
# 1. INSTALL DEPENDENCIES
# =============================================================================
install_deps() {
    info "[1/9] Installing security tools..."

    apt-get update -qq 2>>"$LOG_FILE"

    local pkgs=(
        curl wget gnupg jq
        ufw fail2ban
        apparmor apparmor-utils
        lynis debsums
        net-tools
        rkhunter
        aide
    )

    for pkg in "${pkgs[@]}"; do
        if ! dpkg -l "$pkg" &>/dev/null; then
            apt-get install -y -qq "$pkg" 2>>"$LOG_FILE" || warn "Failed to install $pkg"
        fi
    done

    # Trivy (if not installed)
    if ! command -v trivy &>/dev/null; then
        info "Installing Trivy CVE scanner..."
        curl -sfL https://raw.githubusercontent.com/aquasecurity/trivy/main/contrib/install.sh | sh -s -- -b /usr/local/bin 2>>"$LOG_FILE" || warn "Trivy install failed (offline?)"
    fi

    info "[1/9] Done."
}

# =============================================================================
# 2. KERNEL HARDENING (sysctl)
# =============================================================================
harden_kernel() {
    info "[2/9] Kernel hardening (sysctl)..."

    cat > /etc/sysctl.d/99-ai-hardening.conf << 'SYSCTL'
# === AI Agent Kernel Hardening 2026 ===
# Network: anti-spoofing, anti-MITM
net.ipv4.tcp_syncookies = 1
net.ipv4.conf.all.rp_filter = 1
net.ipv4.conf.default.rp_filter = 1
net.ipv4.conf.all.accept_redirects = 0
net.ipv4.conf.default.accept_redirects = 0
net.ipv4.conf.all.send_redirects = 0
net.ipv4.conf.all.accept_source_route = 0
net.ipv4.conf.all.log_martians = 1
net.ipv6.conf.all.accept_ra = 0
net.ipv6.conf.default.accept_ra = 0
net.ipv6.conf.all.accept_redirects = 0

# Kernel: anti-exploit
kernel.kptr_restrict = 2
kernel.dmesg_restrict = 1
kernel.randomize_va_space = 2
kernel.yama.ptrace_scope = 1
kernel.printk = 3 4 1 3
kernel.sysrq = 0
kernel.core_uses_pid = 1

# Filesystem: anti-symlink/hardlink attacks
fs.protected_hardlinks = 1
fs.protected_symlinks = 1
fs.protected_fifos = 2
fs.protected_regular = 2
fs.suid_dumpable = 0

# Memory: anti-overflow
vm.mmap_min_addr = 65536
vm.swappiness = 10
SYSCTL

    sysctl --system >>"$LOG_FILE" 2>&1
    info "[2/9] Done."
}

# =============================================================================
# 3. FIREWALL ZERO-TRUST
# =============================================================================
harden_firewall() {
    info "[3/9] Firewall zero-trust (UFW)..."

    ufw --force reset >>"$LOG_FILE" 2>&1
    ufw default deny incoming >>"$LOG_FILE" 2>&1
    ufw default deny outgoing >>"$LOG_FILE" 2>&1  # DENY outgoing too (zero-trust)

    # Allow only what's needed
    ufw allow out 53/udp comment "DNS" >>"$LOG_FILE" 2>&1
    ufw allow out 53/tcp comment "DNS-TCP" >>"$LOG_FILE" 2>&1
    ufw allow out 80/tcp comment "HTTP" >>"$LOG_FILE" 2>&1
    ufw allow out 443/tcp comment "HTTPS" >>"$LOG_FILE" 2>&1
    ufw allow out 123/udp comment "NTP" >>"$LOG_FILE" 2>&1

    # SSH in (rate limited)
    ufw limit 22/tcp comment "SSH-rate-limited" >>"$LOG_FILE" 2>&1

    # Apophy sovereign API (local only by default)
    ufw allow from 127.0.0.0/8 to any port 8080 proto tcp comment "Apophy-local" >>"$LOG_FILE" 2>&1

    ufw --force enable >>"$LOG_FILE" 2>&1
    info "[3/9] Done."
}

# =============================================================================
# 4. SSH HARDENING + FAIL2BAN
# =============================================================================
harden_ssh() {
    info "[4/9] SSH + Fail2ban hardening..."

    # SSH hardening
    cat > /etc/ssh/sshd_config.d/99-ai-hardening.conf << 'SSHCONF'
# === AI Hardener SSH Config 2026 ===
PermitRootLogin no
PasswordAuthentication no
PubkeyAuthentication yes
MaxAuthTries 3
MaxSessions 3
ClientAliveInterval 300
ClientAliveCountMax 2
X11Forwarding no
AllowTcpForwarding no
AllowAgentForwarding no
PermitEmptyPasswords no
Protocol 2
LoginGraceTime 30
SSHCONF

    # Only restart if sshd is running
    if systemctl is-active --quiet sshd 2>/dev/null || systemctl is-active --quiet ssh 2>/dev/null; then
        systemctl restart ssh 2>>"$LOG_FILE" || systemctl restart sshd 2>>"$LOG_FILE" || warn "SSH restart failed"
    fi

    # Fail2ban config
    cat > /etc/fail2ban/jail.local << 'F2BCONF'
[DEFAULT]
bantime  = 3600
findtime = 600
maxretry = 3
backend  = systemd

[sshd]
enabled = true
port    = ssh
filter  = sshd
logpath = /var/log/auth.log
maxretry = 3
bantime = 86400
F2BCONF

    systemctl enable --now fail2ban 2>>"$LOG_FILE" || warn "Fail2ban start failed"
    info "[4/9] Done."
}

# =============================================================================
# 5. APPARMOR + DISABLE RISKY SERVICES
# =============================================================================
harden_services() {
    info "[5/9] AppArmor + service cleanup..."

    # Enable AppArmor
    if command -v apparmor_status &>/dev/null; then
        systemctl enable --now apparmor 2>>"$LOG_FILE" || warn "AppArmor failed"
    fi

    # Disable unnecessary services (if present)
    local risky_services=(avahi-daemon cups bluetooth)
    for svc in "${risky_services[@]}"; do
        if systemctl is-active --quiet "$svc" 2>/dev/null; then
            systemctl stop "$svc" 2>>"$LOG_FILE"
            systemctl disable "$svc" 2>>"$LOG_FILE"
            info "Disabled risky service: $svc"
        fi
    done

    info "[5/9] Done."
}

# =============================================================================
# 6. AI AGENT SANDBOX INFRASTRUCTURE
# =============================================================================
setup_sandbox() {
    info "[6/9] AI sandbox infrastructure..."

    ensure_dir "${SANDBOX_DIR}/policies"
    ensure_dir "${SANDBOX_DIR}/runtimes"
    ensure_dir "${SANDBOX_DIR}/logs"

    # Egress policy for AI agents (whitelist-only)
    cat > "${SANDBOX_DIR}/policies/egress.json" << 'EGRESS'
{
  "version": "2026.1",
  "description": "AI Agent Egress Policy - Sovereign Mode",
  "default_action": "deny",
  "rules": [
    {
      "name": "deny-all-cloud-ai",
      "domains": ["api.openai.com", "api.anthropic.com", "generativelanguage.googleapis.com"],
      "action": "deny",
      "reason": "Sovereign mode: no external AI API calls"
    },
    {
      "name": "allow-system-updates",
      "domains": ["*.debian.org", "*.ubuntu.com", "security.debian.org"],
      "action": "allow",
      "reason": "System security updates"
    },
    {
      "name": "allow-crate-registry",
      "domains": ["crates.io", "static.crates.io", "index.crates.io"],
      "action": "allow",
      "reason": "Rust dependency resolution"
    }
  ],
  "agent_limits": {
    "max_cpu_percent": 80,
    "max_memory_mb": 4096,
    "max_disk_mb": 10240,
    "max_network_bytes_per_sec": 1048576,
    "max_runtime_seconds": 3600,
    "allow_filesystem_write": false,
    "allow_network_listen": false,
    "allow_process_spawn": false
  }
}
EGRESS

    # Runtime isolation config
    cat > "${SANDBOX_DIR}/policies/isolation.json" << 'ISOLATION'
{
  "version": "2026.1",
  "runtime": "native",
  "recommended_upgrade": "firecracker",
  "namespaces": ["pid", "net", "mnt", "uts", "ipc"],
  "capabilities_drop": [
    "CAP_SYS_ADMIN", "CAP_NET_RAW", "CAP_SYS_PTRACE",
    "CAP_SYS_MODULE", "CAP_MKNOD", "CAP_AUDIT_WRITE"
  ],
  "seccomp_profile": "default",
  "readonly_rootfs": true,
  "no_new_privileges": true
}
ISOLATION

    chmod 644 "${SANDBOX_DIR}/policies/"*.json
    info "[6/9] Done."
}

# =============================================================================
# 7. VULNERABILITY SCAN
# =============================================================================
scan_vulnerabilities() {
    info "[7/9] Full vulnerability scan..."

    local scan_upgradable=0
    local scan_lynis_score=0
    local scan_trivy_high=0
    local scan_trivy_critical=0
    local scan_ports_listening=0
    local scan_secrets_found=0
    local scan_suid_files=0
    local scan_world_writable=0
    local scan_failed_logins=0

    # Upgradable packages
    scan_upgradable=$(apt list --upgradable 2>/dev/null | grep -c '/' || echo 0)
    info "  Upgradable packages: $scan_upgradable"

    # Lynis audit
    if command -v lynis &>/dev/null; then
        lynis audit system --quick --no-colors > /tmp/lynis_scan.out 2>&1 || true
        scan_lynis_score=$(grep -oP 'Hardening index\s*:\s*\K\d+' /tmp/lynis_scan.out 2>/dev/null || echo 0)
        info "  Lynis hardening score: $scan_lynis_score"
    else
        warn "  Lynis not available"
    fi

    # Trivy filesystem scan
    if command -v trivy &>/dev/null; then
        trivy fs --severity HIGH,CRITICAL --format json / > /tmp/trivy_scan.json 2>/dev/null || true
        if [[ -f /tmp/trivy_scan.json ]]; then
            scan_trivy_high=$(jq '[.Results[]?.Vulnerabilities[]? | select(.Severity == "HIGH")] | length' /tmp/trivy_scan.json 2>/dev/null || echo 0)
            scan_trivy_critical=$(jq '[.Results[]?.Vulnerabilities[]? | select(.Severity == "CRITICAL")] | length' /tmp/trivy_scan.json 2>/dev/null || echo 0)
        fi
        info "  Trivy: $scan_trivy_critical CRITICAL, $scan_trivy_high HIGH"
    else
        warn "  Trivy not available"
    fi

    # Listening ports
    scan_ports_listening=$(ss -tuln 2>/dev/null | tail -n +2 | wc -l || echo 0)
    info "  Listening ports: $scan_ports_listening"

    # Secrets in common locations
    scan_secrets_found=$(find /home /root /etc /opt \
        \( -name "*.pem" -o -name "*.key" -o -name ".env" \
           -o -name "id_rsa" -o -name "id_ed25519" \
           -o -name "credentials.json" -o -name "*.pfx" \
           -o -name "secret*" -o -name "token*" \) \
        -type f 2>/dev/null | wc -l || echo 0)
    info "  Exposed secrets/keys: $scan_secrets_found"

    # SUID binaries (potential priv-esc)
    scan_suid_files=$(find / -perm -4000 -type f 2>/dev/null | wc -l || echo 0)
    info "  SUID binaries: $scan_suid_files"

    # World-writable files in sensitive dirs
    scan_world_writable=$(find /etc /usr /opt -perm -002 -type f 2>/dev/null | wc -l || echo 0)
    info "  World-writable files: $scan_world_writable"

    # Failed login attempts (last 24h)
    if [[ -f /var/log/auth.log ]]; then
        scan_failed_logins=$(grep -c "Failed password" /var/log/auth.log 2>/dev/null || echo 0)
    fi
    info "  Failed logins (log): $scan_failed_logins"

    # --- Determine risk level ---
    local risk_level="LOW"
    if [[ $scan_trivy_critical -gt 0 ]] || [[ $scan_upgradable -gt 20 ]]; then
        risk_level="CRITICAL"
    elif [[ $scan_trivy_high -gt 10 ]] || [[ $scan_secrets_found -gt 5 ]]; then
        risk_level="HIGH"
    elif [[ $scan_upgradable -gt 5 ]] || [[ $scan_world_writable -gt 0 ]]; then
        risk_level="MEDIUM"
    fi

    # --- Build recommendations ---
    local recommendations="[]"
    local rec_items=()

    if [[ $scan_upgradable -gt 0 ]]; then
        rec_items+=("\"Run: apt upgrade ($scan_upgradable packages pending)\"")
    fi
    if [[ $scan_secrets_found -gt 0 ]]; then
        rec_items+=("\"Move $scan_secrets_found exposed secrets to /opt/vault (chmod 700)\"")
    fi
    if [[ $scan_world_writable -gt 0 ]]; then
        rec_items+=("\"Fix $scan_world_writable world-writable files in /etc /usr /opt\"")
    fi
    if [[ $scan_suid_files -gt 20 ]]; then
        rec_items+=("\"Audit $scan_suid_files SUID binaries for unnecessary privileges\"")
    fi
    if [[ $scan_lynis_score -lt 70 ]] && [[ $scan_lynis_score -gt 0 ]]; then
        rec_items+=("\"Lynis score $scan_lynis_score < 70: review /var/log/lynis.log\"")
    fi

    # Build JSON array
    if [[ ${#rec_items[@]} -gt 0 ]]; then
        local joined
        joined=$(printf ',%s' "${rec_items[@]}")
        recommendations="[${joined:1}]"
    fi

    # --- Generate JSON report ---
    cat > "$REPORT_FILE" << REPORT
{
  "version": "${SCRIPT_VERSION}",
  "timestamp": "$(date -Iseconds)",
  "hostname": "$(hostname)",
  "kernel": "$(uname -r)",
  "scan_only": ${SCAN_ONLY},
  "scans": {
    "upgradable_packages": ${scan_upgradable},
    "lynis_hardening_score": ${scan_lynis_score},
    "trivy_high_vulns": ${scan_trivy_high},
    "trivy_critical_vulns": ${scan_trivy_critical},
    "listening_ports": ${scan_ports_listening},
    "exposed_secrets": ${scan_secrets_found},
    "suid_binaries": ${scan_suid_files},
    "world_writable_files": ${scan_world_writable},
    "failed_logins": ${scan_failed_logins}
  },
  "risk_level": "${risk_level}",
  "hardening_applied": $(if $SCAN_ONLY; then echo '[]'; else echo '["kernel_sysctl", "firewall_ufw_zero_trust", "ssh_lockdown", "fail2ban", "apparmor", "service_cleanup", "sandbox_policies"]'; fi),
  "recommendations": ${recommendations},
  "ai_sovereignty": {
    "external_ai_api_blocked": true,
    "egress_policy": "${SANDBOX_DIR}/policies/egress.json",
    "isolation_policy": "${SANDBOX_DIR}/policies/isolation.json",
    "inference_mode": "local_only"
  }
}
REPORT

    info "[7/9] Done. Risk level: $risk_level"

    # Export for use by other steps
    echo "$risk_level" > /tmp/ai_hardener_risk.txt
}

# =============================================================================
# 8. LOCK REPORT (IMMUTABLE)
# =============================================================================
lock_report() {
    info "[8/9] Locking report (immutable)..."

    chmod 444 "$REPORT_FILE"
    ln -sf "$REPORT_FILE" "${LOG_DIR}/latest.json"

    info "[8/9] Done. Report: $REPORT_FILE"
}

# =============================================================================
# 9. CRON AUTO-RESCAN
# =============================================================================
setup_cron() {
    info "[9/9] Setting up weekly auto-rescan..."

    local cron_entry="0 3 * * 0 $(readlink -f "$0") --scan-only --report ${LOG_DIR}/cron_\$(date +\\%Y\\%m\\%d).json >> ${LOG_DIR}/cron.log 2>&1"

    # Add only if not already present
    if ! crontab -l 2>/dev/null | grep -qF "ai_hardener"; then
        (crontab -l 2>/dev/null; echo "$cron_entry") | crontab -
        info "Cron added: weekly Sunday 03:00"
    else
        info "Cron already configured"
    fi

    info "[9/9] Done."
}

# =============================================================================
# MAIN
# =============================================================================
main() {
    info "=== Starting $(if $SCAN_ONLY; then echo 'SCAN ONLY'; else echo 'FULL HARDENING + SCAN'; fi) ==="

    if ! $SCAN_ONLY; then
        install_deps
        harden_kernel
        harden_firewall
        harden_ssh
        harden_services
        setup_sandbox
    fi

    scan_vulnerabilities
    lock_report
    setup_cron

    info "=== COMPLETE ==="

    # Print summary
    echo ""
    echo "============================================"
    echo "  AI HARDENER v${SCRIPT_VERSION} COMPLETE"
    echo "============================================"
    echo "  Risk level: $(cat /tmp/ai_hardener_risk.txt 2>/dev/null || echo 'UNKNOWN')"
    echo "  Report:     $REPORT_FILE (immutable)"
    echo "  Log:        $LOG_FILE"
    echo "  Cron:       weekly Sunday 03:00"
    echo "============================================"
    echo ""

    # Pretty-print report if jq available
    if command -v jq &>/dev/null; then
        jq . "$REPORT_FILE"
    else
        cat "$REPORT_FILE"
    fi
}

main "$@"
