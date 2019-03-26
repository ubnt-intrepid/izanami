//! HTTP/2 connection.

// TODOs:
// * flow control
// * protocol upgrade
// * graceful shutdown of background streams

use {
    crate::BoxedStdError, //
    bytes::{Buf, BufMut, Bytes, BytesMut},
    futures::{stream::FuturesUnordered, try_ready, Async, Future, Poll, Stream},
    http::{Request, Response},
    httpdate::HttpDate,
    izanami_http::{Connection, HttpBody, HttpService},
    std::{error, fmt, time::SystemTime},
    tokio::io::{AsyncRead, AsyncWrite},
};

pub type H2Request = Request<RequestBody>;

/// The error value generated by `H2Connection`.
pub struct H2Error<S: HttpService<RequestBody>> {
    kind: H2ErrorKind<S>,
}

enum H2ErrorKind<S: HttpService<RequestBody>> {
    Protocol(h2::Error),
    Service(S::Error),
    Body(<S::ResponseBody as HttpBody>::Error),
}

/// A builder for creating an `H2Connection`.
#[derive(Debug, Clone)]
pub struct H2 {
    protocol: h2::server::Builder,
}

/// A `Connection` that serves an HTTP/2 connection.
#[allow(missing_debug_implementations)]
pub struct H2Connection<I, S>
where
    S: HttpService<RequestBody>,
{
    state: State<I, S>,
    service: S,
    backgrounds: FuturesUnordered<Background<S>>,
}

#[allow(missing_debug_implementations)]
enum State<I, S: HttpService<RequestBody>> {
    Handshake(h2::server::Handshake<I, SendBuf<<S::ResponseBody as HttpBody>::Data>>),
    Running(h2::server::Connection<I, SendBuf<<S::ResponseBody as HttpBody>::Data>>),
    Closed,
}

#[allow(missing_debug_implementations)]
struct Background<S: HttpService<RequestBody>> {
    state: BackgroundState<S>,
}

#[allow(missing_debug_implementations)]
enum BackgroundState<S: HttpService<RequestBody>> {
    Responding(Respond<S>),
    Sending(SendBody<S>),
}

#[allow(missing_debug_implementations)]
struct Respond<S: HttpService<RequestBody>> {
    respond: S::Respond,
    reply: h2::server::SendResponse<SendBuf<<S::ResponseBody as HttpBody>::Data>>,
}

#[allow(missing_debug_implementations)]
struct SendBody<S: HttpService<RequestBody>> {
    body: S::ResponseBody,
    tx_stream: h2::SendStream<SendBuf<<S::ResponseBody as HttpBody>::Data>>,
    end_of_data: bool,
}

#[allow(missing_debug_implementations)]
enum SendBuf<T> {
    Data(T),
    Eos,
}

/// An `HttpBody` to received the data from client.
#[derive(Debug)]
pub struct RequestBody {
    recv: h2::RecvStream,
}

/// A chunk of bytes received from the client.
#[derive(Debug)]
pub struct Data(Bytes);

// ===== impl H2Error =====

impl<S> H2Error<S>
where
    S: HttpService<RequestBody>,
{
    fn new_protocol(err: h2::Error) -> Self {
        Self {
            kind: H2ErrorKind::Protocol(err),
        }
    }

    fn new_service(err: S::Error) -> Self {
        Self {
            kind: H2ErrorKind::Service(err),
        }
    }

    fn new_body(err: <S::ResponseBody as HttpBody>::Error) -> Self {
        Self {
            kind: H2ErrorKind::Body(err),
        }
    }
}

impl<S> fmt::Debug for H2Error<S>
where
    S: HttpService<RequestBody>,
    S::Error: fmt::Debug,
    <S::ResponseBody as HttpBody>::Error: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_tuple("H2Error");
        match self.kind {
            H2ErrorKind::Protocol(ref err) => f.field(err),
            H2ErrorKind::Service(ref err) => f.field(err),
            H2ErrorKind::Body(ref err) => f.field(err),
        };
        f.finish()
    }
}

impl<S> fmt::Display for H2Error<S>
where
    S: HttpService<RequestBody>,
    S::Error: fmt::Display,
    <S::ResponseBody as HttpBody>::Error: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            H2ErrorKind::Protocol(ref err) => err.fmt(f),
            H2ErrorKind::Service(ref err) => err.fmt(f),
            H2ErrorKind::Body(ref err) => err.fmt(f),
        }
    }
}

impl<S> error::Error for H2Error<S>
where
    S: HttpService<RequestBody>,
    S::Error: error::Error + 'static,
    <S::ResponseBody as HttpBody>::Error: error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self.kind {
            H2ErrorKind::Protocol(ref err) => Some(err),
            H2ErrorKind::Service(ref err) => Some(err),
            H2ErrorKind::Body(ref err) => Some(err),
        }
    }
}

// ===== impl H2Connection =====

impl<I, S> H2Connection<I, S>
where
    I: AsyncRead + AsyncWrite,
    S: HttpService<RequestBody>,
    <S::ResponseBody as HttpBody>::Data: 'static,
{
    fn poll_foreground2(&mut self) -> Poll<(), H2Error<S>> {
        loop {
            self.state = match self.state {
                State::Handshake(ref mut handshake) => {
                    let conn = try_ready!(handshake.poll().map_err(H2Error::new_protocol));
                    State::Running(conn)
                }
                State::Running(ref mut conn) => {
                    if let Async::NotReady =
                        self.service.poll_ready().map_err(H2Error::new_service)?
                    {
                        try_ready!(conn.poll_close().map_err(H2Error::new_protocol));
                        return Ok(Async::Ready(()));
                    }

                    if let Some((req, reply)) =
                        try_ready!(conn.poll().map_err(H2Error::new_protocol))
                    {
                        let req = req.map(|recv| RequestBody { recv });
                        let respond = self.service.respond(req);
                        self.backgrounds.push(Background {
                            state: BackgroundState::Responding(Respond { respond, reply }),
                        });
                        continue;
                    } else {
                        return Ok(Async::Ready(()));
                    }
                }
                State::Closed => return Ok(Async::Ready(())),
            };
        }
    }

    fn poll_foreground(&mut self) -> Poll<(), H2Error<S>> {
        match self.poll_foreground2() {
            Ok(Async::Ready(())) => {
                self.state = State::Closed;
                Ok(Async::Ready(()))
            }
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => {
                self.state = State::Closed;
                Err(e)
            }
        }
    }

    fn poll_background(&mut self) -> Poll<(), H2Error<S>> {
        loop {
            match self.backgrounds.poll() {
                Ok(Async::Ready(Some(()))) => continue,
                Ok(Async::Ready(None)) => return Ok(Async::Ready(())),
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(_err) => {
                    log::trace!("a stream errored");
                    continue;
                }
            }
        }
    }
}

impl<I, S> Connection for H2Connection<I, S>
where
    I: AsyncRead + AsyncWrite,
    S: HttpService<RequestBody>,
    <S::ResponseBody as HttpBody>::Data: 'static,
{
    type Error = H2Error<S>;

    fn poll_close(&mut self) -> Poll<(), Self::Error> {
        let status = self.poll_foreground()?;
        try_ready!(self.poll_background());
        Ok(status)
    }

    fn graceful_shutdown(&mut self) {
        match self.state {
            State::Handshake(..) => (),
            State::Running(ref mut conn) => conn.graceful_shutdown(),
            State::Closed => (),
        }
    }
}

// ===== impl H2 =====

impl H2 {
    /// Creates a new `Builder` with the specified transport.
    pub fn new() -> Self {
        Self {
            protocol: h2::server::Builder::new(),
        }
    }

    /// Returns a mutable reference to the protocol level configuration.
    pub fn protocol(&mut self) -> &mut h2::server::Builder {
        &mut self.protocol
    }

    /// Builds a `H2Connection` with the specified service.
    pub fn serve<I, S>(&self, stream: I, service: S) -> H2Connection<I, S>
    where
        I: AsyncRead + AsyncWrite,
        S: HttpService<RequestBody>,
        <S::ResponseBody as HttpBody>::Data: 'static,
    {
        let handshake = self.protocol.handshake(stream);
        H2Connection {
            state: State::Handshake(handshake),
            service,
            backgrounds: FuturesUnordered::new(),
        }
    }
}

impl Default for H2 {
    fn default() -> Self {
        Self::new()
    }
}

// ===== impl RequestBody =====

impl HttpBody for RequestBody {
    type Data = Data;
    type Error = BoxedStdError;

    fn poll_data(&mut self) -> Poll<Option<Self::Data>, Self::Error> {
        let res = try_ready!(self.recv.poll());
        Ok(Async::Ready(res.map(|data| {
            self.recv
                .release_capacity()
                .release_capacity(data.len())
                .expect("the released capacity should be valid");
            Data(data)
        })))
    }

    fn poll_trailers(&mut self) -> Poll<Option<http::HeaderMap>, Self::Error> {
        self.recv.poll_trailers().map_err(Into::into)
    }

    fn is_end_stream(&self) -> bool {
        self.recv.is_end_stream()
    }
}

// ===== impl Data =====

impl Buf for Data {
    fn remaining(&self) -> usize {
        self.0.len()
    }

    fn bytes(&self) -> &[u8] {
        self.0.as_ref()
    }

    fn advance(&mut self, cnt: usize) {
        self.0.advance(cnt);
    }
}

// ===== impl Background =====

impl<S> Future for Background<S>
where
    S: HttpService<RequestBody>,
{
    type Item = ();
    type Error = H2Error<S>;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            self.state = match self.state {
                BackgroundState::Responding(ref mut respond) => {
                    match try_ready!(respond.poll_send_body()) {
                        Some(send_body) => BackgroundState::Sending(send_body),
                        None => return Ok(Async::Ready(())),
                    }
                }
                BackgroundState::Sending(ref mut send) => return send.poll_send(),
            };
        }
    }
}

// ===== impl Respond =====

impl<S> Respond<S>
where
    S: HttpService<RequestBody>,
{
    fn poll_send_body(&mut self) -> Poll<Option<SendBody<S>>, H2Error<S>> {
        let response = match self.respond.poll() {
            Ok(Async::Ready(res)) => res,
            Ok(Async::NotReady) => {
                if let Async::Ready(reason) =
                    self.reply.poll_reset().map_err(H2Error::new_protocol)?
                {
                    log::debug!(
                        "received RST_STREAM before the response is resolved: {:?}",
                        reason
                    );
                    return Err(H2Error::new_protocol(h2::Error::from(reason)));
                }
                return Ok(Async::NotReady);
            }
            Err(err) => {
                log::debug!("HttpService::respond errored");
                self.reply.send_reset(h2::Reason::INTERNAL_ERROR);
                return Err(H2Error::new_service(err));
            }
        };

        let (parts, body) = response.into_parts();
        let mut response = Response::from_parts(parts, ());

        match response
            .headers_mut()
            .entry(http::header::DATE)
            .expect("DATE is a valid header name")
        {
            http::header::Entry::Occupied(..) => (),
            http::header::Entry::Vacant(entry) => {
                let date = HttpDate::from(SystemTime::now());
                let mut val = BytesMut::new();
                {
                    use std::io::Write;
                    let mut writer = (&mut val).writer();
                    let _ = write!(&mut writer, "{}", date);
                }
                let val = http::header::HeaderValue::from_shared(val.freeze())
                    .expect("formatted HttpDate must be a valid header value.");
                entry.insert(val);
            }
        }

        match response
            .headers_mut()
            .entry(http::header::CONTENT_LENGTH)
            .expect("CONTENT_LENGTH is a valid header name")
        {
            http::header::Entry::Occupied(..) => (),
            http::header::Entry::Vacant(entry) => {
                if let Some(len) = body.content_length() {
                    let mut val = BytesMut::new();
                    {
                        use std::io::Write;
                        let mut writer = (&mut val).writer();
                        let _ = write!(&mut writer, "{}", len);
                    }
                    let val = http::header::HeaderValue::from_shared(val.freeze())
                        .expect("formatted u64 must be a valid header value.");
                    entry.insert(val);
                }
            }
        }

        let end_of_stream = body.is_end_stream();
        let tx_stream = match self.reply.send_response(response, end_of_stream) {
            Ok(tx) => tx,
            Err(e) => {
                log::debug!("send_response errored: {}", e);
                self.reply.send_reset(h2::Reason::INTERNAL_ERROR);
                return Err(H2Error::new_protocol(e));
            }
        };

        if end_of_stream {
            return Ok(Async::Ready(None));
        }

        Ok(Async::Ready(Some(SendBody {
            body,
            tx_stream,
            end_of_data: false,
        })))
    }
}

// ===== impl FlushBody ====

impl<S> SendBody<S>
where
    S: HttpService<RequestBody>,
{
    fn send_data(
        &mut self,
        data: <S::ResponseBody as HttpBody>::Data,
        end_of_stream: bool,
    ) -> Result<(), H2Error<S>> {
        self.tx_stream
            .send_data(SendBuf::Data(data), end_of_stream)
            .map_err(H2Error::new_protocol)
    }

    fn send_eos(&mut self, end_of_stream: bool) -> Result<(), H2Error<S>> {
        self.tx_stream
            .send_data(SendBuf::Eos, end_of_stream)
            .map_err(H2Error::new_protocol)
    }

    fn on_service_error(&mut self, err: <S::ResponseBody as HttpBody>::Error) -> H2Error<S> {
        self.tx_stream.send_reset(h2::Reason::INTERNAL_ERROR);
        H2Error::new_body(err)
    }

    fn poll_send(&mut self) -> Poll<(), H2Error<S>> {
        loop {
            if let Async::Ready(reason) =
                self.tx_stream.poll_reset().map_err(H2Error::new_protocol)?
            {
                log::debug!("received RST_STREAM before sending a frame: {:?}", reason);
                return Err(H2Error::new_protocol(h2::Error::from(reason)));
            }

            if !self.end_of_data {
                match try_ready!(self.body.poll_data().map_err(|e| self.on_service_error(e))) {
                    Some(data) => {
                        let end_of_stream = self.body.is_end_stream();
                        self.send_data(data, end_of_stream)?;
                        if end_of_stream {
                            return Ok(Async::Ready(()));
                        }
                        continue;
                    }
                    None => {
                        self.end_of_data = true;
                        let end_of_stream = self.body.is_end_stream();
                        self.send_eos(end_of_stream)?;
                        if end_of_stream {
                            return Ok(Async::Ready(()));
                        }
                    }
                }
            } else {
                match try_ready!(self
                    .body
                    .poll_trailers()
                    .map_err(|e| self.on_service_error(e)))
                {
                    Some(trailers) => self
                        .tx_stream
                        .send_trailers(trailers)
                        .map_err(H2Error::new_protocol)?,
                    None => self.send_eos(true)?,
                }
                return Ok(Async::Ready(()));
            }
        }
    }
}

// ===== impl SendBuf =====

impl<T: Buf> Buf for SendBuf<T> {
    fn remaining(&self) -> usize {
        match self {
            SendBuf::Data(ref data) => data.remaining(),
            SendBuf::Eos => 0,
        }
    }

    fn bytes(&self) -> &[u8] {
        match self {
            SendBuf::Data(ref data) => data.bytes(),
            SendBuf::Eos => &[],
        }
    }

    fn advance(&mut self, cnt: usize) {
        if let SendBuf::Data(ref mut data) = self {
            data.advance(cnt);
        }
    }
}