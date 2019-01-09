//! A lightweight implementation of HTTP server for Web frameworks.

#![doc(html_root_url = "https://docs.rs/izanami/0.1.0-preview.1")]
#![deny(
    missing_debug_implementations,
    nonstandard_style,
    rust_2018_idioms,
    rust_2018_compatibility,
    unused
)]
#![forbid(clippy::unimplemented)]

mod error;
mod io;
pub mod rt;
pub mod test;

pub use crate::{
    error::{Error, Result},
    io::{Acceptor, Listener},
};

use {
    futures::{Future, Poll, Stream},
    http::{Request, Response},
    hyper::{
        body::{Body, Payload as _Payload},
        server::conn::Http,
    },
    izanami_buf_stream::{BufStream, IntoBufStream},
    izanami_http::Upgradable,
    izanami_service::{MakeServiceRef, Service},
    std::net::SocketAddr,
};

type CritError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// A struct that represents the stream of chunks from client.
#[derive(Debug)]
pub struct RequestBody(hyper::Body);

impl BufStream for RequestBody {
    type Item = hyper::Chunk;
    type Error = hyper::Error;

    fn poll_buf(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.0.poll_data()
    }

    fn is_end_stream(&self) -> bool {
        self.0.is_end_stream()
    }
}

impl Upgradable for RequestBody {
    type Upgraded = hyper::upgrade::Upgraded;
    type Error = hyper::error::Error;
    type OnUpgrade = hyper::upgrade::OnUpgrade;

    fn on_upgrade(self) -> Self::OnUpgrade {
        self.0.on_upgrade()
    }
}

/// An HTTP server.
#[derive(Debug)]
pub struct Server<L = SocketAddr, A = (), R = tokio::runtime::Runtime> {
    listener: L,
    acceptor: A,
    protocol: Http,
    runtime: Option<R>,
}

impl Server {
    /// Creates an HTTP server using a TCP transport with the address `"127.0.0.1:4000"`.
    pub fn build() -> Self {
        Server::bind(([127, 0, 0, 1], 4000).into())
    }
}

impl<L> Server<L> {
    /// Create a new `Server` with the specified `NewService` and default configuration.
    pub fn bind(listener: L) -> Self {
        Self {
            listener,
            acceptor: (),
            protocol: Http::new(),
            runtime: None,
        }
    }
}

impl<L, A, R> Server<L, A, R> {
    /// Sets the instance of `Acceptor` to the server.
    ///
    /// By default, the raw acceptor is set, which returns the incoming
    /// I/Os directly.
    pub fn acceptor<A2>(self, acceptor: A2) -> Server<L, A2, R>
    where
        L: Listener,
        A2: Acceptor<L::Conn>,
    {
        Server {
            listener: self.listener,
            acceptor,
            protocol: self.protocol,
            runtime: self.runtime,
        }
    }

    /// Sets the HTTP-level configuration to this server.
    ///
    /// Note that the executor will be overwritten by the launcher.
    pub fn protocol(self, protocol: Http) -> Self {
        Self { protocol, ..self }
    }

    /// Sets the instance of runtime to the specified `runtime`.
    pub fn runtime<R2>(self, runtime: R2) -> Server<L, A, R2> {
        Server {
            listener: self.listener,
            acceptor: self.acceptor,
            protocol: self.protocol,
            runtime: Some(runtime),
        }
    }

    /// Switches the runtime to be used to [`current_thread::Runtime`].
    ///
    /// [`current_thread::Runtime`]: https://docs.rs/tokio/0.1/tokio/runtime/current_thread/struct.Runtime.html
    pub fn current_thread(self) -> Server<L, A, tokio::runtime::current_thread::Runtime> {
        Server {
            listener: self.listener,
            acceptor: self.acceptor,
            protocol: self.protocol,
            runtime: None,
        }
    }
}

impl<T, A> Server<T, A, tokio::runtime::Runtime>
where
    T: Listener,
    T::Incoming: Send + 'static,
    A: Acceptor<T::Conn> + Send + 'static,
    A::Accepted: Send + 'static,
{
    pub fn serve<S, Bd>(self, make_service: S) -> crate::Result<()>
    where
        S: MakeServiceRef<A::Accepted, Request<RequestBody>, Response = Response<Bd>>
            + Send
            + Sync
            + 'static,
        S::Error: Into<crate::CritError>,
        S::MakeError: Into<crate::CritError>,
        S::Future: Send + 'static,
        S::Service: Send + 'static,
        <S::Service as Service<Request<RequestBody>>>::Future: Send + 'static,
        Bd: IntoBufStream,
        Bd::Item: Send,
        Bd::Stream: Send + 'static,
        Bd::Error: Into<CritError>,
    {
        let Self {
            listener,
            acceptor,
            runtime,
            protocol,
        } = self;

        let mut runtime = match runtime {
            Some(rt) => rt,
            None => tokio::runtime::Runtime::new()?,
        };

        let incoming = listener
            .listen()
            .map_err(|err| failure::Error::from_boxed_compat(err.into()))?
            .map(move |io| acceptor.accept(io));

        let protocol = protocol.with_executor(tokio::executor::DefaultExecutor::current());

        let serve = hyper::server::Builder::new(incoming, protocol) //
            .serve(LiftedMakeHttpService { make_service })
            .map_err(|e| log::error!("server error: {}", e));

        runtime.spawn(serve);
        runtime.shutdown_on_idle().wait().unwrap();

        Ok(())
    }
}

impl<T, A> Server<T, A, tokio::runtime::current_thread::Runtime>
where
    T: Listener,
    T::Incoming: 'static,
    A: Acceptor<T::Conn> + 'static,
    A::Accepted: Send + 'static,
{
    pub fn serve<S, Bd>(self, make_service: S) -> crate::Result<()>
    where
        S: MakeServiceRef<A::Accepted, Request<RequestBody>, Response = Response<Bd>> + 'static,
        S::Error: Into<crate::CritError>,
        S::MakeError: Into<crate::CritError>,
        S::Future: 'static,
        S::Service: 'static,
        <S::Service as Service<Request<RequestBody>>>::Future: 'static,
        Bd: IntoBufStream,
        Bd::Item: Send,
        Bd::Stream: Send + 'static,
        Bd::Error: Into<CritError>,
    {
        let Self {
            listener,
            acceptor,
            runtime,
            protocol,
        } = self;

        let mut runtime = match runtime {
            Some(rt) => rt,
            None => tokio::runtime::current_thread::Runtime::new()?,
        };

        let incoming = listener
            .listen()
            .map_err(|err| failure::Error::from_boxed_compat(err.into()))?
            .map(move |io| acceptor.accept(io));

        let protocol =
            protocol.with_executor(tokio::runtime::current_thread::TaskExecutor::current());

        let serve = hyper::server::Builder::new(incoming, protocol) //
            .serve(LiftedMakeHttpService { make_service })
            .map_err(|e| log::error!("server error: {}", e));

        runtime.spawn(serve);
        runtime.run()?;

        Ok(())
    }
}

#[allow(missing_debug_implementations)]
struct LiftedMakeHttpService<S> {
    make_service: S,
}

#[allow(clippy::type_complexity)]
impl<'a, S, Ctx, Bd> hyper::service::MakeService<&'a Ctx> for LiftedMakeHttpService<S>
where
    S: MakeServiceRef<Ctx, Request<RequestBody>, Response = Response<Bd>>,
    S::Error: Into<CritError>,
    S::MakeError: Into<CritError>,
    Bd: IntoBufStream,
    Bd::Stream: Send + 'static,
    Bd::Item: Send,
    Bd::Error: Into<CritError>,
{
    type ReqBody = Body;
    type ResBody = WrappedBodyStream<Bd::Stream>;
    type Error = S::Error;
    type Service = LiftedHttpService<S::Service>;
    type MakeError = S::MakeError;
    type Future = futures::future::Map<S::Future, fn(S::Service) -> Self::Service>;

    fn make_service(&mut self, ctx: &'a Ctx) -> Self::Future {
        self.make_service
            .make_service_ref(ctx)
            .map(|service| LiftedHttpService { service })
    }
}

#[allow(missing_debug_implementations)]
struct LiftedHttpService<S> {
    service: S,
}

impl<S, Bd> hyper::service::Service for LiftedHttpService<S>
where
    S: Service<Request<RequestBody>, Response = Response<Bd>>,
    S::Error: Into<crate::CritError>,
    Bd: IntoBufStream,
    Bd::Stream: Send + 'static,
    Bd::Item: Send,
    Bd::Error: Into<CritError>,
{
    type ReqBody = Body;
    type ResBody = WrappedBodyStream<Bd::Stream>;
    type Error = S::Error;
    type Future = LiftedHttpServiceFuture<S::Future>;

    #[inline]
    fn call(&mut self, request: Request<Body>) -> Self::Future {
        LiftedHttpServiceFuture {
            inner: self.service.call(request.map(RequestBody)),
        }
    }
}

#[allow(missing_debug_implementations)]
struct LiftedHttpServiceFuture<Fut> {
    inner: Fut,
}

impl<Fut, Bd> Future for LiftedHttpServiceFuture<Fut>
where
    Fut: Future<Item = Response<Bd>>,
    Bd: IntoBufStream,
    Bd::Stream: Send + 'static,
    Bd::Item: Send,
    Bd::Error: Into<CritError>,
{
    type Item = Response<WrappedBodyStream<Bd::Stream>>;
    type Error = Fut::Error;

    #[inline]
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.inner.poll().map(|x| {
            x.map(|response| response.map(|body| WrappedBodyStream(body.into_buf_stream())))
        })
    }
}

#[allow(missing_debug_implementations)]
pub struct WrappedBodyStream<Bd>(Bd);

impl<Bd> hyper::body::Payload for WrappedBodyStream<Bd>
where
    Bd: BufStream + Send + 'static,
    Bd::Item: Send,
    Bd::Error: Into<CritError>,
{
    type Data = Bd::Item;
    type Error = Bd::Error;

    fn poll_data(&mut self) -> Poll<Option<Self::Data>, Self::Error> {
        self.0.poll_buf()
    }

    fn is_end_stream(&self) -> bool {
        self.0.is_end_stream()
    }
}
