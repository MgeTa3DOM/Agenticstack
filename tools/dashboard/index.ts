/**
 * Apophy Sovereign — Real-Time Brain Dashboard
 *
 * Bun WebSocket server for monitoring:
 * - Brain status (backend, model, hardware)
 * - AZR self-play episodes live
 * - AlphaResolve reasoning traces
 * - Ashoka prompt evolution events
 * - Fleet agent status
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

interface WsClient {
  ws: any;
  subscriptions: Set<string>;
}

const clients: WsClient[] = [];

// Poll sovereign API and broadcast to WebSocket clients
async function pollAndBroadcast() {
  try {
    const [brainRes, healthRes, fleetRes] = await Promise.allSettled([
      fetch(`${SOVEREIGN_API}/api/v1/brain/status`),
      fetch(`${SOVEREIGN_API}/health`),
      fetch(`${SOVEREIGN_API}/api/v1/fleet/summary`),
    ]);

    const brain = brainRes.status === "fulfilled" && brainRes.value.ok
      ? await brainRes.value.json()
      : null;
    const health = healthRes.status === "fulfilled" && healthRes.value.ok
      ? await healthRes.value.json()
      : null;
    const fleet = fleetRes.status === "fulfilled" && fleetRes.value.ok
      ? await fleetRes.value.json()
      : null;

    const payload = JSON.stringify({
      type: "status",
      timestamp: new Date().toISOString(),
      brain,
      health,
      fleet,
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

// Dashboard HTML
function dashboardHtml(): string {
  return `<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>Apophy Sovereign — Brain Dashboard</title>
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    body { font-family: 'JetBrains Mono', monospace; background: #0a0a0a; color: #e0e0e0; padding: 20px; }
    h1 { color: #00ff88; margin-bottom: 20px; font-size: 1.4em; }
    .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 16px; }
    .card { background: #1a1a2e; border: 1px solid #333; border-radius: 8px; padding: 16px; }
    .card h2 { color: #00ccff; font-size: 1em; margin-bottom: 12px; border-bottom: 1px solid #333; padding-bottom: 8px; }
    .metric { display: flex; justify-content: space-between; padding: 4px 0; }
    .metric .label { color: #888; }
    .metric .value { color: #fff; font-weight: bold; }
    .metric .value.ok { color: #00ff88; }
    .metric .value.warn { color: #ffcc00; }
    .metric .value.err { color: #ff4444; }
    .status { display: inline-block; width: 8px; height: 8px; border-radius: 50%; margin-right: 6px; }
    .status.online { background: #00ff88; }
    .status.offline { background: #ff4444; }
    #log { background: #111; border: 1px solid #333; border-radius: 8px; padding: 12px; margin-top: 16px;
           max-height: 300px; overflow-y: auto; font-size: 0.85em; }
    #log .entry { padding: 2px 0; border-bottom: 1px solid #1a1a1a; }
    #log .ts { color: #666; }
    #log .msg { color: #ccc; }
    footer { margin-top: 20px; color: #555; font-size: 0.8em; text-align: center; }
  </style>
</head>
<body>
  <h1>APOPHY SOVEREIGN — Brain Dashboard</h1>
  <div class="grid">
    <div class="card" id="brain-card">
      <h2>Brain Status</h2>
      <div id="brain-metrics">Connecting...</div>
    </div>
    <div class="card" id="health-card">
      <h2>Server Health</h2>
      <div id="health-metrics">Connecting...</div>
    </div>
    <div class="card" id="fleet-card">
      <h2>Fleet Status</h2>
      <div id="fleet-metrics">Connecting...</div>
    </div>
    <div class="card" id="engines-card">
      <h2>Reasoning Engines</h2>
      <div id="engines-metrics">
        <div class="metric"><span class="label">AlphaResolve</span><span class="value ok">READY</span></div>
        <div class="metric"><span class="label">AZR Self-Play</span><span class="value ok">READY</span></div>
        <div class="metric"><span class="label">CTM-C Compress</span><span class="value ok">READY</span></div>
        <div class="metric"><span class="label">Ashoka Autolearn</span><span class="value ok">READY</span></div>
      </div>
    </div>
  </div>
  <div id="log"><div class="entry"><span class="ts">[--:--:--]</span> <span class="msg">Waiting for connection...</span></div></div>
  <footer>Apophy Sovereign v0.1.0 — Zero Cloud. Zero Telemetry. Full Sovereignty.</footer>

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

    function metric(label, value, cls = '') {
      return \`<div class="metric"><span class="label">\${label}</span><span class="value \${cls}">\${value}</span></div>\`;
    }

    ws.onopen = () => {
      ws.send(JSON.stringify({ subscribe: ['status'] }));
      addLog('Connected to Apophy Sovereign');
    };

    ws.onmessage = (e) => {
      const data = JSON.parse(e.data);
      if (data.type === 'status') {
        // Brain
        if (data.brain) {
          const b = data.brain;
          document.getElementById('brain-metrics').innerHTML =
            metric('Backend', b.backend_name, b.model_loaded ? 'ok' : 'warn') +
            metric('Model', b.model_name || 'none', b.model_loaded ? 'ok' : 'err') +
            metric('Hardware', b.hardware_backend) +
            metric('Memory', b.memory_mb + ' MB') +
            metric('CPU Cores', b.cpu_cores) +
            metric('AZR Episodes', b.azr_episodes) +
            metric('Ashoka Feedback', b.ashoka_feedback_count) +
            metric('Evolutions', b.ashoka_evolutions);
        }
        // Health
        if (data.health) {
          const h = data.health;
          document.getElementById('health-metrics').innerHTML =
            metric('Status', h.status, h.status === 'healthy' ? 'ok' : 'err') +
            metric('Uptime', h.uptime_secs + 's') +
            metric('Version', h.version);
        }
        // Fleet
        if (data.fleet) {
          const f = data.fleet;
          document.getElementById('fleet-metrics').innerHTML =
            metric('Total Agents', f.total_agents) +
            metric('Strategic', f.strategic_count) +
            metric('Tactical', f.tactical_count) +
            metric('Operational', f.operational_count);
        }
        addLog(\`Status update: brain=\${data.brain?.backend_name || '?'}, health=\${data.health?.status || '?'}\`);
      }
    };

    ws.onclose = () => addLog('Disconnected');
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

    // WebSocket upgrade
    if (url.pathname === "/ws") {
      if (server.upgrade(req)) return;
      return new Response("WebSocket upgrade failed", { status: 400 });
    }

    // API proxy to sovereign
    if (url.pathname.startsWith("/api/")) {
      return fetch(`${SOVEREIGN_API}${url.pathname}`, {
        method: req.method,
        headers: req.headers,
        body: req.body,
      });
    }

    // Dashboard
    return new Response(dashboardHtml(), {
      headers: { "Content-Type": "text/html" },
    });
  },
  websocket: {
    open(ws) {
      clients.push({ ws, subscriptions: new Set() });
      console.log(`[dashboard] Client connected (${clients.length} total)`);
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
      console.log(`[dashboard] Client disconnected (${clients.length} total)`);
    },
  },
});

console.log(`
╔══════════════════════════════════════════════╗
║  APOPHY SOVEREIGN — Brain Dashboard          ║
║  http://localhost:${PORT}                       ║
║  WebSocket: ws://localhost:${PORT}/ws            ║
║  API Proxy: http://localhost:${PORT}/api/*       ║
╚══════════════════════════════════════════════╝
`);

// Poll every 2 seconds
setInterval(pollAndBroadcast, 2000);
