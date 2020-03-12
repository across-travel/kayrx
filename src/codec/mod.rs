//! Utilities for encoding and decoding frames.
//!
//! Contains adapters to go from streams of bytes, [`AsyncRead`] and
//! [`AsyncWrite`], to framed streams implementing [`Sink`] and [`Stream`].
//! Framed streams are also known as transports.
//!

mod encoder;
mod decoder;
mod framed;
mod framed2;
mod framed_read;
mod framed_write;
mod bytes_codec;
mod lines_codec;
pub mod length_delimited;

pub use bytes_codec::BytesCodec;
pub use decoder::Decoder;
pub use encoder::Encoder;
pub use framed::{Framed, FramedParts};
pub use framed2::{Framed as Framed2, FramedParts as FramedParts2};
pub use framed_read::FramedRead;
pub use framed_write::FramedWrite;
pub use length_delimited::{LengthDelimitedCodec, LengthDelimitedCodecError};
pub use lines_codec::{LinesCodec, LinesCodecError};

