// === SSE Connection ===
let evtSource = null;
let pendingCount = 0;

function connectSSE() {
    evtSource = new EventSource('/api/events');

    evtSource.onopen = () => setConnectionStatus('connected');

    evtSource.onmessage = (e) => {
        try {
            handleEvent(JSON.parse(e.data));
        } catch (err) {
            console.error('Failed to parse SSE event:', err);
        }
    };

    evtSource.onerror = () => setConnectionStatus('reconnecting');
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
    const configs = {
        connected:    { color: 'bg-green-500', text: 'Connected',       badgeBg: 'bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-300' },
        disconnected: { color: 'bg-red-500',   text: 'Disconnected',    badgeBg: 'bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-300' },
        reconnecting: { color: 'bg-yellow-400 animate-pulse', text: 'Reconnecting...', badgeBg: 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-300' },
    };
    const cfg = configs[status] || configs.disconnected;

    // Desktop badge
    const desktopBadge = document.getElementById('status-badge-desktop');
    if (desktopBadge) {
        desktopBadge.innerHTML = `
            <span class="inline-flex items-center gap-1.5 px-2.5 py-1 text-xs font-medium rounded-full ${cfg.badgeBg}">
                <span class="w-2 h-2 rounded-full ${cfg.color}"></span>
                ${cfg.text}
            </span>`;
    }

    // Mobile badge
    const mobileBadge = document.getElementById('status-badge-mobile');
    if (mobileBadge) {
        mobileBadge.innerHTML = `
            <span class="inline-flex items-center gap-1.5 px-2.5 py-1 text-xs font-medium rounded-full ${cfg.badgeBg}">
                <span class="w-2 h-2 rounded-full ${cfg.color}"></span>
                ${cfg.text}
            </span>`;
    }
}

// === API Helpers ===
async function apiCall(method, path, body) {
    const opts = { method, headers: { 'Content-Type': 'application/json' } };
    if (body) opts.body = JSON.stringify(body);
    const res = await fetch('/api' + path, opts);
    return res.json();
}

function escapeHtml(str) {
    const div = document.createElement('div');
    div.textContent = str;
    return div.innerHTML;
}

function showResult(elementId, data) {
    const el = document.getElementById(elementId);
    if (!el) return;

    if (data.ok && data.data) {
        const content = typeof data.data === 'object' ? data.data : data;
        el.innerHTML = `
            <div class="p-4 rounded-lg border border-green-200 bg-green-50 dark:bg-gray-800 dark:border-green-800">
                <div class="flex items-center gap-2 mb-3">
                    <svg class="w-4 h-4 text-green-600 dark:text-green-400 shrink-0" fill="currentColor" viewBox="0 0 20 20"><path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.857-9.809a.75.75 0 00-1.214-.882l-3.483 4.79-1.88-1.88a.75.75 0 10-1.06 1.061l2.5 2.5a.75.75 0 001.137-.089l4-5.5z" clip-rule="evenodd"/></svg>
                    <span class="text-sm font-semibold text-green-800 dark:text-green-400">Success</span>
                </div>
                <pre class="bg-gray-900 text-gray-200 p-4 rounded-lg text-xs font-mono leading-relaxed overflow-x-auto">${escapeHtml(JSON.stringify(content, null, 2))}</pre>
            </div>`;
    } else {
        el.innerHTML = `
            <div class="p-4 rounded-lg border border-red-200 bg-red-50 dark:bg-gray-800 dark:border-red-800">
                <div class="flex items-center gap-2 mb-3">
                    <svg class="w-4 h-4 text-red-600 dark:text-red-400 shrink-0" fill="currentColor" viewBox="0 0 20 20"><path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.28 7.22a.75.75 0 00-1.06 1.06L8.94 10l-1.72 1.72a.75.75 0 101.06 1.06L10 11.06l1.72 1.72a.75.75 0 101.06-1.06L11.06 10l1.72-1.72a.75.75 0 00-1.06-1.06L10 8.94 8.28 7.22z" clip-rule="evenodd"/></svg>
                    <span class="text-sm font-semibold text-red-800 dark:text-red-400">Error</span>
                </div>
                <pre class="bg-gray-900 text-gray-200 p-4 rounded-lg text-xs font-mono leading-relaxed overflow-x-auto">${escapeHtml(JSON.stringify(data, null, 2))}</pre>
            </div>`;
    }
}

// === Flowbite-style Toasts ===
function showToast(message, type) {
    const container = document.getElementById('toast-container');
    const id = 'toast-' + Date.now();

    const icons = {
        info: `<div class="inline-flex items-center justify-center shrink-0 w-8 h-8 text-blue-500 bg-blue-100 rounded-lg dark:bg-blue-800 dark:text-blue-200">
                   <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>
               </div>`,
        error: `<div class="inline-flex items-center justify-center shrink-0 w-8 h-8 text-red-500 bg-red-100 rounded-lg dark:bg-red-800 dark:text-red-200">
                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
                </div>`,
        success: `<div class="inline-flex items-center justify-center shrink-0 w-8 h-8 text-green-500 bg-green-100 rounded-lg dark:bg-green-800 dark:text-green-200">
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"/></svg>
                  </div>`,
    };

    const toast = document.createElement('div');
    toast.id = id;
    toast.className = 'flex items-center w-full max-w-sm p-4 text-gray-500 bg-white rounded-lg shadow-lg border border-gray-100 dark:text-gray-400 dark:bg-gray-800 dark:border-gray-700 animate-slide-up';
    toast.setAttribute('role', 'alert');
    toast.innerHTML = `
        ${icons[type] || icons.info}
        <div class="ms-3 text-sm font-normal">${escapeHtml(message)}</div>
        <button type="button" class="ms-auto -mx-1.5 -my-1.5 bg-white text-gray-400 hover:text-gray-900 rounded-lg focus:ring-2 focus:ring-gray-300 p-1.5 hover:bg-gray-100 inline-flex items-center justify-center h-8 w-8 dark:text-gray-500 dark:hover:text-white dark:bg-gray-800 dark:hover:bg-gray-700" onclick="dismissToast('${id}')">
            <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/></svg>
        </button>`;

    container.appendChild(toast);

    setTimeout(() => dismissToast(id), 5000);
}

function dismissToast(id) {
    const toast = document.getElementById(id);
    if (toast) {
        toast.style.opacity = '0';
        toast.style.transform = 'translateX(1rem)';
        toast.style.transition = 'all 0.3s ease-out';
        setTimeout(() => toast.remove(), 300);
    }
}

// === Pending Requests ===
function updatePendingBadge() {
    const badge = document.getElementById('pending-badge');
    if (!badge) return;
    if (pendingCount > 0) {
        badge.textContent = pendingCount > 9 ? '9+' : pendingCount;
        badge.classList.remove('hidden');
        badge.classList.add('flex');
    } else {
        badge.classList.add('hidden');
        badge.classList.remove('flex');
    }
}

function addPendingRequest(data) {
    const list = document.getElementById('pending-list');
    const empty = document.getElementById('pending-empty');
    if (empty) empty.classList.add('hidden');

    pendingCount++;
    updatePendingBadge();

    const card = document.createElement('div');
    card.className = 'bg-white dark:bg-gray-800 rounded-lg border border-blue-200 dark:border-blue-800 shadow-sm overflow-hidden';
    card.innerHTML = `
        <div class="px-4 py-3 bg-blue-50 dark:bg-blue-900/30 border-b border-blue-200 dark:border-blue-800 flex items-center">
            <div class="flex items-center gap-2">
                <span class="relative flex h-3 w-3">
                    <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-blue-400 opacity-75"></span>
                    <span class="relative inline-flex rounded-full h-3 w-3 bg-blue-500"></span>
                </span>
                <span class="text-sm font-semibold text-blue-800 dark:text-blue-300">New Request</span>
            </div>
            <span class="ml-auto inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-300">
                ${new Date().toLocaleTimeString()}
            </span>
        </div>
        <pre class="bg-gray-900 text-gray-200 p-4 text-xs font-mono leading-relaxed overflow-x-auto rounded-b-lg">${escapeHtml(JSON.stringify(data, null, 2))}</pre>`;
    list.prepend(card);
}

async function refreshPending() {
    const result = await apiCall('GET', '/pending');
    const list = document.getElementById('pending-list');
    const empty = document.getElementById('pending-empty');

    if (result.ok && result.data) {
        if (empty) empty.classList.add('hidden');
        list.innerHTML = `
            <div class="bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 shadow-sm overflow-hidden">
                <pre class="bg-gray-900 text-gray-200 p-4 text-xs font-mono leading-relaxed overflow-x-auto rounded-lg">${escapeHtml(JSON.stringify(result.data, null, 2))}</pre>
            </div>`;
    } else {
        list.innerHTML = '';
        if (empty) empty.classList.remove('hidden');
        pendingCount = 0;
        updatePendingBadge();
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
    document.querySelectorAll('.nav-btn').forEach(b => b.classList.remove('active'));
    document.querySelectorAll('.tab-content').forEach(s => s.classList.add('hidden'));

    document.querySelectorAll(`[data-tab="${tabName}"]`).forEach(b => b.classList.add('active'));
    const section = document.getElementById('tab-' + tabName);
    if (section) section.classList.remove('hidden');

    // Close mobile drawer
    const drawerEl = document.getElementById('sidebar');
    if (drawerEl && window.innerWidth < 768) {
        const drawerInstance = FlowbiteInstances.getInstance('Drawer', 'sidebar');
        if (drawerInstance) drawerInstance.hide();
    }
}

document.querySelectorAll('[data-tab]').forEach(btn => {
    btn.addEventListener('click', () => switchTab(btn.dataset.tab));
});

// === Init ===
connectSSE();
loadStatus();
