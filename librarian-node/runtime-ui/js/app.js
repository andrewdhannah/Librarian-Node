const STATE = { data: null, activeTab: 'overview', expandedFindings: {}, expandedAnomalies: {}, expandedPatterns: {} };

function switchTab(tab) {
  STATE.activeTab = tab;
  document.querySelectorAll('.lr-tab').forEach(t => t.classList.toggle('active', t.dataset.tab === tab));
  document.querySelectorAll('.lr-view').forEach(v => v.classList.toggle('active', v.id === `view-${tab}`));
}

function switchTabAndFilter(tab, filter) {
  STATE.activeFilter = filter;
  switchTab(tab);
}

function statusDot(status) {
  const cls = status === 'healthy' || status === 'online' || status === 'running' ? 'running'
    : status === 'degraded' || status === 'loading' ? 'loading'
    : status === 'error' || status === 'offline' || status === 'failed' ? 'error'
    : 'unavailable';
  return `<span class="lr-status lr-status--${cls}"><span class="lr-status-dot"></span>${(status || 'unknown').toUpperCase()}</span>`;
}

function metric(label, value, unit) {
  return `<div class="lr-metric"><div class="lr-metric-label">${label}</div><div class="lr-metric-value">${value || '—'}${unit ? ` <span class="lr-metric-unit">${unit}</span>` : ''}</div></div>`;
}

function badge(text, variant) {
  return `<span class="lr-badge lr-badge--${variant || 'inactive'}">${text}</span>`;
}

function clickableStat(label, value, tab, filter) {
  const filterAttr = filter ? ` data-filter="${filter}"` : '';
  return `<div class="lr-metric lr-clickable-stat" onclick="switchTabAndFilter('${tab}', '${filter || ''}')"${filterAttr}><div class="lr-metric-label">${label}</div><div class="lr-metric-value">${value || '—'}</div></div>`;
}

function toggleExpand(key, type) {
  const target = type === 'finding' ? STATE.expandedFindings
    : type === 'anomaly' ? STATE.expandedAnomalies
    : STATE.expandedPatterns;
  target[key] = !target[key];
  refresh();
}

function findingWhySection(f) {
  return `<div class="lr-why-section"><strong>Why am I seeing this?</strong>
    <div class="lr-why-detail">Detection: ${escHtml(f.detection_method || 'automated')}</div>
    <div class="lr-why-detail">Confidence: ${escHtml(f.confidence || 'medium')}</div>
    ${f.observations ? `<div class="lr-why-detail">Observations: ${f.observations}</div>` : ''}
    ${f.supporting_data ? `<div class="lr-why-detail">Supporting data: <code>${escHtml(JSON.stringify(f.supporting_data))}</code></div>` : ''}
  </div>`;
}

function escHtml(s) {
  if (!s) return '';
  return String(s).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;');
}

function section(title) {
  return `<div class="lr-section-title">${title}</div>`;
}

function renderOverview(d) {
  if (!d) return '<div class="lr-card"><div class="lr-empty">No data available</div></div>';
  const node = d.node || {};
  const identity = node.identity || {};
  const status = node.status || {};
  const health = d.health || {};
  const pending = d.pending_decisions || {};
  const pendingCount = pending.total_pending || 0;
  const pendingItems = pending.items || [];

  return `
    ${section('Node Identity')}
    <div class="lr-card">
      <div class="lr-card-header">
        <span class="lr-card-title">${identity.display_name || 'Unknown Node'}</span>
        ${statusDot(status.state || health.overall_status || 'unavailable')}
      </div>
      <div class="lr-card-body">
        <div class="lr-grid-2 lr-grid-3">
          ${metric('Node ID', identity.node_id)}
          ${metric('Version', identity.runtime_version)}
          ${metric('Platform', identity.platform)}
          ${metric('Uptime', status.uptime_seconds ? formatDuration(status.uptime_seconds) : '—')}
          ${metric('State', status.state)}
          ${metric('Contract', identity.contract_version)}
        </div>
      </div>
    </div>

    ${section('Health')}
    <div class="lr-card">
      <div class="lr-card-header">
        <span class="lr-card-title">Component Health</span>
        ${statusDot(health.overall_status || 'unknown')}
      </div>
      <div class="lr-card-body">
        ${(health.components || []).map(c => `
          <div class="lr-model-row" style="cursor:default">
            <span class="lr-model-name">${c.component}</span>
            <div style="display:flex;align-items:center;gap:8px">
              <span class="lr-model-meta">${c.details || ''}</span>
              ${statusDot(c.status)}
            </div>
          </div>
        `).join('') || '<div class="lr-empty">No component data</div>'}
      </div>
    </div>

    ${section('Quick Stats')}
    <div class="lr-card">
      <div class="lr-card-body">
        <div class="lr-grid-2 lr-grid-4">
          ${clickableStat('Workloads', d.workloads ? d.workloads.total : '—', 'operations', null)}
          ${clickableStat('Active Sessions', d.sessions ? d.sessions.active : '—', 'operations', null)}
          ${clickableStat('Capabilities', d.capabilities ? d.capabilities.total : '—', 'governance', null)}
          ${clickableStat('Verified', d.capabilities ? d.capabilities.verified : '—', 'governance', null)}
          ${clickableStat('Findings', d.intelligence ? d.intelligence.findings_count : '—', 'intelligence', 'findings')}
          ${clickableStat('Anomalies', d.intelligence ? d.intelligence.active_anomalies : '—', 'intelligence', 'anomalies')}
          ${clickableStat('Fleet Nodes', d.fleet ? d.fleet.node_count : '—', 'operations', null)}
          ${clickableStat('Custody Envs', d.custody ? d.custody.envelope_count : '—', 'governance', null)}
        </div>
      </div>
    </div>

    ${section('Pending Owner Decisions')}
    <div class="lr-card">
      <div class="lr-card-header">
        <span class="lr-card-title">Pending Decisions</span>
        ${badge(pendingCount > 0 ? `${pendingCount} pending` : 'None', pendingCount > 0 ? 'warning' : 'inactive')}
      </div>
      <div class="lr-card-body">
        ${pendingCount > 0 ? pendingItems.map(item => `
          <div class="lr-model-row" style="cursor:default">
            <div class="lr-model-info">
              <div class="lr-model-name">${item.description || item.item_type}</div>
              <div class="lr-model-meta">${item.item_type} &middot; ${item.item_id}</div>
            </div>
            <div style="display:flex;align-items:center;gap:8px">
              <span class="lr-model-meta">${item.impact || ''}</span>
              ${badge('Awaiting', 'warning')}
            </div>
          </div>
        `).join('') : '<div class="lr-empty">No pending decisions</div>'}
      </div>
    </div>`;
}

function renderIntelligence(d) {
  if (!d) return '<div class="lr-card"><div class="lr-empty">No data available</div></div>';
  const intel = d.intelligence || {};
  const findingsSummary = intel.findings_summary || {};
  const bySeverity = findingsSummary.by_severity || {};
  const byCategory = findingsSummary.by_category || [];
  const latestFindings = findingsSummary.latest_findings || [];
  const patternSummary = intel.pattern_summary || {};
  const insightFindings = d.insight_findings || [];
  const insightAnomalies = d.insight_anomalies || [];
  const insightPatterns = d.insight_patterns || [];

  const activeFilter = STATE.activeFilter;

  return `
    ${section('Findings Summary')}
    <div class="lr-card">
      <div class="lr-card-header">
        <span class="lr-card-title">Findings</span>
        ${badge(`${findingsSummary.total_findings || 0} total`, findingsSummary.pending_review > 0 ? 'warning' : 'inactive')}
      </div>
      <div class="lr-card-body">
        <div class="lr-grid-2 lr-grid-4">
          ${metric('Total', findingsSummary.total_findings)}
          ${metric('Pending Review', findingsSummary.pending_review)}
          ${metric('Acknowledged', findingsSummary.acknowledged)}
        </div>
        <div class="lr-section-title" style="padding-top:12px">By Severity</div>
        <div class="lr-grid-2 lr-grid-4">
          ${metric('Info', bySeverity.info)}
          ${metric('Notable', bySeverity.notable)}
          ${metric('Warning', bySeverity.warning)}
          ${metric('Critical', bySeverity.critical)}
        </div>
        ${byCategory.length > 0 ? `
        <div class="lr-section-title" style="padding-top:12px">By Category</div>
        ${byCategory.map(c => `
          <div class="lr-model-row" style="cursor:default">
            <span class="lr-model-name">${c.category}</span>
            ${badge(`${c.count}`, c.count > 0 ? 'info' : 'inactive')}
          </div>
        `).join('')}` : ''}
      </div>
    </div>

    ${insightFindings.length > 0 ? renderFindingsDetail(insightFindings, activeFilter) : `
    ${section('Detailed Findings')}
    <div class="lr-card">
      <div class="lr-card-body">
        <div class="lr-empty">No classified findings</div>
      </div>
    </div>`}

    ${insightAnomalies.length > 0 ? renderAnomaliesDetail(insightAnomalies, activeFilter) : ''}

    ${insightPatterns.length > 0 ? renderPatternsDetail(insightPatterns, activeFilter) : ''}

    ${section('Generate Report')}
    <div class="lr-card">
      <div class="lr-card-body" style="text-align:center;padding:16px">
        <button class="lr-btn lr-btn-primary" onclick="generateReport()">Generate Report</button>
        <div id="lr-report-output" style="margin-top:12px;display:none"></div>
      </div>
    </div>

    ${section('Configuration Status')}
    <div class="lr-card">
      <div class="lr-card-body">
        <div class="lr-model-row" style="cursor:default">
          <span class="lr-model-name">Baselines</span>
          ${badge('Active', 'info')}
        </div>
        <div class="lr-model-row" style="cursor:default">
          <span class="lr-model-name">Thresholds</span>
          ${badge('Configured', 'info')}
        </div>
        <div class="lr-model-row" style="cursor:default">
          <span class="lr-model-name">Pattern Detection Config</span>
          ${badge('Active', 'info')}
        </div>
      </div>
    </div>`;
}

function renderFindingsDetail(findings, activeFilter) {
  const filtered = activeFilter === 'findings' ? findings : findings;
  return `
    ${section('Detailed Findings (${filtered.length})')}
    <div class="lr-card">
      <div class="lr-card-body">
        ${filtered.slice(0, 50).map(f => {
          const fid = f.finding_id || f.id || Math.random();
          const expanded = STATE.expandedFindings[fid];
          return `
          <div class="lr-finding-row">
            <div class="lr-model-row" onclick="toggleExpand('${escHtml(fid)}','finding')" style="cursor:pointer">
              <div class="lr-model-info">
                <div class="lr-model-name">${escHtml(f.title)}</div>
                <div class="lr-model-meta">${escHtml(f.category)} &middot; ${escHtml(f.severity)} ${f.affected_entity_id ? '&middot; ' + escHtml(f.affected_entity_id) : ''}</div>
              </div>
              ${badge(f.owner_review_status || 'unknown', f.owner_review_status === 'pending_review' ? 'warning' : 'inactive')}
            </div>
            ${expanded ? `
            <div class="lr-finding-expanded">
              ${findingWhySection(f)}
              ${(f.evidence_references || []).length > 0 ? `
              <div class="lr-evidence-section"><strong>Evidence References:</strong>
                ${f.evidence_references.map(ref => `<div class="lr-evidence-item">${escHtml(ref)}</div>`).join('')}
              </div>` : ''}
              ${f.description ? `<div class="lr-evidence-section"><strong>Description:</strong> ${escHtml(f.description)}</div>` : ''}
              ${f.source_references ? `<div class="lr-evidence-section"><strong>Source Refs:</strong> ${f.source_references.map(r => `<span class="lr-evidence-item" style="display:inline-block;margin-right:4px">${escHtml(r)}</span>`).join('')}</div>` : ''}
              ${f.workload_ids ? `<div class="lr-evidence-section"><strong>Workload IDs:</strong> ${f.workload_ids.map(w => `<span class="lr-evidence-item">${escHtml(w)}</span>`).join(', ')}</div>` : ''}
              ${f.session_ids ? `<div class="lr-evidence-section"><strong>Session IDs:</strong> ${f.session_ids.map(s => `<span class="lr-evidence-item">${escHtml(s)}</span>`).join(', ')}</div>` : ''}
              ${f.custody_envelope_ids ? `<div class="lr-evidence-section"><strong>Custody Envelope IDs:</strong> ${f.custody_envelope_ids.map(c => `<span class="lr-evidence-item">${escHtml(c)}</span>`).join(', ')}</div>` : ''}
            </div>` : `
            <div class="lr-finding-expanded" style="padding:4px 0 4px 16px">
              <span class="lr-model-meta" style="cursor:pointer" onclick="toggleExpand('${escHtml(fid)}','finding')">Click to expand evidence & details</span>
            </div>`}
          </div>`;
        }).join('')}
      </div>
    </div>`;
}

function renderAnomaliesDetail(anomalies, activeFilter) {
  const filtered = activeFilter === 'anomalies' ? anomalies : anomalies;
  return `
    ${section('Active Anomalies (${filtered.length})')}
    <div class="lr-card">
      <div class="lr-card-body">
        ${filtered.slice(0, 50).map(a => {
          const aid = a.anomaly_id || a.id || Math.random();
          const expanded = STATE.expandedAnomalies[aid];
          const obs = a.observation || {};
          return `
          <div class="lr-finding-row">
            <div class="lr-model-row" onclick="toggleExpand('${escHtml(aid)}','anomaly')" style="cursor:pointer">
              <div class="lr-model-info">
                <div class="lr-model-name">${escHtml(obs.metric_name || a.metric_name || 'unknown')} [${escHtml(obs.context || a.context || '')}]</div>
                <div class="lr-model-meta">Deviation: ${(obs.deviation_factor || a.deviation_factor || 0).toFixed(2)} &middot; Severity: ${escHtml(a.severity)}</div>
              </div>
              ${badge(a.status || 'open', a.severity === 'critical' ? 'error' : 'warning')}
            </div>
            ${expanded ? `
            <div class="lr-finding-expanded">
              <div class="lr-why-section"><strong>Why am I seeing this?</strong>
                <div class="lr-why-detail">Metric: ${escHtml(obs.metric_name || a.metric_name)}</div>
                <div class="lr-why-detail">Context: ${escHtml(obs.context || a.context)}</div>
                <div class="lr-why-detail">Baseline mean: ${obs.baseline_mean || '—'} &plusmn; ${obs.baseline_std_dev || '—'}</div>
                <div class="lr-why-detail">Observed: ${obs.observed_value || '—'}</div>
                <div class="lr-why-detail">Deviation factor: ${(obs.deviation_factor || a.deviation_factor || 0).toFixed(2)}</div>
                <div class="lr-why-detail">Direction: ${escHtml(obs.direction || '—')}</div>
              </div>
              ${(obs.evidence_workload_ids || []).length > 0 ? `
              <div class="lr-evidence-section"><strong>Evidence Workload IDs:</strong>
                ${obs.evidence_workload_ids.map(wid => `<div class="lr-evidence-item">${escHtml(wid)}</div>`).join('')}
              </div>` : ''}
            </div>` : `
            <div class="lr-finding-expanded" style="padding:4px 0 4px 16px">
              <span class="lr-model-meta" style="cursor:pointer" onclick="toggleExpand('${escHtml(aid)}','anomaly')">Click to expand deviation details & evidence</span>
            </div>`}
          </div>`;
        }).join('')}
      </div>
    </div>`;
}

function renderPatternsDetail(patterns, activeFilter) {
  const filtered = activeFilter === 'patterns' ? patterns : patterns;
  return `
    ${section('Active Patterns (${filtered.length})')}
    <div class="lr-card">
      <div class="lr-card-body">
        ${filtered.slice(0, 50).map(p => {
          const pid = p.pattern_id || p.id || Math.random();
          const expanded = STATE.expandedPatterns[pid];
          const prov = p.provenance || {};
          return `
          <div class="lr-finding-row">
            <div class="lr-model-row" onclick="toggleExpand('${escHtml(pid)}','pattern')" style="cursor:pointer">
              <div class="lr-model-info">
                <div class="lr-model-name">${escHtml(p.title)}</div>
                <div class="lr-model-meta">${escHtml(p.category)} &middot; ${p.constituent_finding_ids ? p.constituent_finding_ids.length : p.finding_count || 0} constituents &middot; ${escHtml(p.severity)}</div>
              </div>
              ${badge(p.status || p.owner_review_status || 'active', p.severity === 'critical' ? 'error' : 'warning')}
            </div>
            ${expanded ? `
            <div class="lr-finding-expanded">
              <div class="lr-why-section"><strong>Why am I seeing this?</strong>
                <div class="lr-why-detail">Detection: pattern_escalation</div>
                <div class="lr-why-detail">Confidence: ${escHtml(p.confidence || 'medium')}</div>
                <div class="lr-why-detail">Constituent findings: ${(p.constituent_finding_ids || []).length}</div>
                <div class="lr-why-detail">Constituent anomalies: ${(p.constituent_anomaly_ids || []).length}</div>
              </div>
              ${prov.evidence_references && prov.evidence_references.length > 0 ? `
              <div class="lr-evidence-section"><strong>Evidence References:</strong>
                ${prov.evidence_references.map(ref => `<div class="lr-evidence-item">${escHtml(ref)}</div>`).join('')}
              </div>` : ''}
              ${prov.workload_ids && prov.workload_ids.length > 0 ? `
              <div class="lr-evidence-section"><strong>Workload IDs:</strong>
                ${prov.workload_ids.map(w => `<span class="lr-evidence-item">${escHtml(w)}</span>`).join(', ')}
              </div>` : ''}
              ${prov.session_ids && prov.session_ids.length > 0 ? `
              <div class="lr-evidence-section"><strong>Session IDs:</strong>
                ${prov.session_ids.map(s => `<span class="lr-evidence-item">${escHtml(s)}</span>`).join(', ')}
              </div>` : ''}
              ${prov.custody_envelope_ids && prov.custody_envelope_ids.length > 0 ? `
              <div class="lr-evidence-section"><strong>Custody Envelope IDs:</strong>
                ${prov.custody_envelope_ids.map(c => `<span class="lr-evidence-item">${escHtml(c)}</span>`).join(', ')}
              </div>` : ''}
            </div>` : `
            <div class="lr-finding-expanded" style="padding:4px 0 4px 16px">
              <span class="lr-model-meta" style="cursor:pointer" onclick="toggleExpand('${escHtml(pid)}','pattern')">Click to expand provenance & details</span>
            </div>`}
          </div>`;
        }).join('')}
      </div>
    </div>`;
}

async function generateReport() {
  const btn = document.querySelector('.lr-btn-primary');
  const out = document.getElementById('lr-report-output');
  if (!btn || !out) return;
  btn.disabled = true;
  btn.textContent = 'Generating...';
  out.style.display = 'block';
  out.innerHTML = '<div class="lr-empty">Generating report...</div>';
  try {
    const res = await fetch('/owner/insight/report', { method: 'POST', headers: {'Content-Type':'application/json'}, body: JSON.stringify({period: 'last_24h'}) });
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const report = await res.json();
    out.innerHTML = `
      <div class="lr-report-card">
        <div class="lr-report-header">Report ${escHtml(report.report_id || '').slice(0, 8)}…</div>
        <div class="lr-report-meta">Period: ${escHtml(report.report_period || 'last_24h')} &middot; Generated: ${escHtml(report.generated_at || '')}</div>
        <div class="lr-report-meta">Findings: ${(report.detailed_findings || []).length} &middot; Anomalies: ${(report.detailed_anomalies || []).length} &middot; Recommendations: ${(report.recommendations || []).length}</div>
        <div style="margin-top:8px;display:flex;gap:8px">
          <button class="lr-btn lr-btn-sm" onclick="document.getElementById('lr-report-detail').style.display=document.getElementById('lr-report-detail').style.display==='none'?'block':'none'">Toggle Detail</button>
          <button class="lr-btn lr-btn-sm" onclick="downloadReport(${JSON.stringify(report).replace(/"/g,'&quot;')})">Download JSON</button>
        </div>
        <pre id="lr-report-detail" style="display:none;margin-top:8px;background:#1e1e2e;color:#cdd6f4;padding:8px;border-radius:4px;font-size:11px;max-height:300px;overflow:auto;text-align:left">${escHtml(JSON.stringify(report, null, 2))}</pre>
      </div>`;
  } catch(e) {
    out.innerHTML = `<div class="lr-empty" style="color:var(--lr-error)">Report generation failed: ${escHtml(e.message)}</div>`;
  } finally {
    btn.disabled = false;
    btn.textContent = 'Generate Report';
  }
}

function downloadReport(report) {
  const blob = new Blob([JSON.stringify(report, null, 2)], {type: 'application/json'});
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `insight-report-${(report.report_id || 'report').slice(0,8)}.json`;
  a.click();
  URL.revokeObjectURL(url);
}

function renderOperations(d) {
  if (!d) return '<div class="lr-card"><div class="lr-empty">No data available</div></div>';
  const workloads = d.workloads || {};
  const sessions = d.sessions || {};
  const sessionList = sessions.sessions || [];
  const node = d.node || {};

  return `
    ${section('Workload Inventory')}
    <div class="lr-card">
      <div class="lr-card-header">
        <span class="lr-card-title">Workloads</span>
        ${badge(`${workloads.total || 0} total`, workloads.active > 0 ? 'active' : 'inactive')}
      </div>
      <div class="lr-card-body">
        <div class="lr-grid-2 lr-grid-3">
          ${metric('Total', workloads.total)}
          ${metric('Active', workloads.active)}
          ${metric('Completed', workloads.completed)}
          ${metric('Failed', workloads.failed)}
          ${metric('Pending', workloads.pending)}
          ${metric('Cancelled', workloads.cancelled)}
        </div>
        ${(workloads.workloads || []).length > 0 ? `
        <div class="lr-section-title" style="padding-top:12px">Recent Workloads</div>
        ${(workloads.workloads || []).slice(0, 10).map(w => `
          <div class="lr-model-row" style="cursor:default">
            <div class="lr-model-info">
              <div class="lr-model-name">${w.workload_type || w.workload_id}</div>
              <div class="lr-model-meta">${w.node_name || w.node_id} &middot; ${w.session_id || ''}</div>
            </div>
            ${badge(w.state, w.state === 'active' ? 'active' : w.state === 'failed' ? 'error' : w.state === 'completed' ? 'info' : 'inactive')}
          </div>
        `).join('')}` : '<div class="lr-empty" style="padding-top:12px">No workloads recorded</div>'}
      </div>
    </div>

    ${section('Session List')}
    <div class="lr-card">
      <div class="lr-card-header">
        <span class="lr-card-title">Sessions</span>
        ${badge(`${sessions.total || 0} total`, sessions.active > 0 ? 'active' : 'inactive')}
      </div>
      <div class="lr-card-body">
        <div class="lr-grid-2">
          ${metric('Total Sessions', sessions.total)}
          ${metric('Active', sessions.active)}
        </div>
        ${sessionList.length > 0 ? sessionList.slice(0, 10).map(s => `
          <div class="lr-model-row" style="cursor:default">
            <div class="lr-model-info">
              <div class="lr-model-name">${s.session_id}</div>
              <div class="lr-model-meta">${s.agent_id || ''} &middot; ${s.node_id || ''}</div>
            </div>
            ${badge(s.state, s.state === 'active' ? 'active' : s.state === 'closed' ? 'info' : 'inactive')}
          </div>
        `).join('') : '<div class="lr-empty">No sessions</div>'}
      </div>
    </div>

    ${section('Fleet Overview')}
    <div class="lr-card">
      <div class="lr-card-header">
        <span class="lr-card-title">Fleet</span>
        ${badge(`${d.fleet ? d.fleet.node_count : 0} nodes`, 'info')}
      </div>
      <div class="lr-card-body">
        <div class="lr-grid-2 lr-grid-3">
          ${metric('Node Count', d.fleet ? d.fleet.node_count : 0)}
          ${metric('Local Node', node.identity ? node.identity.node_id : '—')}
          ${metric('Trust Assessed', d.fleet && d.fleet.trust ? d.fleet.trust.length : 0)}
        </div>
        ${d.fleet && d.fleet.trust && d.fleet.trust.length > 0 ? `
        <div class="lr-section-title" style="padding-top:12px">Node Trust States</div>
        <div class="lr-table" style="display:grid;gap:4px">
          ${d.fleet.trust.map(t => `
            <div class="lr-model-row" style="cursor:default">
              <div class="lr-model-info">
                <div class="lr-model-name">${escHtml(t.node_id)}</div>
                <div class="lr-model-meta">Score: ${t.score.toFixed(1)}</div>
              </div>
              ${badge(t.trust_level, t.trust_level === 'trusted' ? 'active' : t.trust_level === 'degraded' || t.trust_level === 'suspended' ? 'error' : t.trust_level === 'onboarding' ? 'warning' : 'inactive')}
            </div>
          `).join('')}
        </div>` : ''}
      </div>
    </div>

    ${section('Bootstrap Status')}
    <div class="lr-card">
      <div class="lr-card-header">
        <span class="lr-card-title">Bootstrap</span>
        ${badge(node.overview && node.overview.bootstrap_completed ? 'Completed' : 'Not completed', node.overview && node.overview.bootstrap_completed ? 'active' : 'warning')}
      </div>
      <div class="lr-card-body">
        <div class="lr-grid-2">
          ${metric('Bootstrap Completed', node.overview ? (node.overview.bootstrap_completed ? 'Yes' : 'No') : '—')}
          ${metric('Core Connected', node.overview ? (node.overview.core_connected ? 'Yes' : 'No') : '—')}
        </div>
      </div>
    </div>`;
}

function renderGovernance(d) {
  if (!d) return '<div class="lr-card"><div class="lr-empty">No data available</div></div>';
  const pending = d.pending_decisions || {};
  const pendingWithInsight = d.pending_decisions_with_insight || pending.items || [];
  const reconciliation = d.reconciliation || {};
  const recovery = d.recovery || {};
  const custody = d.custody || {};
  const ownerHistory = d.owner_history;
  const node = d.node || {};
  const identity = node.identity || {};
  const status = node.status || {};
  const overview = node.overview || {};

  return `
    ${section('Node Governance')}
    <div class="lr-card">
      <div class="lr-card-header">
        <span class="lr-card-title">Runtime Node</span>
        ${badge(overview.registered ? 'Registered' : 'Unregistered', overview.registered ? 'active' : 'warning')}
      </div>
      <div class="lr-card-body">
        <div class="lr-grid-2">
          ${metric('Node ID', identity.node_id)}
          ${metric('Display Name', identity.display_name)}
          ${metric('Version', identity.runtime_version)}
          ${metric('State', status.state)}
          ${metric('Uptime', status.uptime_seconds ? formatDuration(status.uptime_seconds) : '—')}
          ${metric('Registration', overview.state)}
        </div>
      </div>
    </div>

    ${section('Owner Decisions')}
    <div class="lr-card">
      <div class="lr-card-header">
        <span class="lr-card-title">Pending Decisions</span>
        ${badge(pending.total_pending > 0 ? `${pending.total_pending} pending` : 'None', pending.total_pending > 0 ? 'warning' : 'inactive')}
      </div>
      <div class="lr-card-body">
        ${pendingWithInsight.length > 0 ? pendingWithInsight.map(item => {
          const ti = item.triggering_insight;
          return `
          <div class="lr-decision-row">
            <div class="lr-model-row" style="cursor:default;border-bottom:1px solid var(--lr-border);padding-bottom:8px">
              <div class="lr-model-info">
                <div class="lr-model-name">${escHtml(item.description || item.item_type)}</div>
                <div class="lr-model-meta">${escHtml(item.item_type)} &middot; ${item.requested_at || ''}</div>
              </div>
              ${badge(item.impact || 'Pending', item.impact === 'high' ? 'error' : 'warning')}
            </div>
            ${ti ? `
            <div class="lr-decision-insight" style="padding:8px 0 4px 12px;font-size:12px">
              <strong>Triggered by insight:</strong>
              <div style="padding:4px 0">${escHtml(ti.title || 'Unknown')}</div>
              <div class="lr-why-detail">Category: ${escHtml(ti.category || '—')} &middot; Severity: ${escHtml(ti.severity || '—')}</div>
              <div class="lr-why-detail">Detection: ${escHtml(ti.detection_method || 'automated')} &middot; Confidence: ${escHtml(ti.confidence || 'medium')}</div>
              ${ti.evidence_references && ti.evidence_references.length > 0 ? `
              <div class="lr-why-detail"><strong>Evidence:</strong> ${ti.evidence_references.map(r => `<span class="lr-evidence-item">${escHtml(r)}</span>`).join(', ')}</div>
              ` : ''}
              ${ti.description ? `<div class="lr-why-detail" style="margin-top:2px">${escHtml(ti.description)}</div>` : ''}
            </div>
            <div style="padding:4px 0 4px 12px;font-size:11px;color:var(--lr-text-muted)">
              Decision action: POST /owner/decide (approve/reject) &middot; No UI shortcut bypasses receipts
            </div>
            ` : `
            <div style="padding:8px 0 4px 12px;font-size:12px;color:var(--lr-text-muted)">
              No directly linked insight finding for this item.
            </div>`}
          </div>`;
        }).join('') : '<div class="lr-empty">No pending owner decisions</div>'}
      </div>
    </div>

    ${ownerHistory ? `
    ${section('Decision History')}
    <div class="lr-card">
      <div class="lr-card-body">
        <div class="lr-empty">Owner history available</div>
      </div>
    </div>` : ''}

    ${section('Reconciliation Status')}
    <div class="lr-card">
      <div class="lr-card-header">
        <span class="lr-card-title">Reconciliation</span>
        ${badge(reconciliation.total_receipts > 0 ? `${reconciliation.total_receipts} receipts` : 'No cycles', reconciliation.total_receipts > 0 ? 'info' : 'inactive')}
      </div>
      <div class="lr-card-body">
        <div class="lr-grid-2">
          ${metric('Total Receipts', reconciliation.total_receipts)}
        </div>
      </div>
    </div>

    ${section('Recovery Status')}
    <div class="lr-card">
      <div class="lr-card-header">
        <span class="lr-card-title">Recovery</span>
        ${badge(recovery.active ? 'Active' : 'None', recovery.active ? 'error' : 'inactive')}
      </div>
      <div class="lr-card-body">
        <div class="lr-grid-2">
          ${metric('Active Recovery', recovery.active ? 'Yes' : 'No')}
          ${recovery.status && recovery.status.state ? metric('State', recovery.status.state) : ''}
        </div>
      </div>
    </div>

    ${section('Capability Lifecycle')}
    <div class="lr-card">
      <div class="lr-card-header">
        <span class="lr-card-title">Lifecycle States</span>
        ${badge(`${(d.capability_lifecycle || []).length} capabilities`, 'info')}
      </div>
      <div class="lr-card-body">
        ${(d.capability_lifecycle || []).length > 0 ? (d.capability_lifecycle || []).map(lc => {
          const stateClass = lc.current_state === 'verified' || lc.current_state === 'active' ? 'active'
            : lc.current_state === 'pending_verification' || lc.current_state === 'discovered' ? 'warning'
            : lc.current_state === 'degraded' ? 'loading'
            : lc.current_state === 'retired' || lc.current_state === 'superseded' ? 'inactive'
            : 'unavailable';
          return `
          <div class="lr-model-row" style="cursor:default">
            <div class="lr-model-info">
              <div class="lr-model-name">${escHtml(lc.capability_type)}</div>
              <div class="lr-model-meta">${lc.change_count} state changes</div>
            </div>
            ${badge(lc.current_state.replace(/_/g, ' '), stateClass)}
          </div>
          `;
        }).join('') : '<div class="lr-empty">No capability lifecycle data</div>'}
      </div>
    </div>

    <div class="lr-card">
      <div class="lr-card-header">
        <span class="lr-card-title">State Change History</span>
        ${badge(d.capability_lifecycle ? d.capability_lifecycle.reduce((sum, lc) => sum + lc.change_count, 0) : 0 + ' changes', 'info')}
      </div>
      <div class="lr-card-body">
        ${(() => {
          const allChanges = (d.capability_lifecycle || []).flatMap(lc => (lc.state_changes || []).map(sc => ({ ...sc, capability_type: lc.capability_type })));
          const recent = allChanges.sort((a, b) => (b.changed_at || '').localeCompare(a.changed_at || '')).slice(0, 20);
          return recent.length > 0 ? recent.map(sc => `
            <div class="lr-model-row" style="cursor:default;font-size:12px">
              <div class="lr-model-info">
                <div class="lr-model-name">${escHtml(sc.capability_type)}: ${escHtml((sc.previous_state || 'unknown').replace(/_/g, ' '))} → ${escHtml((sc.new_state || 'unknown').replace(/_/g, ' '))}</div>
                <div class="lr-model-meta">${escHtml(sc.reason)} &middot; ${sc.changed_at ? new Date(sc.changed_at).toLocaleString() : ''}</div>
              </div>
            </div>
          `).join('') : '<div class="lr-empty">No state changes recorded</div>';
        })()}
      </div>
    </div>

    ${section('Custody & Evidence')}
    <div class="lr-card">
      <div class="lr-card-header">
        <span class="lr-card-title">Custody Integrity</span>
        ${badge(custody.integrity_verified ? 'Verified' : 'Unverified', custody.integrity_verified ? 'active' : 'error')}
      </div>
      <div class="lr-card-body">
        <div class="lr-grid-2">
          ${metric('Envelope Count', custody.envelope_count)}
          ${metric('Integrity Status', custody.integrity_verified ? 'Verified' : 'Unverified')}
        </div>
      </div>
    </div>

    <div class="lr-card">
      <div class="lr-card-header">
        <span class="lr-card-title">Governance Links</span>
      </div>
      <div class="lr-card-body">
        <div class="lr-model-row" style="cursor:default">
          <span class="lr-model-name">Capability Evidence</span>
          <span class="lr-evidence-badge">${d.capabilities && d.capabilities.verified > 0 ? `✓ ${d.capabilities.verified} Verified` : '—'}</span>
        </div>
        <div class="lr-model-row" style="cursor:default">
          <span class="lr-model-name">Evidence Chain</span>
          <span class="lr-evidence-badge">${custody.integrity_verified ? `✓ ${custody.envelope_count} Envelopes` : '—'}</span>
        </div>
        <div class="lr-model-row" style="cursor:default">
          <span class="lr-model-name">Bootstrap</span>
          <span class="lr-evidence-badge">${overview.bootstrap_completed ? '✓ Completed' : '○ Pending'}</span>
        </div>
        <div class="lr-model-row" style="cursor:default">
          <span class="lr-model-name">Core Sync</span>
          <span class="lr-evidence-badge">${overview.core_connected ? `✓ Connected` : '○ Offline'}</span>
        </div>
      </div>
    </div>`;
}

function formatDuration(seconds) {
  if (!seconds) return '—';
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;
  const parts = [];
  if (d > 0) parts.push(`${d}d`);
  if (h > 0) parts.push(`${h}h`);
  if (m > 0) parts.push(`${m}m`);
  parts.push(`${s}s`);
  return parts.join(' ');
}

async function refresh() {
  try {
    const res = await fetch('/operator/dashboard/data');
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    STATE.data = await res.json();
    const d = STATE.data;

    document.getElementById('view-overview').innerHTML = renderOverview(d);
    document.getElementById('view-intelligence').innerHTML = renderIntelligence(d);
    document.getElementById('view-operations').innerHTML = renderOperations(d);
    document.getElementById('view-governance').innerHTML = renderGovernance(d);

    const node = d.node || {};
    const identity = node.identity || {};
    const sessions = d.sessions || {};
    const ws = d.workloads || {};
    const meta = `${identity.display_name || identity.node_id || 'Node'} &middot; ${sessions.active || 0} active sessions &middot; ${ws.active || 0} active workloads`;
    document.getElementById('lr-header-meta').textContent = meta;
  } catch (e) {
    const err = `<div class="lr-card"><div class="lr-empty">Connection error — ${e.message}</div></div>`;
    document.getElementById('view-overview').innerHTML = err;
  }
}

document.addEventListener('DOMContentLoaded', () => {
  document.querySelectorAll('.lr-tab').forEach(tab => {
    tab.addEventListener('click', () => switchTab(tab.dataset.tab));
  });
  refresh();
  setInterval(refresh, 5000);
});
