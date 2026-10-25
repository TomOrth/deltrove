use std::{error::Error, time::Duration};

use deltrove_api::app;
use tokio::{net::TcpListener, time::timeout};

#[tokio::test]
async fn readiness_is_served_over_a_real_loopback_connection() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move { axum::serve(listener, app()).await });

    let response = timeout(
        Duration::from_secs(2),
        reqwest::get(format!("http://{address}/health/ready")),
    )
    .await??;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let payload: serde_json::Value = timeout(Duration::from_secs(2), response.json()).await??;
    assert_eq!(payload["status"], "ready");

    server.abort();
    let _ = server.await;
    Ok(())
}
