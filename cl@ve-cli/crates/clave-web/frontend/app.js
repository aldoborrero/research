// === SSE Connection ===
let evtSource = null;

function connectSSE() {
    evtSource = new EventSource('/api/events');

    evtSource.onopen = () => {
        setConnectionStatus('connected');
    };

    evtSource.onmessage = (e) => {
        try {
            const event = JSON.parse(e.data);
            handleEvent(event);
        } catch (err) {
            console.error('Failed to parse SSE event:', err);
        }
    };

    evtSource.onerror = () => {
        setConnectionStatus('reconnecting');
    };
}

function handleEvent(event) {
    switch (event.type) {
        case 'PendingData':
            addPendingRequest(event.data);
            showToast('New pending request detected', 'info');
            break;
        case 'NoPending':
            break;
        case 'ListenerStarted':
            setConnectionStatus('connected');
            break;
        case 'ListenerStopped':
            setConnectionStatus('disconnected');
            break;
        case 'PollError':
            showToast('Poll error: ' + (event.data?.message || 'Unknown'), 'error');
            break;
        case 'Lagged':
            console.warn('Missed', event.missed, 'events');
            break;
    }
}

function setConnectionStatus(status) {
    const ids = ['connection-dot', 'connection-dot-mobile'];
    const textIds = ['connection-text', 'connection-text-mobile'];
    const labels = { connected: 'Connected', disconnected: 'Disconnected', reconnecting: 'Reconnecting...' };

    ids.forEach(id => {
        const el = document.getElementById(id);
        if (el) {
            el.className = el.className.replace(/connected|disconnected|reconnecting/g, '').trim();
            el.classList.add(status);
        }
    });
    textIds.forEach(id => {
        const el = document.getElementById(id);
        if (el) el.textContent = labels[status] || status;
    });
}

// === API Helpers ===
async function apiCall(method, path, body) {
    const opts = { method, headers: { 'Content-Type': 'application/json' } };
    if (body) opts.body = JSON.stringify(body);
    const res = await fetch('/api' + path, opts);
    return res.json();
}

function showResult(elementId, data) {
    const el = document.getElementById(elementId);
    if (!el) return;

    if (data.ok && data.data) {
        const content = typeof data.data === 'object' ? data.data : data;
        el.innerHTML = `
            <div class="bg-white rounded-xl border border-gray-200 shadow-sm overflow-hidden">
                <div class="px-4 py-3 bg-emerald-50 border-b border-emerald-100 flex items-center gap-2">
                    <svg class="w-4 h-4 text-emerald-600" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/></svg>
                    <span class="text-sm font-medium text-emerald-800">Success</span>
                </div>
                <pre class="result-block">${escapeHtml(JSON.stringify(content, null, 2))}</pre>
            </div>`;
    } else {
        const errorMsg = data.error || 'Unknown error';
        el.innerHTML = `
            <div class="bg-white rounded-xl border border-red-200 shadow-sm overflow-hidden">
                <div class="px-4 py-3 bg-red-50 border-b border-red-100 flex items-center gap-2">
                    <svg class="w-4 h-4 text-red-600" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
                    <span class="text-sm font-medium text-red-800">Error</span>
                </div>
                <pre class="result-block">${escapeHtml(JSON.stringify(data, null, 2))}</pre>
            </div>`;
    }
}

function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
}

function showToast(message, type) {
    const toast = document.getElementById('toast');
    const colors = {
        info: 'bg-aeat-600',
        error: 'bg-red-600',
        success: 'bg-emerald-600',
    };
    const bg = colors[type] || colors.info;

    toast.className = `fixed bottom-6 right-6 z-50 max-w-sm`;
    toast.innerHTML = `
        <div class="${bg} text-white px-4 py-3 rounded-lg shadow-lg flex items-center gap-3 animate-slide-up">
            <span class="text-sm">${escapeHtml(message)}</span>
        </div>`;

    setTimeout(() => {
        toast.className = 'fixed bottom-6 right-6 z-50 hidden max-w-sm';
    }, 4000);
}

// === Pending Requests ===
function addPendingRequest(data) {
    const list = document.getElementById('pending-list');
    const card = document.createElement('div');
    card.className = 'bg-white rounded-xl border border-blue-200 shadow-sm overflow-hidden ring-1 ring-blue-100';
    card.innerHTML = `
        <div class="px-4 py-3 bg-blue-50 border-b border-blue-100 flex items-center gap-2">
            <svg class="w-4 h-4 text-blue-600 animate-pulse" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9"/></svg>
            <span class="text-sm font-medium text-blue-800">New Request</span>
            <span class="ml-auto text-xs text-blue-500">${new Date().toLocaleTimeString()}</span>
        </div>
        <pre class="result-block">${escapeHtml(JSON.stringify(data, null, 2))}</pre>`;
    list.prepend(card);
}

async function refreshPending() {
    const result = await apiCall('GET', '/pending');
    const list = document.getElementById('pending-list');
    if (result.ok && result.data) {
        list.innerHTML = `
            <div class="bg-white rounded-xl border border-gray-200 shadow-sm overflow-hidden">
                <pre class="result-block">${escapeHtml(JSON.stringify(result.data, null, 2))}</pre>
            </div>`;
    } else {
        list.innerHTML = `
            <div class="text-center py-12 text-gray-400">
                <svg class="w-12 h-12 mx-auto mb-3 opacity-40" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4"/></svg>
                <p class="text-sm font-medium">No pending requests</p>
                <p class="text-xs mt-1">Requests will appear here when detected</p>
            </div>`;
    }
}

// === Actions ===
async function requestPin() {
    const result = await apiCall('POST', '/pin');
    showResult('pin-result', result);
    if (result.ok && result.data?.pin) {
        showToast('PIN generated: ' + result.data.pin, 'success');
    }
}

async function activateDevice(e) {
    e.preventDefault();
    const nif = document.getElementById('activate-nif').value;
    const password = document.getElementById('activate-password').value || undefined;
    const result = await apiCall('POST', '/activate', { nif, password });
    showResult('activate-result', result);
    if (result.ok) {
        showToast('Device activated successfully', 'success');
        loadStatus();
    }
}

async function loadHistory() {
    const result = await apiCall('GET', '/history');
    showResult('history-result', result);
}

async function loadMyData() {
    const result = await apiCall('GET', '/my-data');
    showResult('mydata-result', result);
}

async function qrAuth(e) {
    e.preventDefault();
    const value = document.getElementById('qr-value').value;
    const result = await apiCall('POST', '/qr-auth', { value });
    showResult('qr-result', result);
}

async function loadStatus() {
    const result = await apiCall('GET', '/status');
    showResult('status-result', result);
    if (result.ok && result.data?.nif) {
        const nifEl = document.getElementById('nif-display');
        if (nifEl) nifEl.textContent = 'NIF: ' + result.data.nif;
    }
}

// === Tab Navigation ===
function switchTab(tabName) {
    // Update desktop nav
    document.querySelectorAll('.nav-btn').forEach(b => b.classList.remove('active'));
    document.querySelectorAll('.nav-btn-mobile').forEach(b => b.classList.remove('active'));
    document.querySelectorAll('.tab-content').forEach(s => {
        s.classList.add('hidden');
    });

    // Activate clicked tab
    document.querySelectorAll(`[data-tab="${tabName}"]`).forEach(b => b.classList.add('active'));
    const section = document.getElementById('tab-' + tabName);
    if (section) section.classList.remove('hidden');
}

document.querySelectorAll('[data-tab]').forEach(btn => {
    btn.addEventListener('click', () => switchTab(btn.dataset.tab));
});

// === Init ===
connectSSE();
loadStatus();
