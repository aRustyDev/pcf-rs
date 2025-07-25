use pcf_api;

#[cfg(all(not(debug_assertions), feature = "demo"))]
compile_error!("Demo mode MUST NOT be enabled in release builds");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    pcf_api::run_server().await
}