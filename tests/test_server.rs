#![allow(clippy::redundant_closure)]

use {
    http::{Request, Response},
    izanami_service::Service,
    std::io,
};

#[test]
fn version_sync() {
    version_sync::assert_html_root_url_updated!("src/lib.rs");
}

#[derive(Default)]
struct Echo(());

impl<Ctx> Service<Ctx> for Echo {
    type Response = EchoService;
    type Error = io::Error;
    type Future = futures::future::FutureResult<Self::Response, Self::Error>;

    fn poll_ready(&mut self) -> futures::Poll<(), Self::Error> {
        Ok(().into())
    }

    fn call(&mut self, _: Ctx) -> Self::Future {
        futures::future::ok(EchoService(()))
    }
}

struct EchoService(());

impl<Bd> Service<Request<Bd>> for EchoService {
    type Response = Response<String>;
    type Error = io::Error;
    type Future = futures::future::FutureResult<Self::Response, Self::Error>;

    fn poll_ready(&mut self) -> futures::Poll<(), Self::Error> {
        Ok(().into())
    }

    fn call(&mut self, _: Request<Bd>) -> Self::Future {
        futures::future::ok(Response::builder().body("hello".into()).unwrap())
    }
}

mod tcp {
    use {
        futures::{Future, Stream},
        http::Request,
        hyper::{
            client::{
                connect::{Connect, Connected, Destination},
                Client,
            },
            Body,
        },
        izanami::{tls::no_tls, Server},
        std::{io, net::SocketAddr},
        tokio::{net::TcpStream, runtime::current_thread::Runtime},
    };

    #[test]
    fn tcp_server() -> izanami::Result<()> {
        let mut rt = Runtime::new()?;

        let (server, handle) = Server::bind_tcp(
            super::Echo::default(), //
            "127.0.0.1:0",
            no_tls(),
        )? //
        .build();
        let local_addr = server.local_addr();
        server.spawn(&mut rt);

        let client = Client::builder() //
            .build(TestConnect { local_addr });

        let response = rt.block_on(
            client.request(
                Request::get("http://localhost/") //
                    .body(Body::empty())?,
            ),
        )?;
        assert_eq!(response.status(), 200);

        let body = rt.block_on(response.into_body().concat2())?;
        assert_eq!(body.into_bytes(), "hello");

        rt.block_on(handle.shutdown()).unwrap();

        Ok(())
    }

    struct TestConnect {
        local_addr: SocketAddr,
    }

    impl Connect for TestConnect {
        type Transport = TcpStream;
        type Error = io::Error;
        type Future = Box<
            dyn Future<Item = (Self::Transport, Connected), Error = Self::Error> + Send + 'static,
        >;

        fn connect(&self, _: Destination) -> Self::Future {
            Box::new(
                TcpStream::connect(&self.local_addr) //
                    .map(|stream| (stream, Connected::new())),
            )
        }
    }
}

#[cfg(unix)]
mod unix {
    use {
        futures::{Future, Stream},
        http::Request,
        hyper::{
            client::{
                connect::{Connect, Connected, Destination},
                Client,
            },
            Body,
        },
        izanami::{tls::no_tls, Server},
        std::{io, path::PathBuf},
        tempfile::Builder,
        tokio::{net::UnixStream, runtime::current_thread::Runtime},
    };

    #[test]
    fn unix_server() -> izanami::Result<()> {
        let mut rt = Runtime::new()?;

        let sock_tempdir = Builder::new().prefix("izanami-tests").tempdir()?;
        let sock_path = sock_tempdir.path().join("connect.sock");

        let server = Server::bind_unix(
            super::Echo::default(), //
            &sock_path,
            no_tls(),
        )?
        .start(&mut rt);

        let client = Client::builder() //
            .build(TestConnect {
                sock_path: sock_path.clone(),
            });

        let response = rt.block_on(
            client.request(
                Request::get("http://localhost/") //
                    .body(Body::empty())?,
            ),
        )?;
        assert_eq!(response.status(), 200);

        let body = rt.block_on(response.into_body().concat2())?;
        assert_eq!(body.into_bytes(), "hello");

        rt.block_on(server.shutdown()).unwrap();

        Ok(())
    }

    struct TestConnect {
        sock_path: PathBuf,
    }

    impl Connect for TestConnect {
        type Transport = UnixStream;
        type Error = io::Error;
        type Future = Box<
            dyn Future<Item = (Self::Transport, Connected), Error = Self::Error> + Send + 'static,
        >;

        fn connect(&self, _: Destination) -> Self::Future {
            Box::new(
                UnixStream::connect(&self.sock_path) //
                    .map(|stream| (stream, Connected::new())),
            )
        }
    }
}

#[cfg(feature = "native-tls")]
mod native_tls {
    use {
        ::native_tls::{Certificate, Identity, TlsConnector},
        futures::{Future, Stream},
        http::Request,
        hyper::{
            client::{
                connect::{Connect, Connected, Destination},
                Client,
            },
            Body,
        },
        izanami::Server,
        std::{io, net::SocketAddr},
        tokio::{net::TcpStream, runtime::current_thread::Runtime},
        tokio_tls::{TlsAcceptor, TlsStream},
    };

    #[test]
    fn tls_server() -> izanami::Result<()> {
        let mut rt = Runtime::new()?;

        const IDENTITY: &[u8] = include_bytes!("../test/identity.pfx");
        const CERTIFICATE: &[u8] = include_bytes!("../test/server-crt.pem");

        let tls: TlsAcceptor = {
            let der = Identity::from_pkcs12(IDENTITY, "mypass")?;
            ::native_tls::TlsAcceptor::builder(der).build()?.into()
        };

        let (server, handle) = Server::bind_tcp(
            super::Echo::default(), //
            "127.0.0.1:0",
            tls,
        )?
        .build();
        let local_addr = server.local_addr();
        server.spawn(&mut rt);

        let client = Client::builder() //
            .build(TestConnect {
                local_addr,
                connector: TlsConnector::builder()
                    .add_root_certificate(Certificate::from_pem(CERTIFICATE)?)
                    .build()?
                    .into(),
            });

        let response = rt.block_on(
            client.request(
                Request::get("http://localhost/") //
                    .body(Body::empty())?,
            ),
        )?;
        assert_eq!(response.status(), 200);

        let body = rt.block_on(
            response
                .into_body() //
                .concat2(),
        )?;
        assert_eq!(body.into_bytes(), "hello");

        rt.block_on(handle.shutdown()).unwrap();

        Ok(())
    }

    struct TestConnect {
        local_addr: SocketAddr,
        connector: tokio_tls::TlsConnector,
    }

    impl Connect for TestConnect {
        type Transport = TlsStream<TcpStream>;
        type Error = io::Error;
        type Future = Box<
            dyn Future<Item = (Self::Transport, Connected), Error = Self::Error> + Send + 'static,
        >;

        fn connect(&self, _: Destination) -> Self::Future {
            let connector = self.connector.clone();
            Box::new(
                TcpStream::connect(&self.local_addr)
                    .and_then(move |stream| {
                        connector
                            .connect("localhost", stream)
                            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
                    }) //
                    .map(|stream| (stream, Connected::new())),
            )
        }
    }
}

#[cfg(feature = "openssl")]
mod openssl {
    use {
        futures::{Future, Stream},
        http::Request,
        hyper::{
            client::{
                connect::{Connect, Connected, Destination},
                Client,
            },
            Body,
        },
        izanami::Server,
        openssl::{
            pkey::PKey,
            ssl::{
                SslAcceptor, //
                SslConnector,
                SslMethod,
                SslVerifyMode,
            },
            x509::X509,
        },
        std::{io, net::SocketAddr},
        tokio::{net::TcpStream, runtime::current_thread::Runtime},
        tokio_openssl::{SslConnectorExt, SslStream},
    };

    const CERTIFICATE: &[u8] = include_bytes!("../test/server-crt.pem");
    const PRIVATE_KEY: &[u8] = include_bytes!("../test/server-key.pem");

    #[test]
    fn tls_server() -> izanami::Result<()> {
        let mut rt = Runtime::new()?;

        let cert = X509::from_pem(CERTIFICATE)?;
        let pkey = PKey::private_key_from_pem(PRIVATE_KEY)?;
        let tls = {
            let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls())?;
            builder.set_certificate(&cert)?;
            builder.set_private_key(&pkey)?;
            builder.check_private_key()?;
            builder.build()
        };

        let (server, handle) = Server::bind_tcp(
            super::Echo::default(), //
            "127.0.0.1:0",
            tls,
        )?
        .build();
        let local_addr = server.local_addr();
        server.spawn(&mut rt);

        let client = Client::builder() //
            .build(TestConnect {
                local_addr,
                connector: {
                    let cert = X509::from_pem(CERTIFICATE)?;
                    let pkey = PKey::private_key_from_pem(PRIVATE_KEY)?;
                    let mut builder = SslConnector::builder(SslMethod::tls())?;
                    builder.set_verify(SslVerifyMode::NONE);
                    builder.set_certificate(&cert)?;
                    builder.set_private_key(&pkey)?;
                    builder.build()
                },
            });

        let response = rt.block_on(
            client.request(
                Request::get("http://localhost/") //
                    .body(Body::empty())?,
            ),
        )?;
        assert_eq!(response.status(), 200);

        let body = rt.block_on(
            response
                .into_body() //
                .concat2(),
        )?;
        assert_eq!(body.into_bytes(), "hello");

        rt.block_on(handle.shutdown()).unwrap();

        Ok(())
    }

    struct TestConnect {
        local_addr: SocketAddr,
        connector: SslConnector,
    }

    impl Connect for TestConnect {
        type Transport = SslStream<TcpStream>;
        type Error = io::Error;
        type Future = Box<
            dyn Future<Item = (Self::Transport, Connected), Error = Self::Error> + Send + 'static,
        >;

        fn connect(&self, _: Destination) -> Self::Future {
            let connector = self.connector.clone();
            Box::new(
                TcpStream::connect(&self.local_addr)
                    .and_then(move |stream| {
                        connector
                            .connect_async("localhost", stream)
                            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
                    }) //
                    .map(|stream| (stream, Connected::new())),
            )
        }
    }
}

#[cfg(feature = "rustls")]
mod rustls {
    use {
        ::native_tls::Certificate,
        ::rustls::{NoClientAuth, ServerConfig},
        futures::{Future, Stream},
        http::Request,
        hyper::{
            client::{
                connect::{Connect, Connected, Destination},
                Client,
            },
            Body,
        },
        izanami::Server,
        std::{io, net::SocketAddr, sync::Arc},
        tokio::{net::TcpStream, runtime::current_thread::Runtime},
        tokio_rustls::TlsAcceptor,
        tokio_tls::TlsStream,
    };

    #[test]
    fn tls_server() -> izanami::Result<()> {
        let mut rt = Runtime::new()?;

        const CERTIFICATE: &[u8] = include_bytes!("../test/server-crt.pem");
        const PRIVATE_KEY: &[u8] = include_bytes!("../test/server-key.pem");

        let tls: TlsAcceptor = {
            let certs = {
                let mut reader = io::BufReader::new(io::Cursor::new(CERTIFICATE));
                ::rustls::internal::pemfile::certs(&mut reader)
                    .map_err(|_| failure::format_err!("failed to read certificate file"))?
            };

            let priv_key = {
                let mut reader = io::BufReader::new(io::Cursor::new(PRIVATE_KEY));
                let rsa_keys = {
                    ::rustls::internal::pemfile::rsa_private_keys(&mut reader).map_err(|_| {
                        failure::format_err!("failed to read private key file as RSA")
                    })?
                };
                rsa_keys
                    .into_iter()
                    .next()
                    .ok_or_else(|| failure::format_err!("invalid private key"))?
            };

            let mut config = ServerConfig::new(NoClientAuth::new());
            config.set_single_cert(certs, priv_key)?;

            Arc::new(config).into()
        };

        let (server, handle) = Server::bind_tcp(
            super::Echo::default(), //
            "127.0.0.1:0",
            tls,
        )?
        .build();
        let local_addr = server.local_addr();
        server.spawn(&mut rt);

        // FIXME: use rustls
        let client = Client::builder() //
            .build(TestConnect {
                local_addr,
                connector: ::native_tls::TlsConnector::builder()
                    .add_root_certificate(Certificate::from_pem(CERTIFICATE)?)
                    .build()?
                    .into(),
            });

        let response = rt.block_on(
            client.request(
                Request::get("http://localhost/") //
                    .body(Body::empty())?,
            ),
        )?;
        assert_eq!(response.status(), 200);

        let body = rt.block_on(
            response
                .into_body() //
                .concat2(),
        )?;
        assert_eq!(body.into_bytes(), "hello");

        rt.block_on(handle.shutdown()).unwrap();

        Ok(())
    }

    // FIXME: use rustls
    struct TestConnect {
        local_addr: SocketAddr,
        connector: tokio_tls::TlsConnector,
    }

    impl Connect for TestConnect {
        type Transport = TlsStream<TcpStream>;
        type Error = io::Error;
        type Future = Box<
            dyn Future<Item = (Self::Transport, Connected), Error = Self::Error> + Send + 'static,
        >;

        fn connect(&self, _: Destination) -> Self::Future {
            let connector = self.connector.clone();
            Box::new(
                TcpStream::connect(&self.local_addr)
                    .and_then(move |stream| {
                        connector
                            .connect("localhost", stream)
                            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
                    }) //
                    .map(|stream| (stream, Connected::new())),
            )
        }
    }
}
