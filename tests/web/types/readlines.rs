use futures::stream::StreamExt;

use kayrx::krse::Bytes;
use kayrx::web::types::*;
use kayrx::web::test::TestRequest;

#[kayrx::test]
async fn test_readlines() {
    let mut req = TestRequest::default()
        .set_payload(Bytes::from_static(
            b"Lorem Ipsum is simply dummy text of the printing and typesetting\n\
              industry. Lorem Ipsum has been the industry's standard dummy\n\
              Contrary to popular belief, Lorem Ipsum is not simply random text.",
        ))
        .to_request();

    let mut stream = Readlines::new(&mut req);
    assert_eq!(
        stream.next().await.unwrap().unwrap(),
        "Lorem Ipsum is simply dummy text of the printing and typesetting\n"
    );

    assert_eq!(
        stream.next().await.unwrap().unwrap(),
        "industry. Lorem Ipsum has been the industry's standard dummy\n"
    );

    assert_eq!(
        stream.next().await.unwrap().unwrap(),
        "Contrary to popular belief, Lorem Ipsum is not simply random text."
    );
}