use kayrx::web::multipart::dev::*;
use kayrx::http::h1::Payload;
use kayrx::krse::sync::local::mpsc;
use std::task::{Context, Poll};
use kayrx::krse::stream::{Stream, StreamExt};
use std::pin::Pin;
use kayrx::http::header::{self, HeaderMap, DispositionParam, DispositionType};
use bytes::{Bytes, BytesMut};
use futures_util::future::lazy;
use kayrx::http::error::*;

#[kayrx::test]
async fn test_boundary() {
    let headers = HeaderMap::new();
    match Multipart::boundary(&headers) {
        Err(MultipartError::NoContentType) => (),
        _ => unreachable!("should not happen"),
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("test"),
    );

    match Multipart::boundary(&headers) {
        Err(MultipartError::ParseContentType) => (),
        _ => unreachable!("should not happen"),
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("multipart/mixed"),
    );
    match Multipart::boundary(&headers) {
        Err(MultipartError::Boundary) => (),
        _ => unreachable!("should not happen"),
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static(
            "multipart/mixed; boundary=\"5c02368e880e436dab70ed54e1c58209\"",
        ),
    );

    assert_eq!(
        Multipart::boundary(&headers).unwrap(),
        "5c02368e880e436dab70ed54e1c58209"
    );
}

fn create_stream() -> (
    mpsc::Sender<Result<Bytes, PayloadError>>,
    impl Stream<Item = Result<Bytes, PayloadError>>,
) {
    let (tx, rx) = mpsc::channel();

    (tx, rx.map(|res| res.map_err(|_| panic!())))
}
// Stream that returns from a Bytes, one char at a time and Pending every other poll()
struct SlowStream {
    bytes: Bytes,
    pos: usize,
    ready: bool,
}

impl SlowStream {
    fn new(bytes: Bytes) -> SlowStream {
        return SlowStream {
            bytes: bytes,
            pos: 0,
            ready: false,
        };
    }
}

impl Stream for SlowStream {
    type Item = Result<Bytes, PayloadError>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        if !this.ready {
            this.ready = true;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        if this.pos == this.bytes.len() {
            return Poll::Ready(None);
        }
        let res = Poll::Ready(Some(Ok(this.bytes.slice(this.pos..(this.pos + 1)))));
        this.pos += 1;
        this.ready = false;
        res
    }
}

fn create_simple_request_with_header() -> (Bytes, HeaderMap) {
    let bytes = Bytes::from(
        "testasdadsad\r\n\
         --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"fn.txt\"\r\n\
         Content-Type: text/plain; charset=utf-8\r\nContent-Length: 4\r\n\r\n\
         test\r\n\
         --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
         Content-Type: text/plain; charset=utf-8\r\nContent-Length: 4\r\n\r\n\
         data\r\n\
         --abbc761f78ff4d7cb7573b5a23f96ef0--\r\n",
    );
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static(
            "multipart/mixed; boundary=\"abbc761f78ff4d7cb7573b5a23f96ef0\"",
        ),
    );
    (bytes, headers)
}

#[kayrx::test]
async fn test_multipart_no_end_crlf() {
    let (sender, payload) = create_stream();
    let (mut bytes, headers) = create_simple_request_with_header();
    let bytes_stripped = bytes.split_to(bytes.len()); // strip crlf

    sender.send(Ok(bytes_stripped)).unwrap();
    drop(sender); // eof

    let mut multipart = Multipart::new(&headers, payload);

    match multipart.next().await.unwrap() {
        Ok(_) => (),
        _ => unreachable!(),
    }

    match multipart.next().await.unwrap() {
        Ok(_) => (),
        _ => unreachable!(),
    }

    match multipart.next().await {
        None => (),
        _ => unreachable!(),
    }
}

#[kayrx::test]
async fn test_multipart() {
    let (sender, payload) = create_stream();
    let (bytes, headers) = create_simple_request_with_header();

    sender.send(Ok(bytes)).unwrap();

    let mut multipart = Multipart::new(&headers, payload);
    match multipart.next().await {
        Some(Ok(mut field)) => {
            let cd = field.content_disposition().unwrap();
            assert_eq!(cd.disposition, DispositionType::FormData);
            assert_eq!(cd.parameters[0], DispositionParam::Name("file".into()));

            assert_eq!(field.content_type().type_(), mime::TEXT);
            assert_eq!(field.content_type().subtype(), mime::PLAIN);

            match field.next().await.unwrap() {
                Ok(chunk) => assert_eq!(chunk, "test"),
                _ => unreachable!(),
            }
            match field.next().await {
                None => (),
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    }

    match multipart.next().await.unwrap() {
        Ok(mut field) => {
            assert_eq!(field.content_type().type_(), mime::TEXT);
            assert_eq!(field.content_type().subtype(), mime::PLAIN);

            match field.next().await {
                Some(Ok(chunk)) => assert_eq!(chunk, "data"),
                _ => unreachable!(),
            }
            match field.next().await {
                None => (),
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    }

    match multipart.next().await {
        None => (),
        _ => unreachable!(),
    }
}

// Loops, collecting all bytes until end-of-field
async fn get_whole_field(field: &mut Field) -> BytesMut {
    let mut b = BytesMut::new();
    loop {
        match field.next().await {
            Some(Ok(chunk)) => b.extend_from_slice(&chunk),
            None => return b,
            _ => unreachable!(),
        }
    }
}

#[kayrx::test]
async fn test_stream() {
    let (bytes, headers) = create_simple_request_with_header();
    let payload = SlowStream::new(bytes);

    let mut multipart = Multipart::new(&headers, payload);
    match multipart.next().await.unwrap() {
        Ok(mut field) => {
            let cd = field.content_disposition().unwrap();
            assert_eq!(cd.disposition, DispositionType::FormData);
            assert_eq!(cd.parameters[0], DispositionParam::Name("file".into()));

            assert_eq!(field.content_type().type_(), mime::TEXT);
            assert_eq!(field.content_type().subtype(), mime::PLAIN);

            assert_eq!(get_whole_field(&mut field).await, "test");
        }
        _ => unreachable!(),
    }

    match multipart.next().await {
        Some(Ok(mut field)) => {
            assert_eq!(field.content_type().type_(), mime::TEXT);
            assert_eq!(field.content_type().subtype(), mime::PLAIN);

            assert_eq!(get_whole_field(&mut field).await, "data");
        }
        _ => unreachable!(),
    }

    match multipart.next().await {
        None => (),
        _ => unreachable!(),
    }
}

#[kayrx::test]
async fn test_basic() {
    let (_, payload) = Payload::create(false);
    let mut payload = PayloadBuffer::new(payload);

    assert_eq!(payload.buf.len(), 0);
    lazy(|cx| payload.poll_stream(cx)).await.unwrap();
    assert_eq!(None, payload.read_max(1).unwrap());
}

#[kayrx::test]
async fn test_eof() {
    let (mut sender, payload) = Payload::create(false);
    let mut payload = PayloadBuffer::new(payload);

    assert_eq!(None, payload.read_max(4).unwrap());
    sender.feed_data(Bytes::from("data"));
    sender.feed_eof();
    lazy(|cx| payload.poll_stream(cx)).await.unwrap();

    assert_eq!(Some(Bytes::from("data")), payload.read_max(4).unwrap());
    assert_eq!(payload.buf.len(), 0);
    assert!(payload.read_max(1).is_err());
    assert!(payload.eof);
}

#[kayrx::test]
async fn test_err() {
    let (mut sender, payload) = Payload::create(false);
    let mut payload = PayloadBuffer::new(payload);
    assert_eq!(None, payload.read_max(1).unwrap());
    sender.set_error(PayloadError::Incomplete(None));
    lazy(|cx| payload.poll_stream(cx)).await.err().unwrap();
}

#[kayrx::test]
async fn test_readmax() {
    let (mut sender, payload) = Payload::create(false);
    let mut payload = PayloadBuffer::new(payload);

    sender.feed_data(Bytes::from("line1"));
    sender.feed_data(Bytes::from("line2"));
    lazy(|cx| payload.poll_stream(cx)).await.unwrap();
    assert_eq!(payload.buf.len(), 10);

    assert_eq!(Some(Bytes::from("line1")), payload.read_max(5).unwrap());
    assert_eq!(payload.buf.len(), 5);

    assert_eq!(Some(Bytes::from("line2")), payload.read_max(5).unwrap());
    assert_eq!(payload.buf.len(), 0);
}

#[kayrx::test]
async fn test_readexactly() {
    let (mut sender, payload) = Payload::create(false);
    let mut payload = PayloadBuffer::new(payload);

    assert_eq!(None, payload.read_exact(2));

    sender.feed_data(Bytes::from("line1"));
    sender.feed_data(Bytes::from("line2"));
    lazy(|cx| payload.poll_stream(cx)).await.unwrap();

    assert_eq!(Some(Bytes::from_static(b"li")), payload.read_exact(2));
    assert_eq!(payload.buf.len(), 8);

    assert_eq!(Some(Bytes::from_static(b"ne1l")), payload.read_exact(4));
    assert_eq!(payload.buf.len(), 4);
}

#[kayrx::test]
async fn test_readuntil() {
    let (mut sender, payload) = Payload::create(false);
    let mut payload = PayloadBuffer::new(payload);

    assert_eq!(None, payload.read_until(b"ne").unwrap());

    sender.feed_data(Bytes::from("line1"));
    sender.feed_data(Bytes::from("line2"));
    lazy(|cx| payload.poll_stream(cx)).await.unwrap();

    assert_eq!(
        Some(Bytes::from("line")),
        payload.read_until(b"ne").unwrap()
    );
    assert_eq!(payload.buf.len(), 6);

    assert_eq!(
        Some(Bytes::from("1line2")),
        payload.read_until(b"2").unwrap()
    );
    assert_eq!(payload.buf.len(), 0);
}