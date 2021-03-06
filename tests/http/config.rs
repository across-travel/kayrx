use kayrx::krse::BytesMut;
use kayrx::http::{KeepAlive, ServiceConfig};

#[test]
fn test_date_len() {
    const DATE_VALUE_LENGTH: usize = 29;

    assert_eq!(DATE_VALUE_LENGTH, "Sun, 06 Nov 1994 08:49:37 GMT".len());
}

#[kayrx::test]
async fn test_date() {
    const DATE_VALUE_LENGTH: usize = 29;

    let settings = ServiceConfig::new(KeepAlive::Os, 0, 0, false, None);
    let mut buf1 = BytesMut::with_capacity(DATE_VALUE_LENGTH + 10);
    settings.set_date(&mut buf1);
    let mut buf2 = BytesMut::with_capacity(DATE_VALUE_LENGTH + 10);
    settings.set_date(&mut buf2);
    assert_eq!(buf1, buf2);
}
