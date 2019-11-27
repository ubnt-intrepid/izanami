#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = izanami_h2::Server::bind("127.0.0.1:4000").await?;
    server.serve(izanami_examples::Hello::default()).await?;

    Ok(())
}
