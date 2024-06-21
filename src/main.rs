#[tokio::main]
async fn main() {
    if let Err(e) = euroCore::run().await {
        eprintln!("{:#?}", e);
        std::process::exit(1);
    }
}
