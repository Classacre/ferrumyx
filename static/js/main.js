// Ferrumyx Web UI — Main JavaScript

// ── Highlight active nav link ──
document.addEventListener('DOMContentLoaded', () => {
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
