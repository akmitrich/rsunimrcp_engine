#![allow(clippy::not_unsafe_ptr_arg_deref)]

use rsunimrcp_sys::uni;
use std::{
    ffi::CStr,
    sync::{atomic::AtomicUsize, Arc},
    time::Duration,
};
use tokio::runtime::{Handle, Runtime};

#[derive(Debug)]
pub struct RawEngine {
    engine: Arc<Engine>,
    channel_counter: AtomicUsize,
}

impl RawEngine {
    pub fn leaked(engine: *mut uni::mrcp_engine_t) -> *mut Self {
        Box::into_raw(Box::new(Self {
            engine: Arc::new(Engine::new(engine)),
            channel_counter: AtomicUsize::new(0),
        }))
    }

    pub fn destroy(this: *mut Self) {
        if this.is_null() {
            return;
        }
        let this = unsafe { Box::from_raw(this) };
        if let Ok(engine) = Arc::try_unwrap(this.engine) {
            engine.shutdown();
            log::info!("Rust Engine stopped. Bye!");
        }
    }

    pub fn engine(&self) -> Arc<Engine> {
        Arc::clone(&self.engine)
    }

    pub fn channel_opened(&mut self) -> usize {
        1 + self
            .channel_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }
}

#[derive(Debug)]
pub struct Engine {
    filename: String,
    runtime: Arc<Runtime>,
}

impl Engine {
    pub fn new(engine: *mut uni::mrcp_engine_t) -> Self {
        let runtime = Runtime::new()
            .inspect_err(|e| {
                log::error!("Could not start Rust Engine due to: {:?}", e);
            })
            .expect("Start async runtime.");
        log::info!("Start runtime: {:?}", runtime);
        Self {
            filename: get_engine_param(engine, b"filename\0".as_ptr() as _).unwrap_or_default(),
            runtime: Arc::new(runtime),
        }
    }

    pub fn shutdown(self) {
        const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(1);
        if let Ok(runtime) = Arc::try_unwrap(self.runtime) {
            runtime.shutdown_timeout(SHUTDOWN_TIMEOUT);
            log::info!("Runtime shuts down in {:?}.", SHUTDOWN_TIMEOUT);
        } else {
            log::error!("Impossible. Runtime is up while engine is destroyed.")
        }
    }

    pub fn async_handle(&self) -> &Handle {
        self.runtime.handle()
    }

    pub fn filename(&self) -> &str {
        &self.filename
    }
}

fn get_engine_param(
    engine: *const uni::mrcp_engine_t,
    param_name: *const std::os::raw::c_char,
) -> Option<String> {
    unsafe {
        let param = uni::mrcp_engine_param_get(engine, param_name);
        if param.is_null() {
            return None;
        }
        CStr::from_ptr(param).to_str().ok().map(ToOwned::to_owned)
    }
}
