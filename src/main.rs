fn main() {
    env_logger::init();

    #[cfg(not(target_arch = "wasm32"))]
    gravity_well_arena::platform::native::run_native();
}
