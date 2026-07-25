use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone)]
pub(crate) struct UpdateCoordinator {
    inner: Arc<CoordinatorInner>,
}

struct CoordinatorInner {
    changed: Condvar,
    state: Mutex<CoordinatorState>,
}

#[derive(Default)]
struct CoordinatorState {
    active_check: Option<ActiveCheck>,
    installing_version: Option<String>,
    session_start_pending: bool,
}

struct ActiveCheck {
    request_id: String,
    cancellation: CancellationToken,
}

#[derive(Clone)]
pub(crate) struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub(crate) fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

pub(crate) struct UpdateCheckLease {
    cancellation: CancellationToken,
    inner: Arc<CoordinatorInner>,
    request_id: String,
}

impl UpdateCheckLease {
    pub(crate) fn cancellation_token(&self) -> CancellationToken {
        self.cancellation.clone()
    }
}

impl Drop for UpdateCheckLease {
    fn drop(&mut self) {
        if let Ok(mut state) = self.inner.state.lock()
            && state
                .active_check
                .as_ref()
                .is_some_and(|check| check.request_id == self.request_id)
        {
            state.active_check = None;
            self.inner.changed.notify_all();
        }
    }
}

pub(crate) struct SessionStartBarrier {
    inner: Arc<CoordinatorInner>,
}

impl Drop for SessionStartBarrier {
    fn drop(&mut self) {
        if let Ok(mut state) = self.inner.state.lock() {
            state.session_start_pending = false;
            self.inner.changed.notify_all();
        }
    }
}

pub(crate) struct UpdateInstallLease {
    inner: Arc<CoordinatorInner>,
    version: String,
}

impl Drop for UpdateInstallLease {
    fn drop(&mut self) {
        if let Ok(mut state) = self.inner.state.lock()
            && state.installing_version.as_deref() == Some(self.version.as_str())
        {
            state.installing_version = None;
            self.inner.changed.notify_all();
        }
    }
}

impl UpdateCoordinator {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(CoordinatorInner {
                changed: Condvar::new(),
                state: Mutex::new(CoordinatorState::default()),
            }),
        }
    }

    pub(crate) fn check(&self, request_id: &str) -> Result<UpdateCheckLease, String> {
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| "The update coordinator is unavailable.".to_owned())?;
        if state.session_start_pending
            || state.installing_version.is_some()
            || state.active_check.is_some()
        {
            return Err("Update work cannot start during another native activity.".to_owned());
        }
        let cancellation = CancellationToken::new();
        state.active_check = Some(ActiveCheck {
            request_id: request_id.to_owned(),
            cancellation: cancellation.clone(),
        });
        Ok(UpdateCheckLease {
            cancellation,
            inner: Arc::clone(&self.inner),
            request_id: request_id.to_owned(),
        })
    }

    pub(crate) fn install(&self, checked_version: &str) -> Result<UpdateInstallLease, String> {
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| "The update coordinator is unavailable.".to_owned())?;
        if state.session_start_pending
            || state.installing_version.is_some()
            || state.active_check.is_some()
        {
            return Err("Installation cannot overlap a Session or update check.".to_owned());
        }
        state.installing_version = Some(checked_version.to_owned());
        Ok(UpdateInstallLease {
            inner: Arc::clone(&self.inner),
            version: checked_version.to_owned(),
        })
    }

    pub(crate) fn cancel_for_session_start(&self) -> Result<SessionStartBarrier, String> {
        let mut state = self
            .inner
            .state
            .lock()
            .map_err(|_| "The update coordinator is unavailable.".to_owned())?;
        if state.session_start_pending {
            return Err("Another Session start is already waiting.".to_owned());
        }
        if state.installing_version.is_some() {
            return Err("A Formation Lap update is being installed.".to_owned());
        }
        state.session_start_pending = true;
        if let Some(check) = &state.active_check {
            check.cancellation.cancel();
        }
        while state.active_check.is_some() {
            state = self
                .inner
                .changed
                .wait(state)
                .map_err(|_| "The update coordinator is unavailable.".to_owned())?;
        }
        Ok(SessionStartBarrier {
            inner: Arc::clone(&self.inner),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::UpdateCoordinator;
    use std::{
        sync::mpsc,
        thread,
        time::{Duration, Instant},
    };

    #[test]
    fn session_start_cancels_and_joins_the_active_check() {
        let coordinator = UpdateCoordinator::new();
        let check = coordinator
            .check("request-1")
            .expect("the first check should own the coordinator");
        let cancellation = check.cancellation_token();
        let waiting = coordinator.clone();
        let (completed, completion) = mpsc::channel();
        let waiter = thread::spawn(move || {
            let barrier = waiting
                .cancel_for_session_start()
                .expect("Session start should establish a barrier");
            completed
                .send(barrier)
                .expect("test should receive the barrier");
        });

        let deadline = Instant::now() + Duration::from_secs(1);
        while !cancellation.is_cancelled() && Instant::now() < deadline {
            thread::yield_now();
        }
        assert!(cancellation.is_cancelled());
        assert!(
            completion.try_recv().is_err(),
            "Session start must wait until provider work acknowledges completion"
        );

        drop(check);
        let barrier = completion
            .recv_timeout(Duration::from_secs(1))
            .expect("Session barrier should complete after the check joins");
        assert!(
            coordinator.check("request-2").is_err(),
            "new checks must not cross the Session-start barrier"
        );
        drop(barrier);
        waiter.join().expect("waiter should finish");
    }

    #[test]
    fn install_and_session_start_are_mutually_exclusive() {
        let coordinator = UpdateCoordinator::new();
        let install = coordinator
            .install("1.2.3")
            .expect("the checked version should own the install lease");

        assert!(
            coordinator.cancel_for_session_start().is_err(),
            "Session start must not overlap installation"
        );
        drop(install);
        assert!(
            coordinator.cancel_for_session_start().is_ok(),
            "Session start may proceed after installer failure releases the lease"
        );
    }
}
