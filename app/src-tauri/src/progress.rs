use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};

pub type Reporter<'a> = dyn Fn(&str, Option<f64>) + Send + Sync + 'a;
pub fn quiet(_: &str, _: Option<f64>) {}

#[derive(Clone, Serialize)]
pub struct Progress {
    pub phase: String,
    pub percent: Option<f64>,
}

static BUSY: AtomicBool = AtomicBool::new(false);

pub fn busy() -> bool {
    BUSY.load(Ordering::SeqCst)
}

pub struct Operation {
    mutex: windows_sys::Win32::Foundation::HANDLE,
}
impl Operation {
    pub fn acquire() -> Result<Self, String> {
        BUSY.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| "Please wait for the current operation to finish.".to_string())?;
        use windows_sys::Win32::{
            Foundation::{CloseHandle, WAIT_ABANDONED, WAIT_OBJECT_0},
            System::Threading::{CreateMutexW, WaitForSingleObject},
        };
        let name = crate::game::wide("Local\\DeadlockDubbing.Operation");
        let mutex = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if mutex.is_null()
            || !matches!(
                unsafe { WaitForSingleObject(mutex, 0) },
                WAIT_OBJECT_0 | WAIT_ABANDONED
            )
        {
            if !mutex.is_null() {
                unsafe {
                    CloseHandle(mutex);
                }
            }
            BUSY.store(false, Ordering::SeqCst);
            return Err("Another launcher operation is running. Please wait.".into());
        }
        Ok(Self { mutex })
    }
}
impl Drop for Operation {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::System::Threading::ReleaseMutex(self.mutex);
            windows_sys::Win32::Foundation::CloseHandle(self.mutex);
        }
        BUSY.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn concurrent_actions_are_rejected_and_lock_is_released() {
        let first = Operation::acquire().unwrap();
        assert!(busy());
        assert!(Operation::acquire().is_err());
        drop(first);
        assert!(!busy());
        assert!(Operation::acquire().is_ok());
    }
}
