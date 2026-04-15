//! DeployWerk API binary — library logic lives in `lib.rs`.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    deploywerk_api::run().await
}
