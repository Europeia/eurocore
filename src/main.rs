#[tokio::main]
async fn main() {
    // console_subscriber::init();

    if let Err(e) = eurocore::run().await {
        eprintln!("{:?}", e);
        std::process::exit(1);
    }
}
