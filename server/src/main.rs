use server::routes;
use std::net::SocketAddr;

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
    let api = routes::api();

    let server = warp::serve(api).run(addr);
    server.await;
}
