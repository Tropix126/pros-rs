use core::{cell::UnsafeCell, hash::Hash};

use alloc::boxed::Box;
use hashbrown::HashMap;
use snafu::Snafu;
use spin::Once;

use crate::{error::{bail_on, map_errno}, sync::Mutex};

/// Creates a task to be run 'asynchronously' (More information at the [FreeRTOS docs](https://www.freertos.org/taskandcr.html)).
/// Takes in a closure that can move variables if needed.
/// If your task has a loop it is advised to use [`sleep(duration)`](sleep) so that the task does not take up necessary system resources.
/// Tasks should be long-living; starting many tasks can be slow and is usually not necessary.
pub fn spawn<F>(f: F) -> TaskHandle
where
    F: FnOnce() + Send + 'static,
{
    Builder::new().spawn(f).expect("Failed to spawn task")
}

fn spawn_inner<F: FnOnce() + Send + 'static>(
    function: F,
    priority: TaskPriority,
    stack_depth: TaskStackDepth,
    name: Option<&str>,
) -> Result<TaskHandle, SpawnError> {
    let mut entrypoint = TaskEntrypoint { function };
    let name = alloc::ffi::CString::new(name.unwrap_or("<unnamed>"))
        .unwrap()
        .into_raw();
    unsafe {
        let task = bail_on!(
            core::ptr::null(),
            pros_sys::task_create(
                Some(TaskEntrypoint::<F>::cast_and_call_external),
                &mut entrypoint as *mut _ as *mut core::ffi::c_void,
                priority as _,
                stack_depth as _,
                name,
            )
        );

        _ = alloc::ffi::CString::from_raw(name);

        let handle = TaskHandle { task, next_free_tls_index: Box::leak(Box::new(UnsafeCell::new(0))) };
        
        // This task local is used by the thread_local macro to store the next empty thread local index.
        // This needs to be in task local storage so that the task returns from current has the correct value.
        task_local_storage_set::<UnsafeCell<u32>>(task, handle.next_free_tls_index, 0);
        
        Ok(handle)
    }
}

/// An owned permission to perform actions on a task.
#[derive(Clone)]
pub struct TaskHandle {
    task: pros_sys::task_t,
    next_free_tls_index: &'static UnsafeCell<u32>,
}
unsafe impl Send for TaskHandle {}
impl Hash for TaskHandle {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.task.hash(state)
    }
}

impl PartialEq for TaskHandle {
    fn eq(&self, other: &Self) -> bool {
        self.task == other.task
    }
}
impl Eq for TaskHandle {}

impl TaskHandle {
    /// Pause execution of the task.
    /// This can have unintended consequences if you are not careful,
    /// for example, if this task is holding a mutex when paused, there is no way to retrieve it until the task is unpaused.
    pub fn pause(&self) {
        unsafe {
            pros_sys::task_suspend(self.task);
        }
    }

    /// Resumes execution of the task.
    pub fn unpause(&self) {
        unsafe {
            pros_sys::task_resume(self.task);
        }
    }

    /// Sets the task's priority, allowing you to control how much cpu time is allocated to it.
    pub fn set_priority(&self, priority: impl Into<u32>) {
        unsafe {
            pros_sys::task_set_priority(self.task, priority.into());
        }
    }

    /// Get the state of the task.
    pub fn state(&self) -> TaskState {
        unsafe { pros_sys::task_get_state(self.task).into() }
    }

    /// Send a notification to the task.
    pub fn notify(&self) {
        unsafe {
            pros_sys::task_notify(self.task);
        }
    }

    /// Waits for the task to finish, and then deletes it.
    pub fn join(self) {
        unsafe {
            pros_sys::task_join(self.task);
        }
    }

    /// Aborts the task and consumes it. Memory allocated by the task will not be freed.
    pub fn abort(self) {
        unsafe {
            pros_sys::task_delete(self.task);
        }
    }
}

/// An ergonomic builder for tasks. Alternatively you can use [`spawn`].
#[derive(Default)]
pub struct Builder<'a> {
    name: Option<&'a str>,
    priority: Option<TaskPriority>,
    stack_depth: Option<TaskStackDepth>,
}

impl<'a> Builder<'a> {
    /// Creates a task builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the name of the task, this is useful for debugging.
    pub fn name(mut self, name: &'a str) -> Self {
        self.name = Some(name);
        self
    }

    /// Sets the priority of the task (how much time the scheduler gives to it.).
    pub fn priority(mut self, priority: TaskPriority) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Sets how large the stack for the task is.
    /// This can usually be set to default
    pub fn stack_depth(mut self, stack_depth: TaskStackDepth) -> Self {
        self.stack_depth = Some(stack_depth);
        self
    }

    /// Builds and spawns the task
    pub fn spawn<F>(self, function: F) -> Result<TaskHandle, SpawnError>
    where
        F: FnOnce() + Send + 'static,
    {
        spawn_inner(
            function,
            self.priority.unwrap_or_default(),
            self.stack_depth.unwrap_or_default(),
            self.name,
        )
    }
}

/// Represents the current state of a task.
pub enum TaskState {
    /// The task is currently utilizing the processor
    Running,
    /// The task is currently yielding but may run in the future
    Ready,
    /// The task is blocked. For example, it may be [`sleep`]ing or waiting on a mutex.
    /// Tasks that are in this state will usually return to the task queue after a set timeout.
    Blocked,
    /// The task is suspended. For example, it may be waiting on a mutex or semaphore.
    Suspended,
    /// The task has been deleted using [`TaskHandle::abort`].
    Deleted,
    /// The task's state is invalid somehow
    Invalid,
}

impl From<u32> for TaskState {
    fn from(value: u32) -> Self {
        match value {
            pros_sys::E_TASK_STATE_RUNNING => Self::Running,
            pros_sys::E_TASK_STATE_READY => Self::Ready,
            pros_sys::E_TASK_STATE_BLOCKED => Self::Blocked,
            pros_sys::E_TASK_STATE_SUSPENDED => Self::Suspended,
            pros_sys::E_TASK_STATE_DELETED => Self::Deleted,
            pros_sys::E_TASK_STATE_INVALID => Self::Invalid,
            _ => Self::Invalid,
        }
    }
}

/// Represents how much time the cpu should spend on this task.
/// (Otherwise known as the priority)
#[repr(u32)]
pub enum TaskPriority {
    High = 16,
    Default = 8,
    Low = 1,
}

impl Default for TaskPriority {
    fn default() -> Self {
        Self::Default
    }
}

impl From<TaskPriority> for u32 {
    fn from(val: TaskPriority) -> Self {
        val as u32
    }
}

/// Represents how large of a stack the task should get.
/// Tasks that don't have any or many variables and/or don't need floats can use the low stack depth option.
#[repr(u32)]
pub enum TaskStackDepth {
    Default = 8192,
    Low = 512,
}

impl Default for TaskStackDepth {
    fn default() -> Self {
        Self::Default
    }
}

struct TaskEntrypoint<F> {
    function: F,
}

impl<F> TaskEntrypoint<F>
where
    F: FnOnce(),
{
    unsafe extern "C" fn cast_and_call_external(this: *mut core::ffi::c_void) {
        let this = this.cast::<Self>().read();

        (this.function)()
    }
}

#[derive(Debug, Snafu)]
pub enum SpawnError {
    #[snafu(display("The stack cannot be used as the TCB was not created."))]
    TCBNotCreated,
}

map_errno! {
    SpawnError {
        ENOMEM => SpawnError::TCBNotCreated,
    }
}

/// Sleeps the current task for the given amount of time.
/// This is especially useful in loops to provide a chance for other tasks to run.
pub fn sleep(duration: core::time::Duration) {
    unsafe { pros_sys::delay(duration.as_millis() as u32) }
}

/// Returns the task the function was called from.
pub fn current() -> TaskHandle {
    unsafe {
        let task = pros_sys::task_get_current();
        let next = task_local_storage_get::<UnsafeCell<u32>>(task, 0).unwrap();
        TaskHandle {
            task,
            next_free_tls_index: next,
        }
    }
}

/// Gets the first notification in the queue.
/// If there is none, blocks until a notification is received.
/// I am unsure what happens if the thread is unblocked while waiting.
/// returns the value of the notification
pub fn get_notification() -> u32 {
    unsafe { pros_sys::task_notify_take(false, pros_sys::TIMEOUT_MAX) }
}

// Unsafe because you can change the thread local storage while it is being read.
// This requires you to leak val so that you can be sure it lives the entire task.
unsafe fn task_local_storage_set<T>(task: pros_sys::task_t, val: &'static T, index: u32) {
    // Yes, we transmute val. This is the intended use of this function.
    pros_sys::vTaskSetThreadLocalStoragePointer(task, index as _, (val as *const T).cast());
}

// Unsafe because we can't check if the type is the same as the one that was set.
unsafe fn task_local_storage_get<T>(task: pros_sys::task_t, index: u32) -> Option<&'static T> {
    let val = pros_sys::pvTaskGetThreadLocalStoragePointer(task, index as _);
    val.cast::<T>().as_ref()
}

pub struct LocalKey<T: 'static> {
    index_map: Once<Mutex<HashMap<TaskHandle, u32>>>, 
    init: fn() -> T,
}

impl<T: 'static> LocalKey<T> {
    pub const fn new(init: fn() -> T) -> Self {
        Self {
            index_map: Once::new(),
            init,
        }
    }

    pub fn with<F, R>(&'static self, f: F) -> R where F: FnOnce(&T) -> R {
        self.index_map.call_once(|| Mutex::new(HashMap::new()));

        let current = current();
        if let Some(index) = self.index_map.get().unwrap().lock().get(&current) {
            let val = unsafe { task_local_storage_get::<T>(current.task, *index).unwrap() };
            f(val)
        } else {
            // Get the next empty index in thread_local storage. 
            let next_empty: &u32 = unsafe { task_local_storage_get(current.task, 0).unwrap() };
            let val = Box::leak(Box::new((self.init)()));
            unsafe { task_local_storage_set(current.task, val, *next_empty) }
            self.index_map.get().unwrap().lock().insert(current.clone(), *next_empty);

            unsafe { *current.next_free_tls_index.get() += 1; }

            let val = unsafe { task_local_storage_get::<T>(current.task, *next_empty).unwrap() };
            f(val)
        }
    }
}

#[macro_export]
macro_rules! task_local {
    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = $init:expr; $($rest:tt)*) => {
        $(#[$attr])*
        $vis static $name: LocalKey<$t> = $crate::task::LocalKey::new(|| $init);
        task_local!($($rest)*);
    };
    ($(#[$attr:meta])* $vis:vis static $name:ident: $t:ty = $init:expr) => {
        $(#[$attr])*
        $vis static $name: $crate::task::LocalKey<$t> = $crate::task::LocalKey::new(|| $init);
    };
}
