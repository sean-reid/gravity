import init from './pkg/gravity_well_arena.js';

let gameStarted = false;

function showBrowserBlock() {
    if (gameStarted) return;
    document.querySelectorAll('canvas').forEach(c => c.style.display = 'none');
    const block = document.getElementById('browser-block');
    if (block) block.style.display = 'flex';
}

// Catch ALL unhandled errors — if the game hasn't started, assume GPU failure
window.addEventListener('error', (e) => {
    if (!gameStarted) {
        e.preventDefault();
        showBrowserBlock();
    }
});
window.addEventListener('unhandledrejection', (e) => {
    if (!gameStarted) {
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
