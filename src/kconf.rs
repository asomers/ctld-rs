//! Read the state of CTL in the running kernel.
use std::{
    ffi::{CStr, OsString},
    mem,
    os::{
        fd::AsRawFd,
        unix::ffi::OsStringExt,
    },
    process,
};

use anyhow::{Context, Result};
use serde_derive::Deserialize;

use crate::conf;
use crate::ffi;
#[mockall_double::double]
use crate::ioc::ioc;

/// Get either the current lun or port list from the kernel
fn get_lunport_list(port: bool) -> Result<String>
{
    let mut bufsiz: usize = 4096;
    let mut buf = Vec::<u8>::with_capacity(bufsiz);

    // Safe because this is how C does it.
    let mut list: ffi::ctl_lun_list = unsafe{ mem::zeroed() };
    loop {
        buf.reserve(bufsiz - buf.capacity());
        list.alloc_len = bufsiz as u32;
        list.status = ffi::ctl_lun_list_status::CTL_LUN_LIST_NONE;
        list.lun_xml = buf.as_mut_ptr() as *mut i8;
        let ctl_fd = crate::ctl();
        if port {
            unsafe{ ioc::ctl_port_list(ctl_fd.as_raw_fd(), &mut list) }.context("CTL_PORT_LIST")?;
        } else {
            unsafe{ ioc::ctl_lun_list(ctl_fd.as_raw_fd(), &mut list) }.context("CTL_LUN_LIST")?;
        }
        match list.status {
            ffi::ctl_lun_list_status::CTL_LUN_LIST_ERROR => {
                let error_str = unsafe{ CStr::from_ptr(list.error_str.as_ptr()) }
                    .to_string_lossy();
                eprintln!("error returned from CTL_LUN_LIST: {}", error_str);
                process::exit(1);
            },
            ffi::ctl_lun_list_status::CTL_LUN_LIST_NEED_MORE_SPACE => {
                bufsiz <<= 1;
            },
            ffi::ctl_lun_list_status::CTL_LUN_LIST_OK => {
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

/// A CTL LUN published by the kernel.  The kernel may publish other fields too, which we ignore.
#[derive(Debug, Deserialize)]
pub struct Lun {
    #[serde(rename = "@id")]
    pub id: u64,
    pub backend_type: conf::Backend,
    pub lun_type: conf::DeviceType,
    /// Device size in blocks
    pub size: u64,
    /// Blocksize in bytes
    pub blocksize: u32,
    pub serial_number: String,
    pub device_id: String,
    pub num_threads: Option<u32>,
    pub file: Option<String>,
    pub ctld_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Ctllunlist {
    #[serde(rename = "$text")]
    pub text: Option<String>,
    #[serde(default)]
    pub lun: Vec<Lun>,
}

impl Ctllunlist {
    pub fn from_kernel() -> Result<Self> {
        let xml = Self::as_xml()?;
        Self::from_xml(&xml)
    }

    fn from_xml(xml: &str) -> Result<Self> {
        let llist: Self = quick_xml::de::from_str(xml).context("parsing XML")?;
        Ok(llist)
    }

    /// Get the kernel's current LUN list as XML
    pub fn as_xml() -> Result<String> {
        get_lunport_list(false)
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
    pub fn from_kernel() -> Result<Self> {
        let xml = Self::as_xml()?;
        let plist: Self = quick_xml::de::from_str(&xml).context("parsing XML")?;
        Ok(plist)
    }

    /// Get the kernel's current port list as XML
    pub fn as_xml() -> Result<String> {
        get_lunport_list(true)
    }
}

#[cfg(test)]
mod t {
    use super::*;

    mod ctl_lun_list {
        use super::*;

        /// Parse a CtlLunlist that contains no LUNs.
        #[test]
        fn blank() {
            let xml = "<ctllunlist></ctllunlist>";
            let llist = Ctllunlist::from_xml(xml).unwrap();
            assert!(llist.text.is_none());
            assert!(llist.lun.is_empty());
        }

        /// Parse a Ctllunlist containing one ramdisk LUN
        #[test]
        fn ramdisk() {
            let xml =
"<ctllunlist>
<lun id=\"42\">
	<backend_type>ramdisk</backend_type>
	<lun_type>0</lun_type>
	<size>64</size>
	<blocksize>2048</blocksize>
	<serial_number>123456</serial_number>
	<device_id>foo</device_id>
</lun>
</ctllunlist>";
            let llist = Ctllunlist::from_xml(xml).unwrap();
            assert!(llist.text.is_none());
            assert_eq!(llist.lun.len(), 1);
            assert_eq!(llist.lun[0].id, 42);
            assert_eq!(llist.lun[0].backend_type, conf::Backend::Ramdisk);
            assert_eq!(llist.lun[0].blocksize, 2048);
            assert_eq!(llist.lun[0].size, 64);
            assert_eq!(llist.lun[0].device_id, "foo");
            assert_eq!(llist.lun[0].serial_number, "123456");
            assert_eq!(llist.lun[0].lun_type, conf::DeviceType::Disk);
        }

        /// Parse a Ctllunlist containing one block LUN
        #[test]
        fn block() {
            let xml =
"<ctllunlist>
<lun id=\"42\">
	<backend_type>block</backend_type>
	<lun_type>0</lun_type>
	<size>64</size>
	<blocksize>2048</blocksize>
	<serial_number>123456</serial_number>
	<device_id>foo</device_id>
	<num_threads>32</num_threads>
</lun>
</ctllunlist>";
            let llist = Ctllunlist::from_xml(xml).unwrap();
            assert_eq!(llist.lun[0].id, 42);
            assert_eq!(llist.lun[0].backend_type, conf::Backend::Block);
            assert_eq!(llist.lun[0].blocksize, 2048);
            assert_eq!(llist.lun[0].size, 64);
            assert_eq!(llist.lun[0].device_id, "foo");
            assert_eq!(llist.lun[0].serial_number, "123456");
            assert_eq!(llist.lun[0].lun_type, conf::DeviceType::Disk);
            assert_eq!(llist.lun[0].num_threads, Some(32));
        }

        /// Parse a Ctllunlist containing various options
        #[test]
        fn options() {
            let xml =
"<ctllunlist>
<lun id=\"0\">
	<backend_type>block</backend_type>
	<lun_type>0</lun_type>
	<size>2097152</size>
	<blocksize>512</blocksize>
	<serial_number>MYSERIAL0000</serial_number>
	<device_id>MYDEVID0000</device_id>
	<num_threads>32</num_threads>
	<file>/tmp/testlun</file>
	<vendor>foo</vendor>
	<product>bar</product>
	<revision>0123</revision>
	<scsiname>baz</scsiname>
	<eui>0xdeadbeef</eui>
	<naa>0x1a7ebabe</naa>
	<uuid>2dec855d-895c-40a1-8e98-8cba77d79777</uuid>
	<ident_info>0x8888</ident_info>
	<text_ident_info>eighteighteighteight</text_ident_info>
	<ha_role>primary</ha_role>
	<insecure_tpc>on</insecure_tpc>
	<readcache>off</readcache>
	<readonly>on</readonly>
	<removable>on</removable>
	<reordering>unrestricted</reordering>
	<serseq>on</serseq>
	<pblocksize>4096</pblocksize>
	<pblockoffset>512</pblockoffset>
	<ublocksize>131072</ublocksize>
	<ublockoffset>0</ublockoffset>
	<rpm>7200</rpm>
	<formfactor>2</formfactor>
	<temperature>75</temperature>
	<reftemperature>70</reftemperature>
	<provisioning_type>thin</provisioning_type>
	<unmap>on</unmap>
	<unmap_max_lba>1048576</unmap_max_lba>
	<write_same_max_lba>1048576</write_same_max_lba>
	<avail-threashold>20</avail-threashold>
	<used-threshold>81</used-threshold>
	<pool-avail-threshold>22</pool-avail-threshold>
	<pool-used-threshold>83</pool-used-threshold>
	<writecache>off</writecache>
</lun>
</ctllunlist>";
            let llist = Ctllunlist::from_xml(xml).unwrap();
            assert_eq!(llist.lun[0].file, Some(String::from("/tmp/testlun")));
        }
    }
}
