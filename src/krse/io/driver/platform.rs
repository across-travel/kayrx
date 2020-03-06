pub(crate) use self::sys::*;

mod sys {
    use crate::krse::io::driver::linux::unix::UnixReady;
    use crate::krse::io::driver::linux::Ready;

    pub(crate) fn hup() -> Ready {
        UnixReady::hup().into()
    }

    pub(crate) fn is_hup(ready: Ready) -> bool {
        UnixReady::from(ready).is_hup()
    }
}
