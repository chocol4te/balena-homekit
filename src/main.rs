#[tokio::main]
async fn main() {
    match client::client().await {
        Ok(()) => (),
        Err(e) => panic!("{:?}", e),
    }
}
