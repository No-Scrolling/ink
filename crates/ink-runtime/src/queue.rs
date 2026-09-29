use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::Duration,
};

const MAX_BYTES: usize = 16 * 1024 * 1024;

#[derive(Default)]
pub(crate) struct Budget {
    bytes: AtomicUsize,
    high_water: AtomicUsize,
}

pub(crate) struct Retained<T> {
    value: Option<T>,
    bytes: usize,
    budget: Arc<Budget>,
}

impl Budget {
    pub fn retain<T>(self: &Arc<Self>, value: T, bytes: usize) -> Result<Retained<T>, T> {
        let Ok(previous) =
            self.bytes
                .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                    current
                        .checked_add(bytes)
                        .filter(|total| *total <= MAX_BYTES)
                })
        else {
            return Err(value);
        };
        self.high_water
            .fetch_max(previous + bytes, Ordering::Relaxed);
        Ok(Retained {
            value: Some(value),
            bytes,
            budget: self.clone(),
        })
    }
    pub fn metrics(&self) -> (usize, usize) {
        (
            self.bytes.load(Ordering::Acquire),
            self.high_water.load(Ordering::Relaxed),
        )
    }
}

impl<T> Retained<T> {
    pub fn take(mut self) -> T {
        self.value.take().unwrap()
    }
    pub fn replace(&mut self, value: T, bytes: usize) -> Result<(), T> {
        if bytes > self.bytes {
            let extra = bytes - self.bytes;
            let Ok(previous) =
                self.budget
                    .bytes
                    .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                        current
                            .checked_add(extra)
                            .filter(|total| *total <= MAX_BYTES)
                    })
            else {
                return Err(value);
            };
            self.budget
                .high_water
                .fetch_max(previous + extra, Ordering::Relaxed);
        } else {
            self.budget
                .bytes
                .fetch_sub(self.bytes - bytes, Ordering::AcqRel);
        }
        self.value = Some(value);
        self.bytes = bytes;
        Ok(())
    }
}
impl<T> Drop for Retained<T> {
    fn drop(&mut self) {
        self.budget.bytes.fetch_sub(self.bytes, Ordering::AcqRel);
    }
}

pub(crate) struct Sender<T> {
    inner: mpsc::SyncSender<Retained<T>>,
    budget: Arc<Budget>,
}
pub struct Receiver<T>(mpsc::Receiver<Retained<T>>);

pub(crate) fn channel<T>(count: usize, budget: Arc<Budget>) -> (Sender<T>, Receiver<T>) {
    let (send, receive) = mpsc::sync_channel(count);
    (
        Sender {
            inner: send,
            budget,
        },
        Receiver(receive),
    )
}
impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            budget: self.budget.clone(),
        }
    }
}
impl<T> Sender<T> {
    pub fn try_send(&self, value: T, bytes: usize) -> Result<(), mpsc::TrySendError<T>> {
        let value = self
            .budget
            .retain(value, bytes)
            .map_err(mpsc::TrySendError::Full)?;
        self.inner.try_send(value).map_err(|error| match error {
            mpsc::TrySendError::Full(value) => mpsc::TrySendError::Full(value.take()),
            mpsc::TrySendError::Disconnected(value) => {
                mpsc::TrySendError::Disconnected(value.take())
            }
        })
    }
}
impl<T> Receiver<T> {
    pub fn recv(&self) -> Result<T, mpsc::RecvError> {
        self.0.recv().map(Retained::take)
    }
    pub fn recv_timeout(&self, timeout: Duration) -> Result<T, mpsc::RecvTimeoutError> {
        self.0.recv_timeout(timeout).map(Retained::take)
    }
    pub fn try_recv(&self) -> Result<T, mpsc::TryRecvError> {
        self.0.try_recv().map(Retained::take)
    }
}
