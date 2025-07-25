mod config;
mod error;
mod health;
mod server;

use std::panic;
use tracing::{error, info};

#[cfg(all(not(debug_assertions), feature = "demo"))]
compile_error!("Demo mode MUST NOT be enabled in release builds");

fn main() {
    // Set up panic handler as per WORK_PLAN.md section 1.2.2
    panic::set_hook(Box::new(|panic_info| {
        error!(?panic_info, "FATAL: Panic occurred");
        std::process::exit(1);
    }));
    
    info!("PCF API starting up");
    
    // Phase 1: Basic server setup will be implemented in next checkpoint
    info!("Phase 1 foundation complete - ready for server implementation");
}