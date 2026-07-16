use std::sync::Mutex;

type Callback<T> = Box<dyn FnOnce(Result<T, String>, i32) + Send>;

/// Single-use callback for async request/response IPC.
///
/// If dropped without calling 'resolve', the promise is automatically
/// rejected ensuring every pending request eventually settles.
pub struct Responder<T> {
    callback: Mutex<Option<Callback<T>>>,
}

impl<T> Responder<T> {
    pub fn new(callback: Box<dyn FnOnce(Result<T, String>, i32) + Send>) -> Self {
        Self {
            callback: Mutex::new(Some(callback)),
        }
    }

    pub fn resolve(&self, result: Result<T, String>, error_code: i32) {
        let cb = self.callback.lock().unwrap().take();
        if let Some(cb) = cb {
            cb(result, error_code);
        }
    }
}

impl<T> Drop for Responder<T> {
    fn drop(&mut self) {
        if let Some(cb) = self.callback.lock().unwrap().take() {
            cb(Err("handler dropped responder without resolving".into()), -3);
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    type CallRecord = (Result<i32, String>, i32);
    type RecordingResults = Arc<Mutex<Vec<CallRecord>>>;

    /// Creates a responder that records callback invocations for assertions
    fn recording_responder() -> (Responder<i32>, Arc<AtomicUsize>, RecordingResults) {
        let call_count = Arc::new(AtomicUsize::new(0));
        let results: RecordingResults = Arc::new(Mutex::new(Vec::new()));
        let cc = call_count.clone();
        let res = results.clone();
        let responder = Responder::new(Box::new(move |result, error_code| {
            cc.fetch_add(1, Ordering::SeqCst);
            res.lock().unwrap().push((result, error_code));
        }));
        (responder, call_count, results)
    }

    // Resolving a responder invokes its callback exactly once
    #[test]
    fn resolve_once_invokes_callback() {
        let (responder, call_count, results) = recording_responder();
        responder.resolve(Ok(42), 0);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
        let r = results.lock().unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].0, Ok(42));
        assert_eq!(r[0].1, 0);
    }

    // Errors and error codes are forwarded unchanged to the callback
    #[test]
    fn resolve_with_error_passes_error_code() {
        let (responder, call_count, results) = recording_responder();
        responder.resolve(Err("something failed".into()), -42);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
        let r = results.lock().unwrap();
        assert_eq!(r[0].0, Err("something failed".into()));
        assert_eq!(r[0].1, -42);
    }

    // Once resolved, subsequent resolve calls are ignored
    #[test]
    fn resolve_twice_is_noop() {
        let (responder, call_count, results) = recording_responder();
        responder.resolve(Ok(1), 0);
        responder.resolve(Ok(2), 0); // no-op
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
        let r = results.lock().unwrap();
        assert_eq!(r[0].0, Ok(1));
    }

    // The first resolution wins regardless of success or failure
    #[test]
    fn resolve_error_then_ok_is_noop() {
        let (responder, call_count, _) = recording_responder();
        responder.resolve(Err("first".into()), -1);
        responder.resolve(Ok(999), 0);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    // Dropping an unresolved responder automatically rejects the request
    #[test]
    fn drop_without_resolve_auto_rejects() {
        let (responder, call_count, results) = recording_responder();
        drop(responder);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
        let r = results.lock().unwrap();
        assert!(r[0].0.is_err(), "drop should produce an error");
        assert!(
            r[0].0.as_ref().unwrap_err().contains("dropped"),
            "error message should mention 'dropped'"
        );
        assert_eq!(r[0].1, -3, "error code must be -3");
    }

    // Dropping a resolved responder does not invoke the callback again
    #[test]
    fn drop_after_resolve_does_not_call_again() {
        let (responder, call_count, _) = recording_responder();
        responder.resolve(Ok(10), 0);
        drop(responder);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    // Automatic rejection identifies the responder as having been dropped
    #[test]
    fn drop_error_message_contains_dropped_text() {
        let results: Arc<Mutex<Vec<_>>> = Arc::new(Mutex::new(Vec::new()));
        let res = results.clone();
        {
            let _responder: Responder<i32> = Responder::new(Box::new(move |result, code| {
                res.lock().unwrap().push((result, code));
            }));
            // _responder dropped here
        }
        let r = results.lock().unwrap();
        let err_msg = r[0].0.as_ref().unwrap_err();
        assert!(err_msg.contains("handler dropped responder without resolving"));
    }

    // Concurrent resolution invokes the callback at most once
    #[test]
    fn concurrent_resolve_is_safe() {
        use std::thread;

        let (responder, call_count, results) = recording_responder();
        let responder = Arc::new(responder);
        let mut handles = vec![];

        for i in 0..10 {
            let r = responder.clone();
            handles.push(thread::spawn(move || {
                r.resolve(Ok(i), 0);
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(call_count.load(Ordering::SeqCst), 1, "callback must be invoked exactly once");
        let r = results.lock().unwrap();
        assert_eq!(r.len(), 1);
        // Result should be one of the Ok(i) values, not corrupted
        assert!(r[0].0.is_ok());
    }

    // Racing resolve against drop still invokes the callback exactly once
    #[test]
    fn concurrent_resolve_and_drop_is_safe() {
        use std::thread;

        let call_count = Arc::new(AtomicUsize::new(0));
        let cc = call_count.clone();

        let responder = Arc::new(Responder::new(Box::new(move |_, _| {
            cc.fetch_add(1, Ordering::SeqCst);
        })));

        let r1 = responder.clone();
        let h1 = thread::spawn(move || {
            r1.resolve(Ok(1), 0);
        });

        let r2 = responder.clone();
        let h2 = thread::spawn(move || {
            drop(r2);
        });

        h1.join().unwrap();
        h2.join().unwrap();

        assert_eq!(
            call_count.load(Ordering::SeqCst),
            1,
            "callback must be invoked exactly once"
        );
    }
}
