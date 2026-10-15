//! Fetch cancellation that lives until the response body has been consumed.

pub(crate) struct RequestDeadline {
    controller: web_sys::AbortController,
    _timer: gloo_timers::callback::Timeout,
}

impl RequestDeadline {
    pub(crate) fn new(milliseconds: u32) -> Result<Self, String> {
        let controller = web_sys::AbortController::new()
            .map_err(|_| "Could not create request deadline".to_owned())?;
        let abort = controller.clone();
        let timer = gloo_timers::callback::Timeout::new(milliseconds, move || abort.abort());
        Ok(Self {
            controller,
            _timer: timer,
        })
    }

    pub(crate) fn signal(&self) -> web_sys::AbortSignal {
        self.controller.signal()
    }
}

impl Drop for RequestDeadline {
    fn drop(&mut self) {
        self.controller.abort();
    }
}
