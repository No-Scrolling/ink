//! Blocking database operations. Rust owns their worker threads and result delivery.
use std::{
    any::Any,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Instant,
};

use ink_runtime::NativeCall;

use crate::Failure;

pub trait DatabaseBackend: Send + Sync {
    fn connect(&self, store: bool) -> Result<Box<dyn DatabaseConnection>, Failure>;

    fn cancellation(&self) -> Result<Cancellation, Failure> {
        Ok(Cancellation::default())
    }
}

/// Used exclusively by one Rust worker. Implementations may call platform database APIs.
pub trait DatabaseConnection: Send {
    fn execute(
        &mut self,
        call: &NativeCall,
        cancellation: &Cancellation,
        deadline: Instant,
    ) -> Result<String, Failure>;
    /// Release an opened handle whose result could not reach its caller.
    fn discard(&mut self, id: &str);
}

#[derive(Clone, Default)]
pub struct Cancellation {
    cancelled: Arc<AtomicBool>,
    context: Option<Arc<dyn Any + Send + Sync>>,
    interrupt: Option<Arc<dyn Fn() + Send + Sync>>,
}

impl Cancellation {
    pub fn with_interrupt<T: Any + Send + Sync>(
        context: T,
        interrupt: impl Fn(&T) + Send + Sync + 'static,
    ) -> Self {
        let context = Arc::new(context);
        let interrupt_context = context.clone();
        Self {
            cancelled: Arc::default(),
            context: Some(context),
            interrupt: Some(Arc::new(move || interrupt(&interrupt_context))),
        }
    }

    pub fn context<T: Any>(&self) -> Option<&T> {
        self.context.as_ref()?.downcast_ref()
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    pub fn cancel(&self) {
        if !self.cancelled.swap(true, Ordering::AcqRel)
            && let Some(interrupt) = &self.interrupt
        {
            interrupt();
        }
    }
}
