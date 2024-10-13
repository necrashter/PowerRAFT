use log::LevelFilter;
use pyo3::prelude::*;
use pyo3::wrap_pyfunction;
use server::routes;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::runtime::{Builder, Runtime};
use tokio::task::JoinHandle;

// Struct to hold server state
#[pyclass]
struct Server {
    handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    runtime: Arc<Mutex<Option<Runtime>>>,
}

#[pymethods]
impl Server {
    #[new]
    fn new() -> Self {
        Server {
            handle: Arc::new(Mutex::new(None)),
            runtime: Arc::new(Mutex::new(None)),
        }
    }

    fn start(&self, addr: String) -> PyResult<()> {
        let handle_arc = self.handle.clone();
        let runtime_arc = self.runtime.clone();

        // Parse the address
        let addr: SocketAddr = match addr.parse() {
            Ok(addr) => addr,
            Err(e) => {
                log::error!("Cannot parse the address {}: {}", addr, e);
                return Err(pyo3::exceptions::PyValueError::new_err("Invalid address"));
            }
        };

        // Create the runtime and store it in the Server
        let mut runtime_lock = runtime_arc.lock().unwrap();
        if runtime_lock.is_none() {
            let runtime = Builder::new_multi_thread()
                .enable_all() // Enable IO and time features in the runtime
                .build()
                .map_err(|e| {
                    pyo3::exceptions::PyRuntimeError::new_err(format!(
                        "Failed to create runtime: {}",
                        e
                    ))
                })?;
            *runtime_lock = Some(runtime);
        }

        let runtime = runtime_lock.as_ref().unwrap();
        let api = routes::api();

        // Use the runtime to spawn the server
        let handle = runtime.spawn(async move {
            warp::serve(api).run(addr).await;
        });

        *handle_arc.lock().unwrap() = Some(handle);

        Ok(())
    }

    fn stop(&self) -> PyResult<()> {
        let mut handle_lock = self.handle.lock().unwrap();
        if let Some(handle) = handle_lock.take() {
            handle.abort(); // Abort the server task
        }

        // Also drop the runtime
        let mut runtime_lock = self.runtime.lock().unwrap();
        *runtime_lock = None;

        Ok(())
    }

    fn is_running(&self) -> PyResult<bool> {
        Ok(self.handle.lock().unwrap().is_some())
    }
}

#[pyfunction]
#[pyo3(signature = (level=None))]
fn init_logging(level: Option<String>) -> PyResult<()> {
    let log_level = match level.as_deref() {
        Some("debug") => LevelFilter::Debug,
        Some("warn") => LevelFilter::Warn,
        Some("error") => LevelFilter::Error,
        _ => LevelFilter::Info,
    };

    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or(log_level.to_string()),
    )
    .init();
    Ok(())
}

#[pymodule]
fn pydms(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(init_logging, m)?)?;
    m.add_class::<Server>()?;
    Ok(())
}
