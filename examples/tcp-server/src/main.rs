use {
    echo_service::Echo, //
    http::Response,
    izanami::no_tls,
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

    izanami::run_tcp("127.0.0.1:5000", no_tls(), echo).unwrap()
}
