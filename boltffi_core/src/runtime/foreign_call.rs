use std::cell::UnsafeCell;
use std::ffi::c_void;
use std::future::Future;
use std::mem::MaybeUninit;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::task::{Context, Poll, Waker};

use crate::{AsyncCallback, AsyncCallbackVoid, FfiStatus};

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum CompletionState {
    Pending = 0,
    Ready = 1,
    Taken = 2,
}

impl CompletionState {
    const fn as_repr(self) -> u8 {
        self as u8
    }
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum WakerState {
    Waiting = 0,
    Registering = 1,
    Waking = 2,
    RegisteringWaking = 3,
}

impl WakerState {
    const fn as_repr(self) -> u8 {
        self as u8
    }
}

struct Completion<Payload> {
    status: FfiStatus,
    payload: Payload,
}

struct CallState<Payload> {
    completion_state: AtomicU8,
    completion: UnsafeCell<MaybeUninit<Completion<Payload>>>,
    waker_state: AtomicU8,
    waker: UnsafeCell<Option<Waker>>,
}

unsafe impl<Payload: Send> Send for CallState<Payload> {}
unsafe impl<Payload: Send> Sync for CallState<Payload> {}

impl<Payload> CallState<Payload> {
    fn new() -> Self {
        Self {
            completion_state: AtomicU8::new(CompletionState::Pending.as_repr()),
            completion: UnsafeCell::new(MaybeUninit::uninit()),
            waker_state: AtomicU8::new(WakerState::Waiting.as_repr()),
            waker: UnsafeCell::new(None),
        }
    }

    fn complete(&self, completion: Completion<Payload>) {
        unsafe {
            (*self.completion.get()).write(completion);
        }
        self.completion_state
            .store(CompletionState::Ready.as_repr(), Ordering::Release);
        self.wake();
    }

    fn poll(&self, task: &mut Context<'_>) -> Poll<(FfiStatus, Payload)> {
        loop {
            match self.completion_state.load(Ordering::Acquire) {
                state if state == CompletionState::Ready.as_repr() => {
                    if self
                        .completion_state
                        .compare_exchange(
                            CompletionState::Ready.as_repr(),
                            CompletionState::Taken.as_repr(),
                            Ordering::AcqRel,
                            Ordering::Acquire,
                        )
                        .is_ok()
                    {
                        let completion = unsafe { (*self.completion.get()).assume_init_read() };
                        return Poll::Ready((completion.status, completion.payload));
                    }
                }
                state if state == CompletionState::Taken.as_repr() => {
                    panic!("foreign call polled after completion")
                }
                _ => {
                    self.register(task.waker());
                    if self.completion_state.load(Ordering::Acquire)
                        == CompletionState::Pending.as_repr()
                    {
                        return Poll::Pending;
                    }
                }
            }
        }
    }

    fn register(&self, waker: &Waker) {
        match self.waker_state.compare_exchange(
            WakerState::Waiting.as_repr(),
            WakerState::Registering.as_repr(),
            Ordering::Acquire,
            Ordering::Acquire,
        ) {
            Ok(_) => {
                unsafe {
                    let slot = &mut *self.waker.get();
                    if !slot
                        .as_ref()
                        .is_some_and(|registered| registered.will_wake(waker))
                    {
                        *slot = Some(waker.clone());
                    }
                }

                if self
                    .waker_state
                    .compare_exchange(
                        WakerState::Registering.as_repr(),
                        WakerState::Waiting.as_repr(),
                        Ordering::AcqRel,
                        Ordering::Acquire,
                    )
                    .is_err()
                {
                    let registered = unsafe { (&mut *self.waker.get()).take() };
                    self.waker_state
                        .store(WakerState::Waiting.as_repr(), Ordering::Release);
                    registered.into_iter().for_each(Waker::wake);
                }
            }
            Err(state) if state == WakerState::Waking.as_repr() => waker.wake_by_ref(),
            Err(_) => {}
        }
    }

    fn wake(&self) {
        match self
            .waker_state
            .fetch_or(WakerState::Waking.as_repr(), Ordering::AcqRel)
        {
            state if state == WakerState::Waiting.as_repr() => {
                let registered = unsafe { (&mut *self.waker.get()).take() };
                self.waker_state
                    .store(WakerState::Waiting.as_repr(), Ordering::Release);
                registered.into_iter().for_each(Waker::wake);
            }
            state
                if state == WakerState::Registering.as_repr()
                    || state == WakerState::Waking.as_repr()
                    || state == WakerState::RegisteringWaking.as_repr() => {}
            _ => unreachable!("foreign call waker state is invalid"),
        }
    }
}

impl<Payload> Drop for CallState<Payload> {
    fn drop(&mut self) {
        if self.completion_state.load(Ordering::Acquire) == CompletionState::Ready.as_repr() {
            unsafe {
                self.completion.get_mut().assume_init_drop();
            }
        }
    }
}

pub struct ForeignCall<Payload> {
    state: Arc<CallState<Payload>>,
}

impl<Payload: Send + 'static> ForeignCall<Payload> {
    pub unsafe fn start(
        begin: impl FnOnce(AsyncCallback<Payload>, *mut c_void),
    ) -> ForeignCall<Payload> {
        let state = Arc::new(CallState::new());
        let completion_data = Arc::into_raw(Arc::clone(&state)) as *mut c_void;
        begin(Self::complete, completion_data);
        Self { state }
    }

    extern "C" fn complete(completion_data: *mut c_void, status: FfiStatus, payload: Payload) {
        let state = unsafe { Arc::from_raw(completion_data.cast::<CallState<Payload>>()) };
        state.complete(Completion { status, payload });
    }
}

impl ForeignCall<()> {
    pub unsafe fn start_void(
        begin: impl FnOnce(AsyncCallbackVoid, *mut c_void),
    ) -> ForeignCall<()> {
        let state = Arc::new(CallState::new());
        let completion_data = Arc::into_raw(Arc::clone(&state)) as *mut c_void;
        begin(Self::complete_void, completion_data);
        Self { state }
    }

    extern "C" fn complete_void(completion_data: *mut c_void, status: FfiStatus) {
        let state = unsafe { Arc::from_raw(completion_data.cast::<CallState<()>>()) };
        state.complete(Completion {
            status,
            payload: (),
        });
    }
}

impl<Payload: Send> Future for ForeignCall<Payload> {
    type Output = (FfiStatus, Payload);

    fn poll(self: Pin<&mut Self>, task: &mut Context<'_>) -> Poll<Self::Output> {
        self.state.poll(task)
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::c_void;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::task::{Context, Poll, Wake, Waker};
    use std::thread;

    use super::ForeignCall;
    use crate::{AsyncCallback, FfiBuf, FfiStatus};

    #[repr(transparent)]
    struct CompletionData(*mut c_void);

    impl CompletionData {
        fn from_ffi(pointer: *mut c_void) -> Self {
            Self(pointer)
        }

        fn into_ffi(self) -> *mut c_void {
            self.0
        }
    }

    unsafe impl Send for CompletionData {}

    struct WakeCounter(AtomicUsize);

    impl Wake for WakeCounter {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[repr(C)]
    struct DropProbe {
        count: Arc<AtomicUsize>,
    }

    impl Drop for DropProbe {
        fn drop(&mut self) {
            self.count.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn poll<Payload: Send>(
        call: &mut ForeignCall<Payload>,
        waker: &Waker,
    ) -> Poll<(FfiStatus, Payload)> {
        let mut task = Context::from_waker(waker);
        Pin::new(call).poll(&mut task)
    }

    #[test]
    fn resolves_synchronous_completion() {
        let mut call = unsafe {
            ForeignCall::start(|complete, completion_data| {
                complete(completion_data, FfiStatus::OK, 17_u32);
            })
        };
        let waker = Waker::noop();

        assert_eq!(poll(&mut call, waker), Poll::Ready((FfiStatus::OK, 17)));
    }

    #[test]
    fn wakes_after_cross_thread_completion() {
        let (sender, receiver) = mpsc::channel();
        let mut call = unsafe {
            ForeignCall::start(|complete, completion_data| {
                sender
                    .send((complete, CompletionData::from_ffi(completion_data)))
                    .expect("completion registration sends");
            })
        };
        let wake_counter = Arc::new(WakeCounter(AtomicUsize::new(0)));
        let waker = Waker::from(Arc::clone(&wake_counter));

        assert_eq!(poll(&mut call, &waker), Poll::Pending);

        let (complete, completion_data) =
            receiver.recv().expect("completion registration receives");
        thread::spawn(move || {
            complete(
                completion_data.into_ffi(),
                FfiStatus::INTERNAL_ERROR,
                29_u32,
            );
        })
        .join()
        .expect("completion thread joins");

        assert_eq!(wake_counter.0.load(Ordering::Relaxed), 1);
        assert_eq!(
            poll(&mut call, &waker),
            Poll::Ready((FfiStatus::INTERNAL_ERROR, 29))
        );
    }

    #[test]
    fn owns_buffer_without_copying() {
        let bytes = vec![1_u8, 2, 3, 4];
        let pointer = bytes.as_ptr();
        let buffer = FfiBuf::from_vec(bytes);
        let mut call = unsafe {
            ForeignCall::start(|complete, completion_data| {
                complete(completion_data, FfiStatus::OK, buffer);
            })
        };
        let waker = Waker::noop();
        let Poll::Ready((status, buffer)) = poll(&mut call, waker) else {
            panic!("synchronous completion is ready")
        };

        assert_eq!(status, FfiStatus::OK);
        assert_eq!(buffer.as_ptr(), pointer);
        assert_eq!(unsafe { buffer.as_byte_slice() }, &[1, 2, 3, 4]);
    }

    #[test]
    fn drops_late_payload_after_future_drop() {
        let count = Arc::new(AtomicUsize::new(0));
        let (sender, receiver) = mpsc::channel();
        let call = unsafe {
            ForeignCall::start(|complete: AsyncCallback<DropProbe>, completion_data| {
                sender
                    .send((complete, CompletionData::from_ffi(completion_data)))
                    .expect("completion registration sends");
            })
        };
        drop(call);

        let (complete, completion_data) =
            receiver.recv().expect("completion registration receives");
        complete(
            completion_data.into_ffi(),
            FfiStatus::OK,
            DropProbe {
                count: Arc::clone(&count),
            },
        );

        assert_eq!(count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn drops_ready_payload_when_future_is_not_polled() {
        let count = Arc::new(AtomicUsize::new(0));
        let call = unsafe {
            ForeignCall::start(|complete, completion_data| {
                complete(
                    completion_data,
                    FfiStatus::OK,
                    DropProbe {
                        count: Arc::clone(&count),
                    },
                );
            })
        };

        drop(call);

        assert_eq!(count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn resolves_void_completion() {
        let mut call = unsafe {
            ForeignCall::start_void(|complete, completion_data| {
                complete(completion_data, FfiStatus::CANCELLED);
            })
        };
        let waker = Waker::noop();

        assert_eq!(
            poll(&mut call, waker),
            Poll::Ready((FfiStatus::CANCELLED, ()))
        );
    }

    #[test]
    fn foreign_call_with_owned_buffer_is_send() {
        fn assert_send<SendValue: Send>() {}

        assert_send::<ForeignCall<FfiBuf>>();
    }
}
