//! Traits, helpers, and type definitions for asynchronous I/O functionality.
//!
//! This module is the asynchronous version of `std::io`. Primarily, it
//! defines two traits, [`AsyncRead`] and [`AsyncWrite`], which are asynchronous
//! versions of the [`Read`] and [`Write`] traits in the standard library.
//!
//! # AsyncRead and AsyncWrite
//!
//! Like the standard library's [`Read`] and [`Write`] traits, [`AsyncRead`] and
//! [`AsyncWrite`] provide the most general interface for reading and writing
//! input and output. Unlike the standard library's traits, however, they are
//! _asynchronous_ &mdash; meaning that reading from or writing to a `kayrx::krse::io`
//! type will _yield_ to the kayrx scheduler when IO is not ready, rather than
//! blocking. This allows other tasks to run while waiting on IO.
//!
//! Another difference is that [`AsyncRead`] and [`AsyncWrite`] only contain
//! core methods needed to provide asynchronous reading and writing
//! functionality. Instead, utility methods are defined in the [`AsyncReadExt`]
//! and [`AsyncWriteExt`] extension traits. These traits are automatically
//! implemented for all values that implement [`AsyncRead`] and [`AsyncWrite`]
//! respectively.
//!
//! End users will rarely interact directly with [`AsyncRead`] and
//! [`AsyncWrite`]. Instead, they will use the async functions defined in the
//! extension traits. Library authors are expected to implement [`AsyncRead`]
//! and [`AsyncWrite`] in order to provide types that behave like byte streams.
//!
//! Even with these differences, kayrx's [`AsyncRead`] and [`AsyncWrite`] traits
//! can be used in almost exactly the same manner as the standard library's
//! `Read` and `Write`. Most types in the standard library that implement `Read`
//! and `Write` have asynchronous equivalents in `kayrx` that implement
//! `AsyncRead` and `AsyncWrite`, such as [`File`] and [`TcpStream`].
//!
//! For example, the standard library documentation introduces `Read` by
//! [demonstrating][std_example] reading some bytes from a [`std::fs::File`]. We
//! can do the same with [`kayrx::krse::fs::File`][`File`]:
//!
//! [`File`]: crate::fs::File
//! [`TcpStream`]: crate::net::TcpStream
//! [`std::fs::File`]: std::fs::File
//! [std_example]: https://doc.rust-lang.org/std/io/index.html#read-and-write
//!
//! ## Buffered Readers and Writers
//!
//! Byte-based interfaces are unwieldy and can be inefficient, as we'd need to be
//! making near-constant calls to the operating system. To help with this,
//! `std::io` comes with [support for _buffered_ readers and writers][stdbuf],
//! and therefore, `kayrx::krse::io` does as well.
//!
//! kayrx provides an async version of the [`std::io::BufRead`] trait,
//! [`AsyncBufRead`]; and async [`BufReader`] and [`BufWriter`] structs, which
//! wrap readers and writers. These wrappers use a buffer, reducing the number
//! of calls and providing nicer methods for accessing exactly what you want.
//!
//! For example, [`BufReader`] works with the [`AsyncBufRead`] trait to add
//! extra methods to any async reader:
//!
//! [`BufWriter`] doesn't add any new ways of writing; it just buffers every call
//! to [`write`](crate::io::AsyncWriteExt::write):
//!
//!
//! [stdbuf]: https://doc.rust-lang.org/std/io/index.html#bufreader-and-bufwriter
//! [`std::io::BufRead`]: std::io::BufRead
//! [`AsyncBufRead`]: crate::io::AsyncBufRead
//! [`BufReader`]: crate::io::BufReader
//! [`BufWriter`]: crate::io::BufWriter
//!
//! ## Implementing AsyncRead and AsyncWrite
//!
//! Because they are traits, we can implement `AsyncRead` and `AsyncWrite` for
//! our own types, as well. Note that these traits must only be implemented for
//! non-blocking I/O types that integrate with the futures type system. In
//! other words, these types must never block the thread, and instead the
//! current task is notified when the I/O resource is ready.
//!
//! # Standard input and output
//!
//! kayrx provides asynchronous APIs to standard [input], [output], and [error].
//! These APIs are very similar to the ones provided by `std`, but they also
//! implement [`AsyncRead`] and [`AsyncWrite`].
//!
//! Note that the standard input / output APIs  **must** be used from the
//! context of the kayrx runtime, as they require kayrx-specific features to
//! function. Calling these functions outside of a kayrx runtime will panic.
//!
//! [input]: fn.stdin.html
//! [output]: fn.stdout.html
//! [error]: fn.stderr.html
//!
//! # `std` re-exports
//!
//! Additionally, [`Error`], [`ErrorKind`], and [`Result`] are re-exported
//! from `std::io` for ease of use.
//!
//! [`AsyncRead`]: trait.AsyncRead.html
//! [`AsyncWrite`]: trait.AsyncWrite.html
//! [`Error`]: struct.Error.html
//! [`ErrorKind`]: enum.ErrorKind.html
//! [`Result`]: type.Result.html
//! [`Read`]: std::io::Read
//! [`Write`]: std::io::Write

pub(crate) mod blocking;
pub(crate) mod driver;
pub(crate) mod slab;
mod poll_evented;
mod registration;
mod async_buf_read;
mod async_read;
mod async_write;
mod async_seek;
mod stderr;
mod stdin;
mod stdout;
mod split;
pub(crate) mod seek;
pub(crate) mod util;

pub use self::poll_evented::PollEvented;
pub use self::registration::Registration;
pub use self::async_read::AsyncRead;
pub use self::async_write::AsyncWrite;
pub use self::async_buf_read::AsyncBufRead;
pub use self::async_seek::AsyncSeek;
pub use self::stderr::{stderr, Stderr};
pub use self::stdin::{stdin, Stdin};
pub use self::stdout::{stdout, Stdout};
pub use self::split::{split, ReadHalf, WriteHalf};
pub use self::seek::Seek;
pub use self::util::{
    copy, empty, repeat, sink, AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, AsyncWriteExt, BufReader,
    BufStream, BufWriter, Copy, Empty, Lines, Repeat, Sink, Split, Take,
};

// Re-export io::Error so that users don't have to deal with conflicts when
// `use`ing `kayrx::krse::io` and `std::io`.
pub use std::io::{Error, ErrorKind, Result};

/// Types in this module can be mocked out in tests.
mod sys {
    // TODO: don't rename
    pub(crate) use crate::fiber::run;
    pub(crate) use crate::fiber::inner::JoinHandle as Blocking;
}

use std::fmt;	

#[derive(Clone, Copy)]	
pub(crate) struct Pack {	
    mask: usize,	
    shift: u32,	
}	

impl Pack {	
    /// Value is packed in the `width` most-significant bits.	
    pub(crate) const fn most_significant(width: u32) -> Pack {	
        let mask = mask_for(width).reverse_bits();	

        Pack {	
            mask,	
            shift: mask.trailing_zeros(),	
        }	
    }	

    /// Value is packed in the `width` least-significant bits.	
    pub(crate) const fn least_significant(width: u32) -> Pack {	
        let mask = mask_for(width);	

        Pack {	
            mask,	
            shift: 0,	
        }	
    }	

    /// Value is packed in the `width` more-significant bits.	
    pub(crate) const fn then(&self, width: u32) -> Pack {	
        let shift = pointer_width() - self.mask.leading_zeros();	
        let mask = mask_for(width) << shift;	

        Pack {	
            mask,	
            shift,	
        }	
    }	

    /// Mask used to unpack value	
    pub(crate) const fn mask(&self) -> usize {	
        self.mask	
    }	

    /// Width, in bits, dedicated to storing the value.	
    pub(crate) const fn width(&self) -> u32 {	
        pointer_width() - (self.mask >> self.shift).leading_zeros()	
    }	

    /// Max representable value	
    pub(crate) const fn max_value(&self) -> usize {	
        (1 << self.width()) - 1	
    }	

    pub(crate) fn pack(&self, value: usize, base: usize) -> usize {	
        assert!(value <= self.max_value());	
        (base & !self.mask) | (value << self.shift)	
    }	

    pub(crate) fn unpack(&self, src: usize) -> usize {	
        unpack(src, self.mask, self.shift)	
    }	
}	

impl fmt::Debug for Pack {	
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {	
        write!(fmt, "Pack {{ mask: {:b}, shift: {} }}", self.mask, self.shift)	
    }	
}	

/// Returns the width of a pointer in bits	
pub(crate) const fn pointer_width() -> u32 {	
    std::mem::size_of::<usize>() as u32 * 8	
}	

/// Returns a `usize` with the right-most `n` bits set.	
pub(crate) const fn mask_for(n: u32) -> usize {	
    let shift = 1usize.wrapping_shl(n - 1);	
    shift | (shift - 1)	
}	

/// Unpack a value using a mask & shift	
pub(crate) const fn unpack(src: usize, mask: usize, shift: u32) -> usize {	
    (src & mask) >> shift	
}