use std::process::exit;

use clap::Parser;
// use log::{debug, error, log_enabled, info, Level};
use log::debug;

mod model;
use crate::model::*;

mod shared_memory;

mod http_server;
use crate::http_server::*;

mod arguments;
use crate::arguments::*;

fn main() {
    if let Err(e) = run() {
        eprintln!("Error:\n{}", e);
        exit(1);
    }
}

static LINK_FILE_NAME: &str = "/dev/shm/rinha2501_shmem_flink";

fn run() -> Result<(), String> {
    env_logger::init();
    let args = Args::parse();
    let port: usize = std::env::var("PORT")
        .unwrap_or("9999".to_owned())
        .parse()
        .unwrap_or(9999);
    let shared_data = shared_memory::SharedMemory::<Vec<RequestsSummary>>::new(LINK_FILE_NAME).map_err(|e| format!("Failed to create shared memory: {}", e))?;
    shared_data.mutex.set(vec![RequestsSummary::default()])
        .map_err(|e| format!("Failed to set initial value in shared memory: {}", e))?;
    let server = RinhaServer::new(args.server_default.clone(), args.server_fallback.clone(), shared_data.mutex)?
        .start(&format!("0.0.0.0:{port}"))?;
    println!("Server started on http://localhost:{port}. Servers: {}, {}. Press Ctrl+C to stop.", args.server_default, args.server_fallback);
    match server.join() {
        Ok(_) => {
            debug!("Server stopped.");
        }
        Err(e) => {
            return Err(format!("Error waiting for server: {:?}", e));
        }
    }
    Ok(())
}
