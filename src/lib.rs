//! This is not a real library!  It should be used from within the ctld workspace only.

use std::{
    ffi::{CStr, OsStr,FromBytesUntilNulError },
    fs,
    os::unix::ffi::OsStrExt,
    sync::OnceLock
};

pub mod conf;
pub mod ffi;
pub mod ioc;
pub mod kconf;
pub mod kernel;

/// Store a global handle to /dev/ctl.  It needs to be global so it can be used in destructors
/// without needing to reopen the device.
static CTLDEV: OnceLock<fs::File> = OnceLock::new();

/// Get a handle to /dev/ctl, opening it if it isn't already open
pub fn ctl() -> &'static fs::File {
    CTLDEV.get_or_init(|| {
        let ctl_dev_path = {
            const CSTR: std::result::Result<&CStr, FromBytesUntilNulError> =
                CStr::from_bytes_until_nul(ffi::CTL_DEFAULT_DEV);
            OsStr::from_bytes(CSTR.unwrap().to_bytes())
        };
        fs::File::open(&ctl_dev_path).expect("opening ctl device file")
    })
}
