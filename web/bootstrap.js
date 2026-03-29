import init from './pkg/gravity_well_arena.js';

async function run() {
    // Double-check WebGPU adapter availability (Brave has navigator.gpu but may block adapter)
    if (navigator.gpu) {
        try {
            const adapter = await navigator.gpu.requestAdapter();
            if (!adapter) {
                showBrowserBlock();
                return;
            }
        } catch (e) {
            showBrowserBlock();
            return;
        }
    }

    await init();
}

function showBrowserBlock() {
    const canvas = document.getElementById('game-canvas');
    const block = document.getElementById('browser-block');
    if (canvas) canvas.style.display = 'none';
    if (block) block.style.display = 'flex';
}

run().catch((e) => {
    console.error(e);
    // If WASM init fails (likely GPU issue), show the browser compatibility message
    showBrowserBlock();
});
