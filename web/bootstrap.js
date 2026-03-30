import init from './pkg/gravity_well_arena.js';

let gameStarted = false;

async function run() {
    await init();
    gameStarted = true;
}

function showBrowserBlock() {
    if (gameStarted) return;
    // Hide all canvases (winit may have created one)
    document.querySelectorAll('canvas').forEach(c => c.style.display = 'none');
    const block = document.getElementById('browser-block');
    if (block) block.style.display = 'flex';
}

run().catch((e) => {
    // wasm-bindgen uses exceptions for control flow — ignore those
    if (e && e.message && e.message.includes('exceptions for control flow')) {
        return;
    }
    console.error('Game init failed:', e);
    showBrowserBlock();
});

// Fallback: if the game hasn't started after 3 seconds, assume GPU failure
setTimeout(() => {
    if (!gameStarted) {
        // Check if any canvas is actually rendering
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
