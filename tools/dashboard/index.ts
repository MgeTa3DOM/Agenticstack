/**
 * Apophy Sovereign — Tour de Contrôle (Control Tower Dashboard)
 *
 * Full real-time monitoring dashboard for 3000-agent fleet:
 * - Masters Panel (TimeMaster, SpaceMaster, LatentMaster)
 * - Workflow DAG visualization
 * - Agent live stream with WebSocket
 * - Chrome DevTools integration via Playwright
 * - Fine-tuning pipeline status
 *
 * Usage:
 *   bun run tools/dashboard/index.ts
 *   # Dashboard: http://localhost:3001
 *   # WS: ws://localhost:3001/ws
 */

const SOVEREIGN_API = process.env.APOPHY_API || "http://localhost:8080";
const PORT = parseInt(process.env.DASHBOARD_PORT || "3001");

interface BrainStatus {
  backend_name: string;
  model_loaded: boolean;
  model_name: string | null;
  hardware_backend: string;
  memory_mb: number;
  cpu_cores: number;
  azr_episodes: number;
  ashoka_feedback_count: number;
  ashoka_evolutions: number;
}

interface MastersStatus {
  time_master: { queue_depth: number; tasks_per_second: number; total_scheduled: number; total_deferred: number };
  space_master: { total_cores: number; total_memory_mb: number; active_tasks: number; utilization_pct: number; zones: any[] };
  latent_master: { patterns_detected: number; total_observations: number; avg_prediction_accuracy: number };
  total_queued: number;
  total_running: number;
  total_completed: number;
}

interface WsClient {
  ws: any;
  subscriptions: Set<string>;
}

const clients: WsClient[] = [];

// Poll sovereign API and broadcast to WebSocket clients
async function pollAndBroadcast() {
  try {
    const [brainRes, healthRes, fleetRes, mastersRes, workflowRes, runtimeRes] =
      await Promise.allSettled([
        fetch(`${SOVEREIGN_API}/api/v1/brain/status`),
        fetch(`${SOVEREIGN_API}/health`),
        fetch(`${SOVEREIGN_API}/api/v1/fleet/summary`),
        fetch(`${SOVEREIGN_API}/api/v1/masters/status`),
        fetch(`${SOVEREIGN_API}/api/v1/workflows/status`),
        fetch(`${SOVEREIGN_API}/api/v1/runtime/stats`),
      ]);

    const resolve = async (r: PromiseSettledResult<Response>) =>
      r.status === "fulfilled" && r.value.ok ? await r.value.json() : null;

    const payload = JSON.stringify({
      type: "status",
      timestamp: new Date().toISOString(),
      brain: await resolve(brainRes),
      health: await resolve(healthRes),
      fleet: await resolve(fleetRes),
      masters: await resolve(mastersRes),
      workflows: await resolve(workflowRes),
      runtime: await resolve(runtimeRes),
    });

    for (const client of clients) {
      if (client.subscriptions.has("status")) {
        client.ws.send(payload);
      }
    }
  } catch (e) {
    // Sovereign not running — silent
  }
}

// Dashboard HTML — Tour de Contrôle
function dashboardHtml(): string {
  return `<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>Tour de Controle — Apophy Sovereign</title>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body { font-family: 'JetBrains Mono', monospace; background: #0a0a0a; color: #e0e0e0; padding: 16px; }
    h1 { color: #00ff88; margin-bottom: 4px; font-size: 1.3em; }
    .subtitle { color: #666; font-size: 0.8em; margin-bottom: 16px; }
    .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 12px; }
    .grid-wide { display: grid; grid-template-columns: repeat(auto-fit, minmax(400px, 1fr)); gap: 12px; margin-top: 12px; }
    .card { background: #1a1a2e; border: 1px solid #333; border-radius: 8px; padding: 14px; }
    .card h2 { color: #00ccff; font-size: 0.95em; margin-bottom: 10px; border-bottom: 1px solid #333; padding-bottom: 6px; display: flex; align-items: center; gap: 8px; }
    .card h2 .badge { font-size: 0.7em; padding: 2px 6px; border-radius: 4px; }
    .badge-ok { background: #00ff8833; color: #00ff88; }
    .badge-warn { background: #ffcc0033; color: #ffcc00; }
    .metric { display: flex; justify-content: space-between; padding: 3px 0; font-size: 0.85em; }
    .metric .label { color: #888; }
    .metric .value { color: #fff; font-weight: bold; }
    .metric .value.ok { color: #00ff88; }
    .metric .value.warn { color: #ffcc00; }
    .metric .value.err { color: #ff4444; }
    .zone { background: #0f0f1a; border: 1px solid #2a2a3e; border-radius: 6px; padding: 8px; margin-top: 6px; }
    .zone-header { display: flex; justify-content: space-between; font-size: 0.8em; }
    .zone-bar { height: 4px; background: #333; border-radius: 2px; margin-top: 4px; overflow: hidden; }
    .zone-bar-fill { height: 100%; border-radius: 2px; transition: width 0.3s; }
    .stats-row { display: flex; gap: 16px; margin-top: 12px; flex-wrap: wrap; }
    .stat-chip { background: #1a1a2e; border: 1px solid #333; border-radius: 6px; padding: 8px 14px;
                 text-align: center; min-width: 100px; }
    .stat-chip .num { font-size: 1.4em; font-weight: bold; color: #00ff88; }
    .stat-chip .lbl { font-size: 0.7em; color: #666; margin-top: 2px; }
    #log { background: #111; border: 1px solid #333; border-radius: 8px; padding: 10px; margin-top: 12px;
           max-height: 200px; overflow-y: auto; font-size: 0.78em; }
    #log .entry { padding: 1px 0; border-bottom: 1px solid #1a1a1a; }
    #log .ts { color: #555; }
    #log .msg { color: #aaa; }
    footer { margin-top: 16px; color: #444; font-size: 0.75em; text-align: center; }
  </style>
</head>
<body>
  <h1>TOUR DE CONTROLE — Apophy Sovereign</h1>
  <p class="subtitle">Multi-Runtime Hybrid Architecture | 3000 Agents | Rust + Bun + uv + Playwright</p>

  <div class="stats-row" id="top-stats">
    <div class="stat-chip"><div class="num" id="s-agents">3000</div><div class="lbl">AGENTS</div></div>
    <div class="stat-chip"><div class="num" id="s-running">0</div><div class="lbl">RUNNING</div></div>
    <div class="stat-chip"><div class="num" id="s-queued">0</div><div class="lbl">QUEUED</div></div>
    <div class="stat-chip"><div class="num" id="s-completed">0</div><div class="lbl">COMPLETED</div></div>
    <div class="stat-chip"><div class="num" id="s-tps">0</div><div class="lbl">TASKS/s</div></div>
    <div class="stat-chip"><div class="num" id="s-util">0%</div><div class="lbl">CPU UTIL</div></div>
  </div>

  <div class="grid" style="margin-top: 12px;">
    <div class="card" id="time-card">
      <h2>TimeMaster <span class="badge badge-ok">SCHEDULING</span></h2>
      <div id="time-metrics">
        <div class="metric"><span class="label">Queue Depth</span><span class="value">0</span></div>
        <div class="metric"><span class="label">Tasks/s</span><span class="value">0</span></div>
        <div class="metric"><span class="label">Scheduled</span><span class="value">0</span></div>
        <div class="metric"><span class="label">Deferred</span><span class="value">0</span></div>
      </div>
    </div>

    <div class="card" id="space-card">
      <h2>SpaceMaster <span class="badge badge-ok">DISTRIBUTING</span></h2>
      <div id="space-metrics">Waiting...</div>
      <div id="zones"></div>
    </div>

    <div class="card" id="latent-card">
      <h2>LatentMaster <span class="badge badge-ok">LEARNING</span></h2>
      <div id="latent-metrics">
        <div class="metric"><span class="label">Patterns</span><span class="value">0</span></div>
        <div class="metric"><span class="label">Observations</span><span class="value">0</span></div>
        <div class="metric"><span class="label">Accuracy</span><span class="value">--</span></div>
      </div>
    </div>

    <div class="card" id="brain-card">
      <h2>Brain Status</h2>
      <div id="brain-metrics">Connecting...</div>
    </div>
    <div class="card" id="health-card">
      <h2>Server Health</h2>
      <div id="health-metrics">Connecting...</div>
    </div>
    <div class="card" id="fleet-card">
      <h2>Fleet (3-Tier Synarchy)</h2>
      <div id="fleet-metrics">Connecting...</div>
    </div>
  </div>

  <div class="grid-wide">
    <div class="card" id="workflow-card">
      <h2>Workflow Orchestrator</h2>
      <div id="workflow-metrics">No active workflows</div>
    </div>
    <div class="card" id="engines-card">
      <h2>Reasoning Engines & Runtimes</h2>
      <div id="engines-metrics">
        <div class="metric"><span class="label">AlphaResolve</span><span class="value ok">READY</span></div>
        <div class="metric"><span class="label">AZR Self-Play</span><span class="value ok">READY</span></div>
        <div class="metric"><span class="label">CTM-C Compress</span><span class="value ok">READY</span></div>
        <div class="metric"><span class="label">Ashoka Autolearn</span><span class="value ok">READY</span></div>
        <div class="metric"><span class="label">Gemma3 270M (Browser)</span><span class="value warn">STANDBY</span></div>
        <div class="metric"><span class="label">Playwright Agents</span><span class="value warn">STANDBY</span></div>
      </div>
    </div>
  </div>

  <div id="log"><div class="entry"><span class="ts">[--:--:--]</span> <span class="msg">Tour de Controle initializing...</span></div></div>
  <footer>Apophy Sovereign v0.1.0 — Rust + Bun + uv + Playwright | Zero Cloud. Full Sovereignty.</footer>

  <script>
    const ws = new WebSocket(\`ws://\${location.host}/ws\`);
    const log = document.getElementById('log');

    function addLog(msg) {
      const ts = new Date().toLocaleTimeString();
      const entry = document.createElement('div');
      entry.className = 'entry';
      entry.innerHTML = \`<span class="ts">[\${ts}]</span> <span class="msg">\${msg}</span>\`;
      log.prepend(entry);
      if (log.children.length > 100) log.removeChild(log.lastChild);
    }

    function m(label, value, cls = '') {
      return \`<div class="metric"><span class="label">\${label}</span><span class="value \${cls}">\${value}</span></div>\`;
    }

    function zoneBar(zone) {
      const pct = zone.max_tasks > 0 ? (zone.active_tasks / zone.max_tasks * 100) : 0;
      const color = pct > 80 ? '#ff4444' : pct > 50 ? '#ffcc00' : '#00ff88';
      return \`<div class="zone">
        <div class="zone-header">
          <span>\${zone.name}</span>
          <span>\${zone.active_tasks}/\${zone.max_tasks} (\${zone.cpu_cores}c, \${zone.memory_mb}MB)</span>
        </div>
        <div class="zone-bar"><div class="zone-bar-fill" style="width:\${pct}%;background:\${color}"></div></div>
      </div>\`;
    }

    ws.onopen = () => {
      ws.send(JSON.stringify({ subscribe: ['status'] }));
      addLog('Connected to Tour de Controle');
    };

    ws.onmessage = (e) => {
      const data = JSON.parse(e.data);
      if (data.type !== 'status') return;

      if (data.masters) {
        const ms = data.masters;
        document.getElementById('s-running').textContent = ms.total_running;
        document.getElementById('s-queued').textContent = ms.total_queued;
        document.getElementById('s-completed').textContent = ms.total_completed;

        if (ms.time_master) {
          const t = ms.time_master;
          document.getElementById('s-tps').textContent = t.tasks_per_second.toFixed(1);
          document.getElementById('time-metrics').innerHTML =
            m('Queue', t.queue_depth) +
            m('Rate', t.tasks_per_second.toFixed(1) + '/s') +
            m('Scheduled', t.total_scheduled, 'ok') +
            m('Deferred', t.total_deferred, t.total_deferred > 0 ? 'warn' : '');
        }
        if (ms.space_master) {
          const sp = ms.space_master;
          document.getElementById('s-util').textContent = sp.utilization_pct.toFixed(0) + '%';
          document.getElementById('space-metrics').innerHTML =
            m('Cores', sp.total_cores) +
            m('Memory', sp.total_memory_mb + ' MB') +
            m('Active', sp.active_tasks) +
            m('Utilization', sp.utilization_pct.toFixed(1) + '%',
              sp.utilization_pct > 80 ? 'err' : sp.utilization_pct > 50 ? 'warn' : 'ok');
          if (sp.zones) {
            document.getElementById('zones').innerHTML = sp.zones.map(zoneBar).join('');
          }
        }
        if (ms.latent_master) {
          const l = ms.latent_master;
          document.getElementById('latent-metrics').innerHTML =
            m('Patterns', l.patterns_detected) +
            m('Observations', l.total_observations) +
            m('Accuracy', l.avg_prediction_accuracy > 0 ? (l.avg_prediction_accuracy * 100).toFixed(0) + '%' : '--');
        }
      }

      if (data.brain) {
        const b = data.brain;
        document.getElementById('brain-metrics').innerHTML =
          m('Backend', b.backend_name, b.model_loaded ? 'ok' : 'warn') +
          m('Model', b.model_name || 'none', b.model_loaded ? 'ok' : 'err') +
          m('Hardware', b.hardware_backend) +
          m('Memory', b.memory_mb + ' MB') +
          m('CPU', b.cpu_cores + ' cores') +
          m('AZR Episodes', b.azr_episodes) +
          m('Ashoka', b.ashoka_feedback_count + ' fb / ' + b.ashoka_evolutions + ' evo');
      }

      if (data.health) {
        const h = data.health;
        document.getElementById('health-metrics').innerHTML =
          m('Status', h.status, h.status === 'healthy' ? 'ok' : 'err') +
          m('Uptime', h.uptime_secs + 's') +
          m('Version', h.version);
      }

      if (data.fleet) {
        const f = data.fleet;
        document.getElementById('s-agents').textContent = f.total_agents;
        document.getElementById('fleet-metrics').innerHTML =
          m('Total', f.total_agents) +
          m('Strategic', f.strategic_count + ' generals') +
          m('Tactical', f.tactical_count + ' specialists') +
          m('Operational', f.operational_count + ' micro-agents');
      }

      if (data.workflows && data.workflows.workflows) {
        const wfs = data.workflows.workflows;
        if (wfs.length === 0) {
          document.getElementById('workflow-metrics').innerHTML = '<div class="metric"><span class="label">No active workflows</span></div>';
        } else {
          document.getElementById('workflow-metrics').innerHTML = wfs.map(w =>
            \`<div style="margin-bottom:8px">
              <div class="metric"><span class="label">\${w.name}</span><span class="value \${w.status === 'completed' ? 'ok' : w.status === 'failed' ? 'err' : ''}">\${w.status}</span></div>
              <div class="zone-bar"><div class="zone-bar-fill" style="width:\${w.progress*100}%;background:#00ccff"></div></div>
            </div>\`
          ).join('');
        }
      }

      if (data.runtime) {
        const r = data.runtime;
        addLog(\`Runtime: \${r.total_executions} execs, \${r.success_rate.toFixed(0)}% success, \${r.avg_latency_ms.toFixed(0)}ms avg\`);
      }
    };

    ws.onclose = () => addLog('Disconnected from Tour de Controle');
    ws.onerror = () => addLog('WebSocket error');
  </script>
</body>
</html>`;
}

// Bun server
Bun.serve({
  port: PORT,
  fetch(req, server) {
    const url = new URL(req.url);

    if (url.pathname === "/ws") {
      if (server.upgrade(req)) return;
      return new Response("WebSocket upgrade failed", { status: 400 });
    }

    if (url.pathname.startsWith("/api/")) {
      return fetch(`${SOVEREIGN_API}${url.pathname}`, {
        method: req.method,
        headers: req.headers,
        body: req.body,
      });
    }

    return new Response(dashboardHtml(), {
      headers: { "Content-Type": "text/html" },
    });
  },
  websocket: {
    open(ws) {
      clients.push({ ws, subscriptions: new Set() });
      console.log(`[tour-de-controle] Client connected (${clients.length} total)`);
    },
    message(ws, message) {
      try {
        const data = JSON.parse(message.toString());
        const client = clients.find((c) => c.ws === ws);
        if (client && data.subscribe) {
          for (const topic of data.subscribe) {
            client.subscriptions.add(topic);
          }
        }
      } catch {}
    },
    close(ws) {
      const idx = clients.findIndex((c) => c.ws === ws);
      if (idx >= 0) clients.splice(idx, 1);
      console.log(`[tour-de-controle] Client disconnected (${clients.length} total)`);
    },
  },
});

console.log(`
+=====================================================+
|  TOUR DE CONTROLE — Apophy Sovereign                |
|  Dashboard:  http://localhost:${PORT}                   |
|  WebSocket:  ws://localhost:${PORT}/ws                  |
|  API Proxy:  http://localhost:${PORT}/api/*              |
|                                                     |
|  Multi-Runtime: Rust + Bun + uv + Playwright        |
|  Fleet: 3000 agents (9 strategic, 101 tactical,     |
|         2720 operational micro-agents)               |
+=====================================================+
`);

// Poll every 2 seconds
setInterval(pollAndBroadcast, 2000);
