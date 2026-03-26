import init from './pkg/gravity_well_arena.js';

async function run() {
    await init();
}

run().catch(console.error);
