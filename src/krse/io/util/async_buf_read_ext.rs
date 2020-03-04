use crate::krse::io::util::lines::{lines, Lines};
use crate::krse::io::util::read_line::{read_line, ReadLine};
use crate::krse::io::util::read_until::{read_until, ReadUntil};
use crate::krse::io::util::split::{split, Split};
use crate::krse::io::AsyncBufRead;

    /// An extension trait which adds utility methods to `AsyncBufRead` types.
    pub trait AsyncBufReadExt: AsyncBufRead {
        /// Read all bytes into `buf` until the delimiter `byte` or EOF is reached.
        ///
        /// Equivalent to:
        ///
        /// ```ignore
        /// async fn read_until(&mut self, buf: &mut Vec<u8>) -> io::Result<usize>;
        /// ```
        ///
        /// This function will read bytes from the underlying stream until the
        /// delimiter or EOF is found. Once found, all bytes up to, and including,
        /// the delimiter (if found) will be appended to `buf`.
        ///
        /// If successful, this function will return the total number of bytes read.
        ///
        /// # Errors
        ///
        /// This function will ignore all instances of [`ErrorKind::Interrupted`] and
        /// will otherwise return any errors returned by [`fill_buf`].
        ///
        /// If an I/O error is encountered then all bytes read so far will be
        /// present in `buf` and its length will have been adjusted appropriately.
        ///
        /// [`fill_buf`]: AsyncBufRead::poll_fill_buf
        /// [`ErrorKind::Interrupted`]: std::io::ErrorKind::Interrupted
        ///
        /// # Examples
        ///
        /// [`std::io::Cursor`][`Cursor`] is a type that implements `BufRead`. In
        /// this example, we use [`Cursor`] to read all the bytes in a byte slice
        /// in hyphen delimited segments:
        ///
        /// [`Cursor`]: std::io::Cursor
        ///
        fn read_until<'a>(&'a mut self, byte: u8, buf: &'a mut Vec<u8>) -> ReadUntil<'a, Self>
        where
            Self: Unpin,
        {
            read_until(self, byte, buf)
        }

        /// Read all bytes until a newline (the 0xA byte) is reached, and append
        /// them to the provided buffer.
        ///
        /// Equivalent to:
        ///
        /// ```ignore
        /// async fn read_line(&mut self, buf: &mut String) -> io::Result<usize>;
        /// ```
        ///
        /// This function will read bytes from the underlying stream until the
        /// newline delimiter (the 0xA byte) or EOF is found. Once found, all bytes
        /// up to, and including, the delimiter (if found) will be appended to
        /// `buf`.
        ///
        /// If successful, this function will return the total number of bytes read.
        ///
        /// If this function returns `Ok(0)`, the stream has reached EOF.
        ///
        /// # Errors
        ///
        /// This function has the same error semantics as [`read_until`] and will
        /// also return an error if the read bytes are not valid UTF-8. If an I/O
        /// error is encountered then `buf` may contain some bytes already read in
        /// the event that all data read so far was valid UTF-8.
        ///
        /// [`read_until`]: AsyncBufReadExt::read_until
        ///
        /// # Examples
        ///
        /// [`std::io::Cursor`][`Cursor`] is a type that implements
        /// `AsyncBufRead`. In this example, we use [`Cursor`] to read all the
        /// lines in a byte slice:
        ///
        /// [`Cursor`]: std::io::Cursor
        ///
        fn read_line<'a>(&'a mut self, buf: &'a mut String) -> ReadLine<'a, Self>
        where
            Self: Unpin,
        {
            read_line(self, buf)
        }

        /// Returns a stream of the contents of this reader split on the byte
        /// `byte`.
        ///
        /// This method is the asynchronous equivalent to
        /// [`BufRead::split`](std::io::BufRead::split).
        ///
        /// The stream returned from this function will yield instances of
        /// [`io::Result`]`<`[`Vec<u8>`]`>`. Each vector returned will *not* have
        /// the delimiter byte at the end.
        ///
        /// [`io::Result`]: std::io::Result
        /// [`Vec<u8>`]: std::vec::Vec
        ///
        /// # Errors
        ///
        /// Each item of the stream has the same error semantics as
        /// [`AsyncBufReadExt::read_until`](AsyncBufReadExt::read_until).
        ///
        /// # Examples
        ///
        /// ```
        /// # use kayrx::krse::io::AsyncBufRead;
        /// use kayrx::krse::io::AsyncBufReadExt;
        ///
        /// # async fn dox(my_buf_read: impl AsyncBufRead + Unpin) -> std::io::Result<()> {
        /// let mut segments = my_buf_read.split(b'f');
        ///
        /// while let Some(segment) = segments.next_segment().await? {
        ///     println!("length = {}", segment.len())
        /// }
        /// # Ok(())
        /// # }
        /// ```
        fn split(self, byte: u8) -> Split<Self>
        where
            Self: Sized + Unpin,
        {
            split(self, byte)
        }

        /// Returns a stream over the lines of this reader.
        /// This method is the async equivalent to [`BufRead::lines`](std::io::BufRead::lines).
        ///
        /// The stream returned from this function will yield instances of
        /// [`io::Result`]`<`[`String`]`>`. Each string returned will *not* have a newline
        /// byte (the 0xA byte) or CRLF (0xD, 0xA bytes) at the end.
        ///
        /// [`io::Result`]: std::io::Result
        /// [`String`]: String
        ///
        /// # Errors
        ///
        /// Each line of the stream has the same error semantics as [`AsyncBufReadExt::read_line`].
        ///
        /// # Examples
        ///
        /// [`std::io::Cursor`][`Cursor`] is a type that implements `BufRead`. In
        /// this example, we use [`Cursor`] to iterate over all the lines in a byte
        /// slice.
        ///
        /// [`Cursor`]: std::io::Cursor
        ///
        /// [`AsyncBufReadExt::read_line`]: AsyncBufReadExt::read_line
        fn lines(self) -> Lines<Self>
        where
            Self: Sized,
        {
            lines(self)
        }
    }

impl<R: AsyncBufRead + ?Sized> AsyncBufReadExt for R {}
