import init from './pkg/gravity_well_arena.js';

let gameStarted = false;

function showBrowserBlock() {
    if (gameStarted) return;
    document.querySelectorAll('canvas').forEach(c => c.style.display = 'none');
    const block = document.getElementById('browser-block');
    if (block) block.style.display = 'flex';
}

// Catch unhandled errors from WASM (adapter failures, panics)
window.addEventListener('error', (e) => {
    const msg = e.message || '';
    if (msg.includes('No GPU adapter') || msg.includes('unreachable')) {
        e.preventDefault();
        showBrowserBlock();
    }
});
window.addEventListener('unhandledrejection', (e) => {
    const msg = e.reason && (e.reason.message || String(e.reason)) || '';
    if (msg.includes('No GPU adapter') || msg.includes('unreachable')) {
        e.preventDefault();
        showBrowserBlock();
    }
});

async function run() {
    await init();
    gameStarted = true;
}

run().catch((e) => {
    const msg = e && (e.message || String(e)) || '';
    if (msg.includes('exceptions for control flow')) return;
    console.error('Game init failed:', e);
    showBrowserBlock();
});

// Fallback timeout
setTimeout(() => {
    if (!gameStarted) {
        const canvases = document.querySelectorAll('canvas');
        for (const c of canvases) {
            if (c.width > 100 && c.height > 100) {
                gameStarted = true;
                return;
            }
        }
        showBrowserBlock();
    }
}, 3000);
