//! CTL's kernel interface
use std::{
    ffi::{CStr, OsString},
    fs,
    io,
    mem,
    os::{
        fd::AsRawFd,
        unix::ffi::OsStringExt,
    },
    process,
};

mod ioc {
    use nix::ioctl_readwrite;

    ioctl_readwrite!(ctl_lun_list, 225, 0x22, crate::ffi::ctl_lun_list);
    ioctl_readwrite!(ctl_port_list, 225, 0x27, crate::ffi::ctl_lun_list);
}

/// Get either the current lun or port list from the kernel
fn get_lunport_list(ctl_fd: &fs::File, port: bool) -> io::Result<String>
{
    let mut bufsiz: usize = 4096;
    let mut buf = Vec::<u8>::with_capacity(bufsiz);

    // Safe because this is how C does it.
    let mut list: crate::ffi::ctl_lun_list = unsafe{ mem::zeroed() };
    loop {
        buf.reserve(bufsiz - buf.capacity());
        list.alloc_len = bufsiz as u32;
        list.status = crate::ffi::ctl_lun_list_status::CTL_LUN_LIST_NONE;
        list.lun_xml = buf.as_mut_ptr() as *mut i8;
        if port {
            unsafe{ ioc::ctl_port_list(ctl_fd.as_raw_fd(), &mut list) }?;
        } else {
            unsafe{ ioc::ctl_lun_list(ctl_fd.as_raw_fd(), &mut list) }?;
        }
        match list.status {
            crate::ffi::ctl_lun_list_status::CTL_LUN_LIST_ERROR => {
                let error_str = unsafe{ CStr::from_ptr(list.error_str.as_ptr()) }
                    .to_string_lossy();
                eprintln!("error returned from CTL_LUN_LIST: {}", error_str);
                process::exit(1);
            },
            crate::ffi::ctl_lun_list_status::CTL_LUN_LIST_NEED_MORE_SPACE => {
                bufsiz <<= 1;
            },
            crate::ffi::ctl_lun_list_status::CTL_LUN_LIST_OK => {
                break;
            },
            status => panic!("Unexpected status from CTL_LUN_LIST: {:?}", status)
        }
    }
    list.fill_len -= 1; // Trim trailing NUL
    unsafe{ buf.set_len(list.fill_len as usize) };
    Ok(OsString::from_vec(buf).into_string().unwrap())
}

/// Get the kernel's current LUN list as XML
pub fn get_lun_list(ctl_fd: &fs::File) -> io::Result<String> {
    get_lunport_list(ctl_fd, false)
}
/// Get the kernel's current port list as XML
pub fn get_port_list(ctl_fd: &fs::File) -> io::Result<String> {
    get_lunport_list(ctl_fd, true)
}
