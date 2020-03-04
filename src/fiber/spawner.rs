    use crate::fiber::scheduler;
    use crate::fiber::JoinHandle;

    use std::future::Future;


#[derive(Debug, Clone)]
pub(crate) enum Spawner {
    Basic(scheduler::Spawner),
}

impl Spawner {
    /// Enter the scheduler context
    pub(crate) fn enter<F, R>(&self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        match self {
            Spawner::Basic(spawner) => spawner.enter(f),
        }
    }
}

    impl Spawner {
        pub(crate) fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
        where
            F: Future + Send + 'static,
            F::Output: Send + 'static,
        {
            match self {
                Spawner::Basic(spawner) => spawner.spawn(future),
            }
        }
    }
