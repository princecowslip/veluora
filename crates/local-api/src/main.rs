use std::sync::Arc;
use std::time::Duration;

use application::AppContext;
use local_api::{
    build_router, generate_token, recover_and_resume_downloads, write_port_file, write_token_file,
    ApiState,
};
use tokio::net::TcpListener;

/// How often the periodic recovery task re-checks for a download that
/// went stale *while this process is running* — e.g. one that crashed
/// once already, just under the staleness threshold, before this
/// instance even started. See `recover_and_resume_downloads`.
const RECOVERY_RECHECK_INTERVAL: Duration = Duration::from_secs(60);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = Arc::new(AppContext::open_default()?);
    let data_dir = ctx.data_dir.clone();
    let token = generate_token();
    let token_path = write_token_file(&data_dir, &token)?;

    let state = ApiState::new(ctx, Arc::from(token.as_str()))?;

    let recovered = recover_and_resume_downloads(&state.ctx, &state.download_semaphore)?;
    if !recovered.is_empty() {
        println!("recovered {} interrupted download(s)", recovered.len());
    }
    {
        let ctx = state.ctx.clone();
        let download_semaphore = state.download_semaphore.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(RECOVERY_RECHECK_INTERVAL);
            interval.tick().await; // first tick fires immediately; skip it
            loop {
                interval.tick().await;
                let _ = recover_and_resume_downloads(&ctx, &download_semaphore);
            }
        });
    }

    let app = build_router(state);

    // Loopback only, always — see docs/19-local-api.md.
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let port_path = write_port_file(&data_dir, addr.port())?;
    println!("veloura local-api listening on http://{addr}/api/v1");
    println!("auth token written to {}", token_path.display());
    println!("port written to {}", port_path.display());

    axum::serve(listener, app).await?;
    Ok(())
}
