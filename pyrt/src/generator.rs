//! Generators: the body runs on its own thread, strictly in turn with the
//! consumer (it starts on the first `next` and stops at each `yield` until the
//! next one), so output interleaves exactly as in CPython.

use std::sync::mpsc::{sync_channel, Receiver, SyncSender};

/// The unwinding that ends a generator whose consumer is gone.
struct Closed;

/// The generator side: `y.yield_(v)`.
pub struct Yield<T> {
    req: Receiver<()>,
    val: SyncSender<Option<T>>,
}

impl<T> Yield<T> {
    pub fn yield_(&self, v: T) {
        if self.val.send(Some(v)).is_err() || self.req.recv().is_err() {
            std::panic::resume_unwind(Box::new(Closed));
        }
    }
}

/// A generator object (an iterator).
pub struct Gen<T> {
    req: Option<SyncSender<()>>,
    val: Receiver<Option<T>>,
}

impl<T: Send + 'static> Gen<T> {
    pub fn new(body: impl FnOnce(&Yield<T>) + Send + 'static) -> Gen<T> {
        let (req_tx, req_rx) = sync_channel::<()>(0);
        let (val_tx, val_rx) = sync_channel::<Option<T>>(0);
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(move || {
                if req_rx.recv().is_err() {
                    return;
                }
                let y = Yield { req: req_rx, val: val_tx };
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| body(&y)));
                match r {
                    Ok(()) => {
                        let _ = y.val.send(None);
                    }
                    Err(e) if e.is::<Closed>() => {}
                    Err(e) => std::panic::resume_unwind(e),
                }
            })
            .expect("generator thread");
        Gen { req: Some(req_tx), val: val_rx }
    }
}

impl<T> Iterator for Gen<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        let req = self.req.as_ref()?;
        if req.send(()).is_err() {
            self.req = None;
            return None;
        }
        match self.val.recv() {
            Ok(Some(v)) => Some(v),
            _ => {
                self.req = None;
                None
            }
        }
    }
}

/// `next(g)`
pub fn next<T>(g: &mut Gen<T>) -> T {
    g.next().unwrap_or_else(|| crate::raise("StopIteration", ""))
}
