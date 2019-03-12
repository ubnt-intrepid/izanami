#![allow(missing_docs)]

use {
    crate::Service,
    futures::{Async, Future, Poll},
};

#[derive(Debug)]
pub struct With<S1, S2> {
    pub(super) s1: S1,
    pub(super) s2: S2,
}

impl<S1, S2, Req> Service<Req> for With<S1, S2>
where
    S1: Service<Req>,
    S2: Service<Req, Error = S1::Error>,
    Req: Clone,
{
    type Response = (S1::Response, S2::Response);
    type Error = S1::Error;
    type Future = futures::future::Join<S1::Future, S2::Future>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        futures::try_ready!(self.s1.poll_ready());
        futures::try_ready!(self.s2.poll_ready());
        Ok(Async::Ready(()))
    }

    fn call(&mut self, request: Req) -> Self::Future {
        let f1 = self.s1.call(request.clone());
        let f2 = self.s2.call(request);
        f1.join(f2)
    }
}