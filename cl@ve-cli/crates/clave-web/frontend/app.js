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
            showToast('New pending request detected');
            break;
        case 'NoPending':
            // Quiet — no action needed
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
    const dot = document.getElementById('connection-dot');
    const text = document.getElementById('connection-text');
    dot.className = 'dot ' + status;
    const labels = { connected: 'Connected', disconnected: 'Disconnected', reconnecting: 'Reconnecting...' };
    text.textContent = labels[status] || status;
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
    el.innerHTML = '<pre>' + JSON.stringify(data, null, 2) + '</pre>';
}

function showToast(message, type) {
    const toast = document.getElementById('toast');
    toast.textContent = message;
    toast.className = 'toast ' + (type || 'info');
    setTimeout(() => { toast.className = 'toast hidden'; }, 4000);
}

// === Pending Requests ===
function addPendingRequest(data) {
    const list = document.getElementById('pending-list');
    const card = document.createElement('div');
    card.className = 'card';
    card.innerHTML = '<pre>' + JSON.stringify(data, null, 2) + '</pre>';
    list.prepend(card);
}

async function refreshPending() {
    const result = await apiCall('GET', '/pending');
    const list = document.getElementById('pending-list');
    if (result.ok && result.data) {
        list.innerHTML = '<div class="card"><pre>' + JSON.stringify(result.data, null, 2) + '</pre></div>';
    } else {
        list.innerHTML = '<p class="muted">No pending requests</p>';
    }
}

// === Actions ===
async function requestPin() {
    const result = await apiCall('POST', '/pin');
    showResult('pin-result', result);
    if (result.ok) {
        showToast('PIN: ' + result.data.pin);
    }
}

async function activateDevice(e) {
    e.preventDefault();
    const nif = document.getElementById('activate-nif').value;
    const password = document.getElementById('activate-password').value || undefined;
    const result = await apiCall('POST', '/activate', { nif, password });
    showResult('activate-result', result);
    if (result.ok) {
        showToast('Device activated');
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
        document.getElementById('nif-display').textContent = 'NIF: ' + result.data.nif;
    }
}

// === Tab Navigation ===
document.querySelectorAll('.tab').forEach(btn => {
    btn.addEventListener('click', () => {
        document.querySelectorAll('.tab').forEach(b => b.classList.remove('active'));
        document.querySelectorAll('.tab-content').forEach(s => s.classList.remove('active'));
        btn.classList.add('active');
        document.getElementById('tab-' + btn.dataset.tab).classList.add('active');
    });
});

// === Init ===
connectSSE();
loadStatus();
