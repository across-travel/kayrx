//! UDP utility types.

mod socket;
mod split;

pub use split::{RecvHalf, SendHalf, ReuniteError};
pub use socket::UdpSocket;
