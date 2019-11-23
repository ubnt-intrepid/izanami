use crate::{app::App, eventer::Eventer};
use async_trait::async_trait;
use bytes::{Buf, Bytes};
use futures::future::poll_fn;
use h2::{server::SendResponse, RecvStream, SendStream};
use http::{HeaderMap, Request, Response};
use std::{io, net::ToSocketAddrs};
use tokio::net::TcpListener;

#[derive(Debug)]
pub struct Server {
    listener: TcpListener,
    h2: h2::server::Builder,
}

impl Server {
    pub async fn bind<A>(addr: A) -> io::Result<Self>
    where
        A: ToSocketAddrs,
    {
        let addr = addr.to_socket_addrs()?.next().unwrap();
        let listener = TcpListener::bind(&addr).await?;
        let h2 = h2::server::Builder::new();
        Ok(Self { listener, h2 })
    }

    pub async fn serve<T>(self, app: T) -> io::Result<()>
    where
        T: App + Clone + Send + Sync + 'static,
    {
        let mut listener = self.listener;
        let h2 = self.h2;
        loop {
            if let Ok((socket, _)) = listener.accept().await {
                let app = app.clone();
                let handshake = h2.handshake::<_, Bytes>(socket);
                tokio::spawn(async move {
                    match handshake.await {
                        Ok(mut conn) => {
                            while let Some(request) = conn.accept().await {
                                let (request, respond) = request.unwrap();
                                tokio::spawn(handle_request(app.clone(), request, respond));
                            }
                        }
                        Err(err) => {
                            eprintln!("handshake error: {}", err);
                        }
                    }
                });
            }
        }
    }
}

#[allow(unused_variables)]
async fn handle_request<T>(app: T, request: Request<RecvStream>, sender: SendResponse<Bytes>)
where
    T: App,
{
    let (parts, receiver) = request.into_parts();
    let request = Request::from_parts(parts, ());
    let mut eventer = H2Eventer {
        receiver,
        sender,
        stream: None,
    };

    if let Err(..) = app.call(&request, &mut eventer).await {
        eprintln!("App error");
    }
}

#[derive(Debug)]
pub struct H2Eventer {
    receiver: RecvStream,
    sender: SendResponse<Bytes>,
    stream: Option<SendStream<Bytes>>,
}

#[async_trait]
impl Eventer for H2Eventer {
    type Data = io::Cursor<Bytes>;
    type Error = h2::Error;

    async fn data(&mut self) -> Result<Option<Self::Data>, Self::Error> {
        let data = self.receiver.data().await.transpose()?;
        if let Some(ref data) = data {
            let release_capacity = self.receiver.release_capacity();
            release_capacity.release_capacity(data.len())?;
        }
        Ok(data.map(io::Cursor::new))
    }

    async fn trailers(&mut self) -> Result<Option<HeaderMap>, Self::Error> {
        self.receiver.trailers().await
    }

    async fn start_send_response(&mut self, response: Response<()>) -> Result<(), Self::Error> {
        let stream = self.sender.send_response(response, false)?;
        self.stream.replace(stream);
        Ok(())
    }

    async fn send_data<T>(&mut self, data: T, end_of_stream: bool) -> Result<(), Self::Error>
    where
        T: Buf + Send,
    {
        let stream = self.stream.as_mut().unwrap();

        stream.reserve_capacity(data.remaining());
        poll_fn(|cx| stream.poll_capacity(cx)).await.transpose()?;
        stream.send_data(data.collect(), end_of_stream)?;

        Ok(())
    }

    async fn send_trailers(&mut self, trailers: HeaderMap) -> Result<(), Self::Error> {
        let stream = self.stream.as_mut().unwrap();
        stream.send_trailers(trailers)
    }
}
