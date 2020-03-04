pub(crate) use self::sys::*;

mod sys {
    use mio::unix::UnixReady;
    use mio::Ready;

    pub(crate) fn hup() -> Ready {
        UnixReady::hup().into()
    }

    pub(crate) fn is_hup(ready: Ready) -> bool {
        UnixReady::from(ready).is_hup()
    }
}
