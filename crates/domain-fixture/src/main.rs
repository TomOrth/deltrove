//! Process-boundary helper for the domain fixture's lost-response experiment.

use std::{env, error::Error};

use domain_fixture::{DomainFixture, PublishCandidate};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let schema = env::var("DELTROVE_FIXTURE_SCHEMA")?;
    let database_url = env::var("DATABASE_URL")?;
    let fixture = DomainFixture::connect(&database_url, &schema).await?;
    let mut bytes = Vec::new();
    tokio::io::stdin()
        .take(400_000)
        .read_to_end(&mut bytes)
        .await?;
    let command: PublishCandidate = serde_json::from_slice(&bytes)?;
    let outcome = fixture.publish(&command).await?;
    if env::args().any(|arg| arg == "--hold-after-commit") {
        let mut stderr = tokio::io::stderr();
        stderr
            .write_all(b"DEL26_COMMITTED_RESPONSE_WITHHELD\n")
            .await?;
        stderr.flush().await?;
        std::future::pending::<()>().await;
    }
    let mut stdout = tokio::io::stdout();
    stdout.write_all(&serde_json::to_vec(&outcome)?).await?;
    stdout.write_all(b"\n").await?;
    stdout.flush().await?;
    Ok(())
}
