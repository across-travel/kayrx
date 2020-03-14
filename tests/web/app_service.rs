use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use kayrx::web::test::{init_service, TestRequest};
use kayrx::web::{self, App};
use kayrx::http::Response as HttpResponse;
use kayrx::service::Service;

struct DropData(Arc<AtomicBool>);

impl Drop for DropData {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

#[kayrx::test]
async fn test_drop_data() {
    let data = Arc::new(AtomicBool::new(false));

    {
        let mut app = init_service(
            App::new()
                .data(DropData(data.clone()))
                .service(web::resource("/test").to(|| HttpResponse::Ok())),
        )
        .await;
        let req = TestRequest::with_uri("/test").to_request();
        let _ = app.call(req).await.unwrap();
    }
    assert!(data.load(Ordering::Relaxed));
}