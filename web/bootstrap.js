import init from './pkg/gravity_well_arena.js';

// The game signals it's alive by rendering to the canvas.
// If nothing happens after a timeout, show the browser compatibility message.
let gameStarted = false;

async function run() {
    await init();
    gameStarted = true;
}

function showBrowserBlock() {
    if (gameStarted) return;
    const canvas = document.getElementById('game-canvas');
    const block = document.getElementById('browser-block');
    if (canvas) canvas.style.display = 'none';
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

// Fallback: if the game hasn't started after 8 seconds, assume failure
setTimeout(() => {
    if (!gameStarted) {
        // Check if the canvas has content (non-zero size means wgpu is rendering)
        const canvas = document.querySelector('canvas');
        if (canvas && canvas.width > 0 && canvas.height > 0) {
            gameStarted = true;
            return;
        }
        showBrowserBlock();
    }
}, 8000);
