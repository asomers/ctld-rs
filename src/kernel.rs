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

use serde::{Deserialize};
use serde_derive::{Deserialize};

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

#[derive(Debug, Deserialize)]
pub struct Lun {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub backend_type: String,
    pub lun_type: String,
    pub size: String,
    pub blocksize: String,
    pub serial_number: String,
    pub device_id: String,
    pub num_threads: String,
    pub file: String,
    pub ctld_name: String,
}

#[derive(Debug, Deserialize)]
pub struct Ctllunlist {
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub lun: Vec<Lun>,
}

impl Ctllunlist {
    pub fn from_kernel(ctl_fd: &fs::File) -> io::Result<Self> {
        let xml = Self::as_xml(ctl_fd)?;
        let llist: Self = quick_xml::de::from_str(&xml).unwrap();
        Ok(llist)
    }

    /// Get the kernel's current LUN list as XML
    pub fn as_xml(ctl_fd: &fs::File) -> io::Result<String> {
        get_lunport_list(ctl_fd, false)
    }
}

#[derive(Debug, Deserialize)]
pub struct TargPort {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub frontend_type: String,
    pub port_type: String,
    pub online: String,
    pub port_name: String,
    pub physical_port: String,
    pub virtual_port: String,
    pub lun: Option<TargetLun>,
    pub lun_map: Option<String>,
    pub cfiscsi_portal_group_tag: Option<String>,
    pub ctld_portal_group_name: Option<String>,
    pub cfiscsi_target: Option<String>,
    pub cfiscsi_state: Option<String>,
    pub port: Option<String>,
    pub target: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TargetLun {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "$text")]
    pub text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Ctlportlist {
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub targ_port: Vec<TargPort>,
}

impl Ctlportlist {
    pub fn from_kernel(ctl_fd: &fs::File) -> io::Result<Self> {
        let xml = Self::as_xml(ctl_fd)?;
        let plist: Self = quick_xml::de::from_str(&xml).unwrap();
        Ok(plist)
    }

    /// Get the kernel's current port list as XML
    pub fn as_xml(ctl_fd: &fs::File) -> io::Result<String> {
        get_lunport_list(ctl_fd, true)
    }
}
