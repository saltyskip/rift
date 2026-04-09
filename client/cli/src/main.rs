#[tokio::main]
async fn main() {
    rift_cli::error::run_main(rift_cli::run().await);
}
