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
pub(crate) mod length_delimited;

pub use self::bytes_codec::BytesCodec;
pub use self::decoder::Decoder;
pub use self::encoder::Encoder;
pub use self::framed::{Framed, FramedParts};
pub use self::framed2::{Framed as Framed2, FramedParts as FramedParts2};
pub use self::framed_read::FramedRead;
pub use self::framed_write::FramedWrite;
pub use self::length_delimited::{LengthDelimitedCodec, LengthDelimitedCodecError};
pub use self::lines_codec::{LinesCodec, LinesCodecError};

