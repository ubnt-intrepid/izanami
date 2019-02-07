//! TLS support.

#[path = "tls/native_tls.rs"]
pub mod native_tls;
pub mod openssl;
pub mod rustls;

use tokio::io::{AsyncRead, AsyncWrite};

/// A trait that represents the conversion of asynchronous I/Os.
///
/// Typically, the implementors of this trait establish a TLS session.
pub trait Acceptor<T> {
    type Accepted: AsyncRead + AsyncWrite;

    /// Converts the supplied I/O object into an `Accepted`.
    ///
    /// The returned I/O from this method includes the handshake process,
    /// and the process will be executed by reading/writing the I/O.
    fn accept(&self, io: T) -> Self::Accepted;
}

/// Create an `Acceptor` using the specified function.
pub fn accept_fn<T, U>(accept: impl Fn(T) -> U) -> impl Acceptor<T, Accepted = U>
where
    U: AsyncRead + AsyncWrite,
{
    #[allow(missing_debug_implementations)]
    struct AcceptFn<F>(F);

    impl<F, T, U> Acceptor<T> for AcceptFn<F>
    where
        F: Fn(T) -> U,
        U: AsyncRead + AsyncWrite,
    {
        type Accepted = U;

        #[inline]
        fn accept(&self, io: T) -> Self::Accepted {
            (self.0)(io)
        }
    }

    AcceptFn(accept)
}