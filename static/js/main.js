// Ferrumyx Web UI — Main JavaScript

// ── SSE Connection for real-time updates ──
let eventSource = null;
let reconnectAttempts = 0;
const maxReconnectDelay = 30000; // 30 seconds max

function connectSSE() {
    if (eventSource) {
        eventSource.close();
    }

    eventSource = new EventSource('/events');

    eventSource.onopen = () => {
        reconnectAttempts = 0;
        console.log('SSE connected');
        updateConnectionStatus('connected');
    };

    eventSource.onmessage = (event) => {
        try {
            const data = JSON.parse(event.data);
            handleSSEEvent(data);
        } catch (e) {
            console.error('Failed to parse SSE event:', e);
        }
    };

    eventSource.onerror = () => {
        console.log('SSE disconnected, reconnecting...');
        updateConnectionStatus('disconnected');

        // Exponential backoff reconnection
        const delay = Math.min(1000 * Math.pow(2, reconnectAttempts), maxReconnectDelay);
        reconnectAttempts++;
        setTimeout(connectSSE, delay);
    };
}

function updateConnectionStatus(status) {
    const indicator = document.getElementById('sse-status');
    if (indicator) {
        indicator.className = status === 'connected' 
            ? 'badge bg-success' 
            : 'badge bg-warning text-dark';
        indicator.textContent = status === 'connected' ? '● Live' : '○ Reconnecting...';
    }
}

function handleSSEEvent(event) {
    console.log('SSE event:', event.type, event);

    switch (event.type) {
        case 'paper_ingested':
            showNotification('📄 New paper ingested', event.title, 'info');
            incrementStat('papers-count');
            break;

        case 'target_scored':
            showNotification('🎯 Target scored', `${event.gene} in ${event.cancer}: ${event.score.toFixed(3)}`, 'success');
            incrementStat('targets-count');
            break;

        case 'docking_complete':
            showNotification('🧪 Docking complete', `${event.molecule_id} → ${event.gene}: ${event.vina_score.toFixed(1)}`, 'success');
            break;

        case 'pipeline_status':
            updatePipelineProgress(event);
            break;

        case 'feedback_metric':
            // Update metrics display if visible
            const metricEl = document.querySelector(`[data-metric="${event.metric}"]`);
            if (metricEl) metricEl.textContent = event.value.toFixed(4);
            break;

        case 'notification':
            showNotification(event.level === 'error' ? '❌ Error' : 'ℹ️ Info', event.message, event.level);
            break;
    }
}

function showNotification(title, message, level = 'info') {
    const container = document.getElementById('notification-container') || createNotificationContainer();

    const alertClass = {
        'info': 'alert-info',
        'success': 'alert-success',
        'warning': 'alert-warning',
        'error': 'alert-danger'
    }[level] || 'alert-info';

    const notification = document.createElement('div');
    notification.className = `alert ${alertClass} alert-dismissible fade show notification-toast`;
    notification.innerHTML = `
        <strong>${title}</strong><br>
        <small>${message}</small>
        <button type="button" class="btn-close" data-bs-dismiss="alert"></button>
    `;

    container.appendChild(notification);

    // Auto-dismiss after 5 seconds
    setTimeout(() => {
        notification.classList.remove('show');
        setTimeout(() => notification.remove(), 300);
    }, 5000);
}

function createNotificationContainer() {
    const container = document.createElement('div');
    container.id = 'notification-container';
    container.style.cssText = 'position: fixed; top: 20px; right: 20px; z-index: 9999; max-width: 400px;';
    document.body.appendChild(container);
    return container;
}

function incrementStat(statId) {
    const el = document.getElementById(statId);
    if (el) {
        const current = parseInt(el.textContent.replace(/[^0-9]/g, ''), 10) || 0;
        animateCounter(el, current + 1);
    }
}

function updatePipelineProgress(event) {
    const progressBar = document.getElementById('pipeline-progress');
    const statusText = document.getElementById('pipeline-status-text');
    const stageText = document.getElementById('pipeline-stage');

    if (progressBar) {
        const progress = Math.min(event.count || 0, 100);
        progressBar.style.width = `${progress}%`;
        progressBar.setAttribute('aria-valuenow', progress);
    }

    if (statusText) {
        statusText.textContent = event.message;
    }

    if (stageText) {
        stageText.textContent = event.stage;
        stageText.className = 'badge ' + getStageBadgeClass(event.stage);
    }
}

function getStageBadgeClass(stage) {
    switch (stage) {
        case 'search': return 'bg-info text-dark';
        case 'upsert': return 'bg-primary';
        case 'chunk': return 'bg-warning text-dark';
        case 'embed': return 'bg-secondary';
        case 'ner': return 'bg-danger';
        case 'complete': return 'bg-success';
        case 'error': return 'bg-danger';
        default: return 'bg-secondary';
    }
}

// ── Highlight active nav link ──
document.addEventListener('DOMContentLoaded', () => {
    // Connect to SSE on page load
    connectSSE();

    const path = window.location.pathname;
    document.querySelectorAll('.nav-link').forEach(link => {
        const href = link.getAttribute('href');
        if (href && href !== '/' && path.startsWith(href)) {
            link.classList.add('active');
        } else if (href === '/' && path === '/') {
            link.classList.add('active');
        } else {
            link.classList.remove('active');
        }
    });

    // Auto-dismiss flash messages after 5s
    document.querySelectorAll('.alert-dismissible').forEach(el => {
        setTimeout(() => el.classList.add('fade'), 5000);
    });

    // Animate stat values on page load
    document.querySelectorAll('.stat-value').forEach(el => {
        const val = parseInt(el.textContent.replace(/[^0-9]/g, ''), 10);
        if (!isNaN(val) && val > 0) animateCounter(el, val);
    });
});

// ── Counter animation ──
function animateCounter(el, target) {
    const duration = 800;
    const start = performance.now();
    const startVal = 0;
    const fmt = el.textContent.includes(',') ? (n) => n.toLocaleString() : (n) => String(n);
    const animate = (now) => {
        const elapsed = now - start;
        const progress = Math.min(elapsed / duration, 1);
        const ease = 1 - Math.pow(1 - progress, 3); // ease-out cubic
        el.textContent = fmt(Math.round(startVal + (target - startVal) * ease));
        if (progress < 1) requestAnimationFrame(animate);
    };
    requestAnimationFrame(animate);
}

// ── Refresh stat cards ──
function refreshStats() {
    window.location.reload();
}

// ── Score bar tooltips ──
document.querySelectorAll('.progress-bar').forEach(bar => {
    const pct = parseInt(bar.style.width, 10);
    bar.title = `Score: ${(pct / 100).toFixed(3)}`;
});

// ── Confidence slider live display ──
const confSlider = document.querySelector('input[name="min_confidence"]');
if (confSlider) {
    confSlider.addEventListener('input', () => {
        const display = document.getElementById('conf-val');
        if (display) display.textContent = parseFloat(confSlider.value).toFixed(2);
    });
}

// ── Copy to clipboard for SMILES ──
document.querySelectorAll('td.font-monospace').forEach(cell => {
    cell.style.cursor = 'pointer';
    cell.addEventListener('click', () => {
        navigator.clipboard.writeText(cell.title || cell.textContent.trim()).then(() => {
            const orig = cell.textContent;
            cell.textContent = '✓ Copied';
            setTimeout(() => { cell.textContent = orig; }, 1000);
        });
    });
});

// ── Ingestion source checkboxes → hidden field ──
(function() {
    const checkboxMap = {
        'src_pubmed':         'pubmed',
        'src_europepmc':      'europepmc',
        'src_biorxiv':        'biorxiv',
        'src_medrxiv':        'medrxiv',
        'src_clinicaltrials': 'clinicaltrials',
        'src_crossref':       'crossref',
    };
    const hidden = document.getElementById('sources_hidden');
    if (!hidden) return;

    function updateSources() {
        const selected = Object.entries(checkboxMap)
            .filter(([id]) => document.getElementById(id)?.checked)
            .map(([, val]) => val);
        hidden.value = selected.join(',') || 'pubmed';
    }

    Object.keys(checkboxMap).forEach(id => {
        const el = document.getElementById(id);
        if (el) el.addEventListener('change', updateSources);
    });
    updateSources();
})();

// ── Query form: extract entities from NL query ──
const queryTextarea = document.querySelector('textarea[name="query_text"]');
if (queryTextarea) {
    queryTextarea.addEventListener('blur', () => {
        const text = queryTextarea.value.toLowerCase();

        // Simple client-side entity hints (server does proper NER)
        const geneMatch = text.match(/\b(kras|brca1|brca2|egfr|tp53|pten|myc|alk|ros1|ret|met)\b/i);
        const mutMatch  = text.match(/\b([a-z]\d+[a-z])\b/i);
        const cancerMap = {
            'pancreatic': 'PAAD', 'lung': 'LUAD', 'breast': 'BRCA',
            'colorectal': 'COAD', 'ovarian': 'OV', 'melanoma': 'SKCM'
        };

        if (geneMatch) {
            const geneInput = document.querySelector('input[name="gene"]');
            if (geneInput && !geneInput.value) geneInput.value = geneMatch[1].toUpperCase();
        }
        if (mutMatch) {
            const mutInput = document.querySelector('input[name="mutation"]');
            if (mutInput && !mutInput.value) mutInput.value = mutMatch[1].toUpperCase();
        }
        for (const [keyword, code] of Object.entries(cancerMap)) {
            if (text.includes(keyword)) {
                const cancerInput = document.querySelector('input[name="cancer_code"]');
                if (cancerInput && cancerInput.value === 'PAAD') cancerInput.value = code;
                break;
            }
        }
    });
}
