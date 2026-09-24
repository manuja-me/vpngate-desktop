// VPN Gate Studio Client State
let allServers = [];
let selectedServer = null;
let currentCountryFilter = "ALL";
let currentVpnState = "Disconnected";
let timerInterval = null;
let sessionSeconds = 0;

// Communication with Rust backend
function postIpc(cmd, payload) {
  try {
    if (window.ipc) {
      window.ipc.postMessage(JSON.stringify({ cmd: cmd, payload: payload }));
    } else {
      console.warn("IPC bridge unavailable:", cmd, payload);
    }
  } catch (err) {
    console.error("IPC send error:", err);
  }
}

// Window Callbacks from Rust
window.onServersLoaded = function(servers) {
  if (!Array.isArray(servers)) return;
  allServers = servers;
  
  updateMetrics();
  updateCountryPills();
  
  if (!selectedServer && allServers.length > 0) {
    selectServer(allServers[0]);
  } else if (selectedServer) {
    // Preserve selection if still in list
    const found = allServers.find(s => s.ip === selectedServer.ip);
    if (found) selectServer(found);
  }

  renderServerList();
  
  const refreshBtn = document.getElementById("btn-refresh");
  if (refreshBtn) refreshBtn.innerText = "↻ REFRESH";
};

window.onVpnStatus = function(state, durationSecs) {
  const previousState = currentVpnState;
  currentVpnState = state;
  const badge = document.getElementById("badge-status");
  const btn = document.getElementById("btn-toggle-vpn");
  const btnIcon = document.getElementById("btn-icon");
  const btnText = document.getElementById("btn-text");

  badge.className = "status-badge mono " + state.toLowerCase();
  badge.innerText = state.toUpperCase();

  if (state === "Connected") {
    btn.className = "btn-primary-action mono disconnect";
    btnIcon.innerText = "■";
    btnText.innerText = "DISCONNECT FROM RELAY";
    
    if (typeof durationSecs === "number") {
      sessionSeconds = durationSecs;
    }
    startTimer();

    // Print explicit [CONNECTED] line when connection is achieved
    if (previousState !== "Connected") {
      const targetStr = selectedServer 
        ? `${selectedServer.country_long} (${selectedServer.ip}:${selectedServer.port} / ${selectedServer.proto})`
        : "relay node";
      window.onVpnLog(`[CONNECTED] >>> Tunnel established successfully to ${targetStr}. Traffic is now secured.`);
    }
  } else if (state === "Connecting") {
    btn.className = "btn-primary-action mono disconnect";
    btnIcon.innerText = "◌";
    btnText.innerText = "CANCEL CONNECTION";
    startTimer();
  } else if (state === "Disconnecting") {
    btn.className = "btn-primary-action mono disconnect";
    btnIcon.innerText = "◌";
    btnText.innerText = "DISCONNECTING...";
    stopTimer();
  } else {
    // Disconnected or Error
    btn.className = "btn-primary-action mono";
    btnIcon.innerText = "⚡";
    btnText.innerText = "CONNECT TO RELAY";
    if (previousState === "Connected") {
      window.onVpnLog("[DISCONNECTED] >>> Tunnel closed. System default routes restored.");
    }
    stopTimer();
  }
};

window.onVpnLog = function(logLine) {
  const con = document.getElementById("log-console");
  if (!con) return;

  const div = document.createElement("div");
  if (logLine.includes("[CONNECTED]") || logLine.includes(">>> Tunnel established") || logLine.includes("Tunnel Established")) {
    div.className = "log-connected";
  } else if (logLine.includes("Initialization Sequence Completed") || logLine.includes("[DISCONNECTED]")) {
    div.className = "log-highlight";
  } else if (logLine.includes("WARNING:") || logLine.includes("WARN") || logLine.includes("[Warning]")) {
    div.className = "log-warn";
  } else if (logLine.includes("ERROR:") || logLine.includes("FAILED") || logLine.includes("Fatal")) {
    div.className = "log-error";
  }
  
  // Format with current timestamp if line does not start with a bracketed tag or timestamp
  if (!logLine.startsWith("[")) {
    const now = new Date();
    const timeStr = `[${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}:${String(now.getSeconds()).padStart(2, '0')}] `;
    div.textContent = timeStr + logLine;
  } else {
    div.textContent = logLine;
  }

  con.appendChild(div);

  // Auto-scroll to bottom
  con.scrollTop = con.scrollHeight;
};

window.onAdminStatus = function(isAdmin) {
  const badge = document.getElementById("admin-badge");
  if (!badge) return;
  if (isAdmin) {
    badge.innerText = "ADMIN (ELEVATED)";
    badge.className = "admin-badge mono";
    badge.title = "Application running with full administrative routing privileges.";
  } else {
    badge.innerText = "USER MODE • CLICK TO ELEVATE";
    badge.className = "admin-badge mono warn";
    badge.title = "Click to restart with administrative privileges (required for route injection).";
  }
};

window.onMemoryTelemetry = function(mb) {
  const el = document.getElementById("footer-memory");
  if (el) el.innerText = "MEM: " + mb.toFixed(1) + " MB";
};

// UI Rendering & Interaction
function renderServerList() {
  const listEl = document.getElementById("server-list");
  if (!listEl) return;

  const query = document.getElementById("search-input").value.trim().toLowerCase();
  const sort = document.getElementById("sort-select").value;

  let filtered = allServers.filter(s => {
    if (currentCountryFilter !== "ALL") {
      if (s.country_long.toLowerCase() !== currentCountryFilter.toLowerCase() && 
          s.country_short.toLowerCase() !== currentCountryFilter.toLowerCase()) {
        return false;
      }
    }
    if (query) {
      const matchCountry = s.country_long.toLowerCase().includes(query);
      const matchIp = s.ip.includes(query);
      const matchHost = s.host_name.toLowerCase().includes(query);
      const matchOp = s.operator.toLowerCase().includes(query);
      if (!matchCountry && !matchIp && !matchHost && !matchOp) return false;
    }
    return true;
  });

  filtered.sort((a, b) => {
    if (sort === "speed") return b.speed_mbps - a.speed_mbps;
    if (sort === "ping") return a.ping - b.ping;
    if (sort === "sessions") return b.num_sessions - a.num_sessions;
    if (sort === "score") return b.score - a.score;
    return 0;
  });

  if (filtered.length === 0) {
    listEl.innerHTML = `
      <div class="empty-state mono">
        <div>NO SERVERS MATCHING CURRENT FILTER</div>
        <div style="font-size: 10px; color: #404040;">Try adjusting your search query or selecting ALL regions.</div>
      </div>
    `;
    return;
  }

  listEl.innerHTML = filtered.map(s => {
    const isSel = selectedServer && selectedServer.ip === s.ip;
    const rowClass = "server-row" + (isSel ? " selected" : "");
    const pingFast = s.ping < 50 ? " fast" : "";
    const flag = s.country_short ? getFlagEmoji(s.country_short) : "🌐";

    return `
      <div class="${rowClass}" onclick="selectServerByIp('${s.ip}')" ondblclick="toggleVpn()">
        <div class="td-cell td-center server-flag">${flag}</div>
        <div class="td-cell">
          <div class="server-title">${escapeHtml(s.country_long)}</div>
          <div class="server-sub mono">${s.ip}:${s.port} • ${escapeHtml(s.host_name)}</div>
        </div>
        <div class="td-cell td-right server-speed mono">${s.speed_mbps.toFixed(1)} Mbps</div>
        <div class="td-cell td-right server-ping mono${pingFast}">${s.ping} ms</div>
        <div class="td-cell td-right mono" style="color: #737373;">${s.num_sessions}</div>
        <div class="td-cell td-center">
          <span class="server-proto-badge mono">${s.proto}</span>
        </div>
      </div>
    `;
  }).join("");
}

function selectServerByIp(ip) {
  const s = allServers.find(x => x.ip === ip);
  if (s) {
    selectServer(s);
  }
}

function selectServer(s) {
  selectedServer = s;
  
  const flagEl = document.getElementById("target-flag");
  const nameEl = document.getElementById("target-name");
  const subEl = document.getElementById("target-sub");
  const speedEl = document.getElementById("target-speed");
  const pingEl = document.getElementById("target-ping");

  if (flagEl) flagEl.innerText = getFlagEmoji(s.country_short);
  if (nameEl) nameEl.innerText = s.country_long;
  if (subEl) subEl.innerText = s.ip + " : " + s.port + " (" + s.proto + ")";
  if (speedEl) speedEl.innerText = s.speed_mbps.toFixed(1) + " Mbps";
  if (pingEl) pingEl.innerText = s.ping + " ms";

  renderServerList();
}

function updateCountryPills() {
  const container = document.getElementById("country-pills-bar");
  if (!container) return;

  const counts = {};
  allServers.forEach(s => {
    const c = s.country_long || "Unknown";
    counts[c] = (counts[c] || 0) + 1;
  });

  const sortedCountries = Object.keys(counts).sort((a, b) => counts[b] - counts[a]);

  let html = `<button onclick="selectCountry('ALL')" class="country-pill mono ${currentCountryFilter === 'ALL' ? 'active' : ''}">ALL (${allServers.length})</button>`;
  
  sortedCountries.slice(0, 10).forEach(c => {
    const isActive = currentCountryFilter.toLowerCase() === c.toLowerCase();
    html += `<button onclick="selectCountry('${escapeHtml(c)}')" class="country-pill mono ${isActive ? 'active' : ''}">${escapeHtml(c.toUpperCase())} (${counts[c]})</button>`;
  });

  container.innerHTML = html;
}

function selectCountry(country) {
  currentCountryFilter = country;
  updateCountryPills();
  renderServerList();
}

function onFilterChange() {
  renderServerList();
}

function onSortChange() {
  renderServerList();
}

function updateMetrics() {
  const nodesEl = document.getElementById("metric-nodes");
  const countriesEl = document.getElementById("metric-countries");
  const bwEl = document.getElementById("metric-bandwidth");

  if (nodesEl) nodesEl.innerText = allServers.length + " RELAYS";

  const uniqueCountries = new Set(allServers.map(s => s.country_short)).size;
  if (countriesEl) countriesEl.innerText = uniqueCountries + " REGIONS";

  const peak = allServers.reduce((max, s) => s.speed_mbps > max ? s.speed_mbps : max, 0);
  if (bwEl) bwEl.innerText = peak.toFixed(1) + " MBPS";
}

// User Actions
function toggleVpn() {
  if (currentVpnState === "Connected" || currentVpnState === "Connecting") {
    postIpc("disconnect", null);
  } else {
    if (!selectedServer) {
      alert("Please select a VPN server first.");
      return;
    }
    postIpc("connect", selectedServer);
  }
}

function exportSelectedProfile() {
  if (!selectedServer) {
    alert("Please select a VPN server first.");
    return;
  }
  postIpc("export_config", selectedServer);
}

function requestAdminElevation() {
  postIpc("restart_as_admin", null);
}

function refreshServers(force) {
  const refreshBtn = document.getElementById("btn-refresh");
  if (refreshBtn) refreshBtn.innerText = "↻ FETCHING...";
  postIpc("fetch_servers", { force: !!force });
}

function switchTab(tab) {
  const tabRelays = document.getElementById("tab-relays");
  const tabLogs = document.getElementById("tab-logs");
  const viewRelays = document.getElementById("view-relays");
  const viewLogs = document.getElementById("view-logs");

  if (tab === "relays") {
    tabRelays.className = "tab-btn active";
    tabLogs.className = "tab-btn";
    viewRelays.style.display = "flex";
    viewLogs.style.display = "none";
  } else {
    tabRelays.className = "tab-btn";
    tabLogs.className = "tab-btn active";
    viewRelays.style.display = "none";
    viewLogs.style.display = "flex";
  }
}

function clearLogs() {
  const con = document.getElementById("log-console");
  if (con) {
    con.innerHTML = "<div>[00:00:00] Diagnostics console cleared.</div>";
  }
}

function copyLogs() {
  const con = document.getElementById("log-console");
  if (!con) return;
  const text = con.innerText;
  postIpc("copy_clipboard", { text: text });
  alert("Diagnostic logs copied to clipboard.");
}

// Timer management
function startTimer() {
  if (timerInterval) clearInterval(timerInterval);
  timerInterval = setInterval(() => {
    sessionSeconds++;
    updateTimerDisplay();
  }, 1000);
  updateTimerDisplay();
}

function stopTimer() {
  if (timerInterval) {
    clearInterval(timerInterval);
    timerInterval = null;
  }
  sessionSeconds = 0;
  updateTimerDisplay();
}

function updateTimerDisplay() {
  const clock = document.getElementById("session-clock");
  if (!clock) return;
  const h = String(Math.floor(sessionSeconds / 3600)).padStart(2, '0');
  const m = String(Math.floor((sessionSeconds % 3600) / 60)).padStart(2, '0');
  const s = String(sessionSeconds % 60).padStart(2, '0');
  clock.innerText = `${h}:${m}:${s}`;
}

// Helper utilities
function escapeHtml(str) {
  if (!str) return "";
  return str.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;").replace(/'/g, "&#039;");
}

function getFlagEmoji(countryCode) {
  if (!countryCode || countryCode.length !== 2) return "🌐";
  const codePoints = countryCode
    .toUpperCase()
    .split('')
    .map(char => 127397 + char.charCodeAt(0));
  return String.fromCodePoint(...codePoints);
}

// Bootstrap on DOM ready
document.addEventListener("DOMContentLoaded", () => {
  postIpc("init", null);
});
