//! CTL's kernel interface
use std::{
    ffi::{CStr, OsStr, OsString},
    fs,
    io,
    mem,
    os::{
        fd::AsRawFd,
        unix::ffi::{OsStrExt, OsStringExt},
    },
    process,
};

use anyhow::{Context, Result};
use libnv::libnv::{NvList, NvFlag};
use serde::{Deserialize};
use serde_derive::{Deserialize};

mod ioc {
    use nix::ioctl_readwrite;

    ioctl_readwrite!(ctl_lun_list, 225, 0x22, crate::ffi::ctl_lun_list);
    ioctl_readwrite!(ctl_port_list, 225, 0x27, crate::ffi::ctl_lun_list);
    ioctl_readwrite!(ctl_lun_req, 225, 0x21, crate::ffi::ctl_lun_req);
}

/// Get either the current lun or port list from the kernel
fn get_lunport_list(ctl_fd: &fs::File, port: bool) -> Result<String>
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
            unsafe{ ioc::ctl_port_list(ctl_fd.as_raw_fd(), &mut list) }.context("CTL_PORT_LIST")?;
        } else {
            unsafe{ ioc::ctl_lun_list(ctl_fd.as_raw_fd(), &mut list) }.context("CTL_LUN_LIST")?;
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
    OsString::from_vec(buf)
        .into_string()
        .map_err(|_| anyhow::Error::msg("not a valid UTF-8 string"))
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
    #[serde(default)]
    pub lun: Vec<Lun>,
}

impl Ctllunlist {
    pub fn from_kernel(ctl_fd: &fs::File) -> Result<Self> {
        let xml = Self::as_xml(ctl_fd)?;
        let llist: Self = quick_xml::de::from_str(&xml).context("parsing XML")?;
        Ok(llist)
    }

    /// Get the kernel's current LUN list as XML
    pub fn as_xml(ctl_fd: &fs::File) -> Result<String> {
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
    pub cfiscsi_portal_group_tag: Option<u16>,
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
    pub fn from_kernel(ctl_fd: &fs::File) -> Result<Self> {
        let xml = Self::as_xml(ctl_fd)?;
        let plist: Self = quick_xml::de::from_str(&xml).context("parsing XML")?;
        Ok(plist)
    }

    /// Get the kernel's current port list as XML
    pub fn as_xml(ctl_fd: &fs::File) -> Result<String> {
        get_lunport_list(ctl_fd, true)
    }
}

pub fn add_lun(ctl_fd: &fs::File, name: &str, lun: &crate::conf::Lun) -> Result<()> {
    let mut req: crate::ffi::ctl_lun_req = unsafe{ mem::zeroed() };
    let backend = OsStr::new(Into::<&str>::into(lun.backend)).as_bytes();
    let p = backend.as_ptr() as *const i8;
    unsafe{req.backend.as_mut_ptr().copy_from_nonoverlapping(p, backend.len())};
    req.reqtype = crate::ffi::ctl_lunreq_type::CTL_LUNREQ_CREATE;
    req.reqdata.create.blocksize_bytes = lun.blocksize.unwrap_or(0);
    if let Some(size) = lun.size {
        req.reqdata.create.lun_size_bytes = size;
    }
    if let Some(ctl_lun) = lun.ctl_lun {
        req.reqdata.create.req_lun_id = ctl_lun;
        // Safe because we know that we're creating, and the union is already zero-initialized
        unsafe{ req.reqdata.create.flags |= crate::ffi::ctl_backend_lun_flags::CTL_LUN_FLAG_ID_REQ};
    }
    // Safe because we know that we're creating, and the union is already zero-initialized
    unsafe{ req.reqdata.create.flags |= crate::ffi::ctl_backend_lun_flags::CTL_LUN_FLAG_DEV_TYPE };
    req.reqdata.create.device_type = lun.device_type as u8;

    if let Some(s) = &lun.serial {
        // Safe because we know that we're creating, and the union is already zero-initialized
        unsafe {
            req.reqdata.create.serial_num.copy_from_slice(OsStr::new(s).as_bytes());
            req.reqdata.create.flags |= crate::ffi::ctl_backend_lun_flags::CTL_LUN_FLAG_SERIAL_NUM;
        }
    }

    // Safe because we know that we're creating, and the union is already zero-initialized
    let os_device_id = OsStr::new(&lun.device_id);
    unsafe {
        let l = os_device_id.len();
        req.reqdata.create.device_id[0..l].copy_from_slice(os_device_id.as_bytes());
        req.reqdata.create.flags |= crate::ffi::ctl_backend_lun_flags::CTL_LUN_FLAG_DEVID;
    }

    let mut nvl = libnv::libnv::NvList::new(NvFlag::None).context("NvList::new")?;

    nvl.insert_string("file", lun.path.to_str().context("file is not a valid Str")?).context("nvlist_add_string(file)")?;
    nvl.insert_string("ctld_name", name).context("nvlist_add_string(ctld_name)")?;
    // TODO: handle scsiname, for target_lun only
    for (k, v) in lun.options.iter() {
        if ["file", "ctld_name"].contains(&k.as_str()) {
            // These options are overwritten by regular fields
            continue;
        }
        nvl.insert_string(k.as_str(), v.as_str()).context("nvlist_add_string")?;
    }
            
    let mut packed_nvl = nvl.pack().context("nvlist_pack")?;
    req.args = packed_nvl.as_mut_ptr();
    req.args_len = packed_nvl.len();
    unsafe{ ioc::ctl_lun_req(ctl_fd.as_raw_fd(), &mut req) }.context("CTL_LUNREQ_CREATE")?;

    // TODO: log on error in req.status
    assert_eq!(req.status, crate::ffi::ctl_lun_status::CTL_LUN_OK);
    Ok(())
}
