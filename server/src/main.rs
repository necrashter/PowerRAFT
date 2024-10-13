use server::routes;
use std::net::SocketAddr;

/// Path to static files for the client.
pub const STATIC_PATH: &str = "../pydms/python/pydms/client";

/// Path where graphs are stored.
/// Must end with `/`, or all subdirectory names will start with `/`.
pub const GRAPHS_PATH: &str = "../graphs/";

/// Path where the problems and experiments are stored.
pub const EXPERIMENTS_PATH: &str = "../experiments/";

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let addrstr = "127.0.0.1:8000";
    let addr: SocketAddr = match addrstr.parse() {
        Ok(addr) => addr,
        Err(e) => {
            log::error!("Cannot parse the address {addrstr}: {e}");
            return;
        }
    };
    let api = routes::api(
        STATIC_PATH.to_string(),
        GRAPHS_PATH.to_string(),
        EXPERIMENTS_PATH.to_string(),
    );

    let server = warp::serve(api).run(addr);
    server.await;
}
