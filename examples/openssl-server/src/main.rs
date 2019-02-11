use {
    echo_service::Echo,
    http::Response,
    izanami::{tls::openssl::Ssl, Http},
    openssl::{pkey::PKey, x509::X509},
};

const CERTIFICATE: &[u8] = include_bytes!("../../../test/server-crt.pem");
const PRIVATE_KEY: &[u8] = include_bytes!("../../../test/server-key.pem");

fn main() -> izanami::Result<()> {
    izanami::system::default(move |sys| {
        let echo = Echo::builder()
            .add_route("/", |_cx| {
                Response::builder() //
                    .body("Hello")
                    .unwrap()
            })? //
            .build();

        let cert = X509::from_pem(CERTIFICATE)?;
        let pkey = PKey::private_key_from_pem(PRIVATE_KEY)?;
        let ssl = Ssl::single_cert(cert, pkey);
        sys.spawn(
            Http::bind("127.0.0.1:4000") //
                .with_tls(ssl)
                .serve(echo)?,
        );

        Ok(())
    })
}
