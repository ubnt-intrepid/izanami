use http::{Request, Response};
use izanami_hyper::{Events, Server};

async fn app(_: Request<()>, mut events: Events<'_>) -> anyhow::Result<()> {
    events
        .send_response(
            Response::builder() //
                .header("content-length", "14")
                .body("Hello, world!\n")
                .unwrap(),
        )
        .await?;

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = Server::bind("127.0.0.1:4000").await?;
    server.serve(app).await?;

    Ok(())
}