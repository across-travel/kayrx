use kayrx::util::time::*;
use kayrx::timer::*;
use std::time::{Duration, SystemTime};

/// State Under Test: Two calls of `SystemTimeService::now()` return the same value if 
/// they are done within resolution interval of `SystemTimeService`.
///
/// Expected Behavior: Two back-to-back calls of `SystemTimeService::now()` return the same value.
#[kayrx::test]
async fn system_time_service_time_does_not_immediately_change() {
    let resolution = Duration::from_millis(50);

    let time_service = SystemTimeService::with(resolution);
    assert_eq!(time_service.now(), time_service.now());
}

/// State Under Test: Two calls of `LowResTimeService::now()` return the same value if 
/// they are done within resolution interval of `SystemTimeService`.
///
/// Expected Behavior: Two back-to-back calls of `LowResTimeService::now()` return the same value.
#[kayrx::test]
async fn lowres_time_service_time_does_not_immediately_change() {
    let resolution = Duration::from_millis(50);
    let time_service = LowResTimeService::with(resolution);
    assert_eq!(time_service.now(), time_service.now());
}

/// State Under Test: `SystemTimeService::now()` updates returned value every resolution period.
///
/// Expected Behavior: Two calls of `LowResTimeService::now()` made in subsequent resolution interval return different values
/// and second value is greater than the first one at least by a resolution interval.
#[kayrx::test]
async fn system_time_service_time_updates_after_resolution_interval() {
    let resolution = Duration::from_millis(100);
    let wait_time = Duration::from_millis(300);

    let time_service = SystemTimeService::with(resolution);

    let first_time = time_service
        .now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();

    delay_for(wait_time).await;

    let second_time = time_service
        .now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();

    assert!(second_time - first_time >= wait_time);
}

/// State Under Test: `LowResTimeService::now()` updates returned value every resolution period.
///
/// Expected Behavior: Two calls of `LowResTimeService::now()` made in subsequent resolution interval return different values
/// and second value is greater than the first one at least by a resolution interval.
#[kayrx::test]
async fn lowres_time_service_time_updates_after_resolution_interval() {
    let resolution = Duration::from_millis(100);
    let wait_time = Duration::from_millis(300);
    let time_service = LowResTimeService::with(resolution);

    let first_time = time_service.now();

    delay_for(wait_time).await;

    let second_time = time_service.now();
    assert!(second_time - first_time >= wait_time);
}