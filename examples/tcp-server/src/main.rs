use {
    echo_service::Echo, //
    http::Response,
    izanami::server::Server,
};

fn main() {
    let echo = Echo::builder()
        .add_route("/", |_cx| {
            Response::builder() //
                .body("Hello")
                .unwrap()
        })
        .unwrap()
        .build();

    izanami::rt::run(
        Server::bind_tcp("127.0.0.1:5000") //
            .unwrap()
            .serve(echo),
    );
}
