use std::fmt::{self, Display};
use std::io::Write;
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use time::{Date, OffsetDateTime, PrimitiveDateTime};
use bytes::{buf::BufMutExt, BytesMut};
use http::header::{HeaderValue, InvalidHeaderValue};

use crate::http::error::ParseError;
use crate::http::header::IntoHeaderValue;

/// A timestamp with HTTP formatting and parsing
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct HttpDate(OffsetDateTime);

impl FromStr for HttpDate {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<HttpDate, ParseError> {
        match parse_http_date(s) {
            Some(t) => Ok(HttpDate(t.assume_utc())),
            None => Err(ParseError::Header),
        }
    }
}

impl Display for HttpDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0.format("%a, %d %b %Y %H:%M:%S GMT"), f)
    }
}

impl From<OffsetDateTime> for HttpDate {
    fn from(dt: OffsetDateTime) -> HttpDate {
        HttpDate(dt)
    }
}

impl From<SystemTime> for HttpDate {
    fn from(sys: SystemTime) -> HttpDate {
        HttpDate(PrimitiveDateTime::from(sys).assume_utc())
    }
}

impl IntoHeaderValue for HttpDate {
    type Error = InvalidHeaderValue;

    fn try_into(self) -> Result<HeaderValue, Self::Error> {
        let mut wrt = BytesMut::with_capacity(29).writer();
        write!(
            wrt,
            "{}",
            self.0
                .to_offset(time::offset!(UTC))
                .format("%a, %d %b %Y %H:%M:%S GMT")
        )
        .unwrap();
        HeaderValue::from_maybe_shared(wrt.get_mut().split().freeze())
    }
}

impl From<HttpDate> for SystemTime {
    fn from(date: HttpDate) -> SystemTime {
        let dt = date.0;
        let epoch = OffsetDateTime::unix_epoch();

        UNIX_EPOCH + (dt - epoch)
    }
}


/// Attempt to parse a `time` string as one of either RFC 1123, RFC 850, or asctime.
pub fn parse_http_date(time: &str) -> Option<PrimitiveDateTime> {
    try_parse_rfc_1123(time)
        .or_else(|| try_parse_rfc_850(time))
        .or_else(|| try_parse_asctime(time))
}

/// Attempt to parse a `time` string as a RFC 1123 formatted date time string.
fn try_parse_rfc_1123(time: &str) -> Option<PrimitiveDateTime> {
    time::parse(time, "%a, %d %b %Y %H:%M:%S").ok()
}

/// Attempt to parse a `time` string as a RFC 850 formatted date time string.
fn try_parse_rfc_850(time: &str) -> Option<PrimitiveDateTime> {
    match PrimitiveDateTime::parse(time, "%A, %d-%b-%y %H:%M:%S") {
        Ok(dt) => {
            // If the `time` string contains a two-digit year, then as per RFC 2616 § 19.3,
            // we consider the year as part of this century if it's within the next 50 years,
            // otherwise we consider as part of the previous century.
            let now = OffsetDateTime::now();
            let century_start_year = (now.year() / 100) * 100;
            let mut expanded_year = century_start_year + dt.year();

            if expanded_year > now.year() + 50 {
                expanded_year -= 100;
            }

            match Date::try_from_ymd(expanded_year, dt.month(), dt.day()) {
                Ok(date) => Some(PrimitiveDateTime::new(date, dt.time())),
                Err(_) => None,
            }
        }
        Err(_) => None,
    }
}

/// Attempt to parse a `time` string using ANSI C's `asctime` format.
fn try_parse_asctime(time: &str) -> Option<PrimitiveDateTime> {
    time::parse(time, "%a %b %_d %H:%M:%S %Y").ok()
}

#[cfg(test)]
mod tests {
    use super::HttpDate;
    use time::{date, time, PrimitiveDateTime};

    #[test]
    fn test_date() {

        let nov_07 = HttpDate(
            PrimitiveDateTime::new(date!(1994 - 11 - 07), time!(8:48:37)).assume_utc(),
        );

        assert_eq!(
            "Sun, 07 Nov 1994 08:48:37 GMT".parse::<HttpDate>().unwrap(),
            NOV_07
        );
        assert_eq!(
            "Sunday, 07-Nov-94 08:48:37 GMT"
                .parse::<HttpDate>()
                .unwrap(),
            NOV_07
        );
        assert_eq!(
            "Sun Nov  7 08:48:37 1994".parse::<HttpDate>().unwrap(),
            NOV_07
        );
        assert!("this-is-no-date".parse::<HttpDate>().is_err());
    }
}