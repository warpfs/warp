use std::ffi::c_void;
use std::io::Error;
use windows_sys::Win32::Foundation::FALSE;
use windows_sys::Win32::Networking::WinHttp::WinHttpCloseHandle;

/// Encapsulate a WinHTTP handle.
pub struct Handle(*mut c_void);

impl Handle {
    /// # Safety
    /// `raw` must be a valid WinHTTP handle.
    pub(crate) unsafe fn new(raw: *mut c_void) -> Self {
        Self(raw)
    }

    pub fn get(&self) -> *mut c_void {
        self.0
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        if unsafe { WinHttpCloseHandle(self.0) } == FALSE {
            panic!(
                "WinHttpCloseHandle() was failed: {}",
                Error::last_os_error()
            );
        }
    }
}
