use {
    crate::{
        runtime::{Awaitable, Runtime},
        server::Server,
        service::{
            imp::{ResponseBody, TestService},
            MakeTestService, MockRequestBody,
        },
    },
    bytes::{Buf, Bytes},
    futures::{Async, Future, Poll},
    http::{Request, Response},
    std::{borrow::Cow, str},
};

/// A type that simulates an established connection with a client.
#[derive(Debug)]
pub struct Client<'s, S: MakeTestService> {
    server: &'s mut Server<S>,
    service: S::Service,
}

impl<'s, S> Client<'s, S>
where
    S: MakeTestService,
{
    pub(crate) fn new(server: &'s mut Server<S>, service: S::Service) -> Self {
        Client { server, service }
    }

    /// Applies an HTTP request to this client and await its response.
    pub fn respond<Rt>(
        &mut self,
        request: Request<impl Into<MockRequestBody>>,
    ) -> impl Awaitable<Rt, Ok = AwaitResponse<S>, Error = crate::Error>
    where
        Rt: Runtime<<S::Service as TestService>::Future>,
    {
        #[allow(missing_debug_implementations)]
        struct Respond<S: MakeTestService> {
            future: <S::Service as TestService>::Future,
        }

        impl<S, Rt> Awaitable<Rt> for Respond<S>
        where
            S: MakeTestService,
            Rt: Runtime<<S::Service as TestService>::Future>,
        {
            type Ok = AwaitResponse<S>;
            type Error = crate::Error;

            fn wait(self, rt: &mut Rt) -> Result<Self::Ok, Self::Error> {
                Ok(AwaitResponse {
                    response: rt.block_on(self.future)?,
                })
            }
        }

        Respond {
            future: self.service.call(request.map(Into::into)),
        }
    }
}

/// A type representing the result when the `Future`
/// returned from `S::Service` is completed.
#[allow(missing_debug_implementations)]
pub struct AwaitResponse<S: MakeTestService> {
    response: Response<S::ResponseBody>,
}

impl<S> std::ops::Deref for AwaitResponse<S>
where
    S: MakeTestService,
{
    type Target = Response<S::ResponseBody>;

    fn deref(&self) -> &Self::Target {
        &self.response
    }
}

impl<S> std::ops::DerefMut for AwaitResponse<S>
where
    S: MakeTestService,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.response
    }
}

impl<S> AwaitResponse<S>
where
    S: MakeTestService,
{
    pub fn into_response(self) -> Response<S::ResponseBody> {
        self.response
    }

    /// Converts the internal response body into a `Future` and awaits its result.
    pub fn send_body<Rt>(self) -> impl Awaitable<Rt, Ok = ResponseData, Error = crate::Error>
    where
        Rt: Runtime<SendResponseBody<S::ResponseBody>>,
    {
        #[allow(missing_debug_implementations)]
        struct SendBody<S: MakeTestService> {
            body: S::ResponseBody,
        }

        impl<S, Rt> Awaitable<Rt> for SendBody<S>
        where
            S: MakeTestService,
            Rt: Runtime<SendResponseBody<S::ResponseBody>>,
        {
            type Ok = ResponseData;
            type Error = crate::Error;

            fn wait(self, rt: &mut Rt) -> Result<Self::Ok, Self::Error> {
                let future = SendResponseBody {
                    state: SendResponseBodyState::Init(Some(self.body)),
                };
                rt.block_on(future)
            }
        }

        SendBody::<S> {
            body: self.response.into_body(),
        }
    }
}

#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub struct SendResponseBody<Bd> {
    state: SendResponseBodyState<Bd>,
}

#[allow(missing_debug_implementations)]
enum SendResponseBodyState<Bd> {
    Init(Option<Bd>),
    InFlight { body: Bd, chunks: Vec<Bytes> },
}

impl<Bd> Future for SendResponseBody<Bd>
where
    Bd: ResponseBody,
{
    type Item = ResponseData;
    type Error = Bd::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            self.state = match self.state {
                SendResponseBodyState::Init(ref mut body) => SendResponseBodyState::InFlight {
                    body: body.take().expect("unexpected condition"),
                    chunks: vec![],
                },
                SendResponseBodyState::InFlight {
                    ref mut body,
                    ref mut chunks,
                } => {
                    while let Some(chunk) = futures::try_ready!(body.poll_buf()) {
                        chunks.push(chunk.collect());
                    }
                    return Ok(Async::Ready(ResponseData {
                        chunks: std::mem::replace(chunks, vec![]),
                        _priv: (),
                    }));
                }
            }
        }
    }
}

/// A collection of data generated by the response body.
#[derive(Debug)]
pub struct ResponseData {
    pub chunks: Vec<Bytes>,
    _priv: (),
}

impl ResponseData {
    /// Returns a representation of the chunks as a byte sequence.
    pub fn to_bytes(&self) -> Cow<'_, [u8]> {
        match self.chunks.len() {
            0 => Cow::Borrowed(&[]),
            1 => Cow::Borrowed(&self.chunks[0]),
            _ => Cow::Owned(self.chunks.iter().fold(Vec::new(), |mut acc, chunk| {
                acc.extend_from_slice(&*chunk);
                acc
            })),
        }
    }

    /// Returns a representation of the chunks as an UTF-8 sequence.
    pub fn to_utf8(&self) -> Result<Cow<'_, str>, str::Utf8Error> {
        match self.to_bytes() {
            Cow::Borrowed(bytes) => str::from_utf8(bytes).map(Cow::Borrowed),
            Cow::Owned(bytes) => String::from_utf8(bytes)
                .map_err(|e| e.utf8_error())
                .map(Cow::Owned),
        }
    }
}
