#[cfg(not(test))]
pub mod ioc {
    use nix::ioctl_readwrite;

    use crate::ffi;

    ioctl_readwrite!(ctl_lun_list, 225, 0x22, ffi::ctl_lun_list);
    ioctl_readwrite!(ctl_port_list, 225, 0x27, ffi::ctl_lun_list);
    ioctl_readwrite!(ctl_lun_req, 225, 0x21, ffi::ctl_lun_req);
}
// Mockall doesn't understand Nix's ioctl_readwrite! macro, so we need to write the mocks manually
#[cfg(test)]
mod mockable {
    #[mockall::automock]
    #[allow(unused)]    // Because Mockall can't create the mock methods without the "real" ones.
    pub mod ioc {
        use std::os::fd::RawFd;
        use crate::ffi;

        pub unsafe fn ctl_lun_list(_fd: RawFd, _data: *mut ffi::ctl_lun_list)
            -> nix::Result<i32> { unimplemented!() }
        pub unsafe fn ctl_port_list(_fd: RawFd, _data: *mut ffi::ctl_lun_list)
            -> nix::Result<i32> { unimplemented!() }
        pub unsafe fn ctl_lun_req(_fd: RawFd, _data: *mut ffi::ctl_lun_req)
            -> nix::Result<i32> { unimplemented!() }
    }
}
#[cfg(test)]
pub use mockable::mock_ioc;
