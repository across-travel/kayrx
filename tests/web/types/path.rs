use kayrx::router::ResourceDef;
use derive_more::Display;
use serde::Deserialize;

use kayrx::web::*;
use kayrx::web::web::{Path,PathConfig};
use kayrx::web::test::TestRequest;
use kayrx::http::{self, error};
use kayrx::http::Response as HttpResponse;

#[derive(Deserialize, Debug, Display)]
#[display(fmt = "MyStruct({}, {})", key, value)]
struct MyStruct {
    key: String,
    value: String,
}

#[derive(Deserialize)]
struct Test2 {
    key: String,
    value: u32,
}

#[kayrx::test]
async fn test_extract_path_single() {
    let resource = ResourceDef::new("/{value}/");

    let mut req = TestRequest::with_uri("/32/").to_srv_request();
    resource.match_path(req.match_info_mut());

    let (req, mut pl) = req.into_parts();
    assert_eq!(*Path::<i8>::from_request(&req, &mut pl).await.unwrap(), 32);
    assert!(Path::<MyStruct>::from_request(&req, &mut pl).await.is_err());
}

#[kayrx::test]
async fn test_tuple_extract() {
    let resource = ResourceDef::new("/{key}/{value}/");

    let mut req = TestRequest::with_uri("/name/user1/?id=test").to_srv_request();
    resource.match_path(req.match_info_mut());

    let (req, mut pl) = req.into_parts();
    let res = <(Path<(String, String)>,)>::from_request(&req, &mut pl)
        .await
        .unwrap();
    assert_eq!((res.0).0, "name");
    assert_eq!((res.0).1, "user1");

    let res = <(Path<(String, String)>, Path<(String, String)>)>::from_request(
        &req, &mut pl,
    )
    .await
    .unwrap();
    assert_eq!((res.0).0, "name");
    assert_eq!((res.0).1, "user1");
    assert_eq!((res.1).0, "name");
    assert_eq!((res.1).1, "user1");

    let () = <()>::from_request(&req, &mut pl).await.unwrap();
}

#[kayrx::test]
async fn test_request_extract() {
    let mut req = TestRequest::with_uri("/name/user1/?id=test").to_srv_request();

    let resource = ResourceDef::new("/{key}/{value}/");
    resource.match_path(req.match_info_mut());

    let (req, mut pl) = req.into_parts();
    let mut s = Path::<MyStruct>::from_request(&req, &mut pl).await.unwrap();
    assert_eq!(s.key, "name");
    assert_eq!(s.value, "user1");
    s.value = "user2".to_string();
    assert_eq!(s.value, "user2");
    assert_eq!(
        format!("{}, {:?}", s, s),
        "MyStruct(name, user2), MyStruct { key: \"name\", value: \"user2\" }"
    );
    let s = s.into_inner();
    assert_eq!(s.value, "user2");

    let s = Path::<(String, String)>::from_request(&req, &mut pl)
        .await
        .unwrap();
    assert_eq!(s.0, "name");
    assert_eq!(s.1, "user1");

    let mut req = TestRequest::with_uri("/name/32/").to_srv_request();
    let resource = ResourceDef::new("/{key}/{value}/");
    resource.match_path(req.match_info_mut());

    let (req, mut pl) = req.into_parts();
    let s = Path::<Test2>::from_request(&req, &mut pl).await.unwrap();
    assert_eq!(s.as_ref().key, "name");
    assert_eq!(s.value, 32);

    let s = Path::<(String, u8)>::from_request(&req, &mut pl)
        .await
        .unwrap();
    assert_eq!(s.0, "name");
    assert_eq!(s.1, 32);

    let res = Path::<Vec<String>>::from_request(&req, &mut pl)
        .await
        .unwrap();
    assert_eq!(res[0], "name".to_owned());
    assert_eq!(res[1], "32".to_owned());
}

#[kayrx::test]
async fn test_custom_err_handler() {
    let (req, mut pl) = TestRequest::with_uri("/name/user1/")
        .app_data(PathConfig::default().error_handler(|err, _| {
            error::InternalError::from_response(
                err,
                HttpResponse::Conflict().finish(),
            )
            .into()
        }))
        .to_http_parts();

    let s = Path::<(usize,)>::from_request(&req, &mut pl)
        .await
        .unwrap_err();
    let res: HttpResponse = s.into();

    assert_eq!(res.status(), http::StatusCode::CONFLICT);
}