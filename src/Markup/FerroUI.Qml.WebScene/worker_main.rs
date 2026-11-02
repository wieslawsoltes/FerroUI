//! Desktop native-engine helper. Browser builds launch their dedicated script worker in JavaScript.
#[cfg(not(target_arch = "wasm32"))]
#[path = "worker.rs"]
mod desktop;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    desktop::main();
}
