#[tokio::main]
async fn main() {
    if let Err(e) = eurocore::run().await {
        eprintln!("{:#?}", e);
        std::process::exit(1);
    }
}
