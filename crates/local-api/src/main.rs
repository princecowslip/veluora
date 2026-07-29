use std::sync::Arc;

use application::AppContext;
use local_api::{build_router, generate_token, write_port_file, write_token_file, ApiState};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ctx = Arc::new(AppContext::open_default()?);
    let data_dir = ctx.data_dir.clone();
    let token = generate_token();
    let token_path = write_token_file(&data_dir, &token)?;

    let state = ApiState {
        ctx,
        token: Arc::from(token.as_str()),
    };
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
