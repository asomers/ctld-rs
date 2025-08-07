//! Create, destroy, and manipulate CTL kernel objects

use std::{
    ffi::OsStr,
    fs,
    mem,
    os::{
        fd::AsRawFd,
        unix::ffi::OsStrExt,
    },
};

use anyhow::{Context, Result};
use libnv::libnv::NvFlag;

use crate::conf;
use crate::ffi;
#[mockall_double::double]
use crate::ioc::ioc;

/// Owns a LUN as its exists within the kernel.  Will destroy on Drop
#[derive(Debug)]
pub struct Lun {
    backend: conf::Backend,
    id: u32
}

impl Lun {
    /// Low-level, non-RAII LUN creation
    fn lunreq_create(ctl_fd: &fs::File, name: &str, lun: &crate::conf::Lun)
        -> Result<ffi::ctl_lun_req>
    {
        let mut req: ffi::ctl_lun_req = unsafe{ mem::zeroed() };
        let backend = OsStr::new(Into::<&str>::into(lun.backend)).as_bytes();
        let p = backend.as_ptr() as *const i8;
        unsafe{req.backend.as_mut_ptr().copy_from_nonoverlapping(p, backend.len())};
        req.reqtype = ffi::ctl_lunreq_type::CTL_LUNREQ_CREATE;
        {
            let create = unsafe{&mut req.reqdata.create};
            create.blocksize_bytes = lun.blocksize.unwrap_or(0);
            if let Some(size) = lun.size {
                create.lun_size_bytes = size;
            }
            if let Some(ctl_lun) = lun.ctl_lun {
                create.req_lun_id = ctl_lun;
                // Safe because we're creating, and the union is already zero-initialized
                create.flags |= ffi::ctl_backend_lun_flags::CTL_LUN_FLAG_ID_REQ;
            }
            // Safe because we know that we're creating, and the union is already zero-initialized
            create.flags |= ffi::ctl_backend_lun_flags::CTL_LUN_FLAG_DEV_TYPE;
            create.device_type = lun.device_type as u8;

            if let Some(s) = &lun.serial {
                // Safe because we're creating, and the union is already zero-initialized
                create.serial_num.copy_from_slice(OsStr::new(s).as_bytes());
                create.flags |= ffi::ctl_backend_lun_flags::CTL_LUN_FLAG_SERIAL_NUM;
            }

            // Safe because we know that we're creating, and the union is already zero-initialized
            let os_device_id = OsStr::new(&lun.device_id);
            let l = os_device_id.len();
            create.device_id[0..l].copy_from_slice(os_device_id.as_bytes());
            create.flags |= ffi::ctl_backend_lun_flags::CTL_LUN_FLAG_DEVID;
        }

        let mut nvl = libnv::libnv::NvList::new(NvFlag::None).context("NvList::new")?;

        nvl.insert_string("file", lun.path.to_str().context("file is not a valid Str")?)
            .context("nvlist_add_string(file)")?;
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
        assert_eq!(req.status, ffi::ctl_lun_status::CTL_LUN_OK);

        Ok(req)
    }

    /// Low-level, non-RAII removal function
    fn lunreq_rm(ctl_fd: &fs::File, backend: conf::Backend, id: u32) -> Result<()> {
        let mut req: ffi::ctl_lun_req = unsafe{ mem::zeroed() };
        let backend = OsStr::new(Into::<&str>::into(backend)).as_bytes();
        let p = backend.as_ptr() as *const i8;
        unsafe{req.backend.as_mut_ptr().copy_from_nonoverlapping(p, backend.len())};
        req.reqtype = ffi::ctl_lunreq_type::CTL_LUNREQ_RM;
        req.reqdata.rm.lun_id = id;

        unsafe{ ioc::ctl_lun_req(ctl_fd.as_raw_fd(), &mut req)}.context("CTL_LUNREQ_RM")?;

        // TODO: log on error in req.status
        Ok(())
    }

    pub fn create(name: &str, lun: &crate::conf::Lun) -> Result<Self> {
        let ctl_fd = crate::ctl();
        let req = Self::lunreq_create(ctl_fd, name, lun)?;
        let id = unsafe { req.reqdata.create.req_lun_id };
        Ok(Lun {
            backend: lun.backend,
            id
        })
    }
}

impl Drop for Lun {
    fn drop(&mut self) {
        let r = Self::lunreq_rm(crate::ctl(), self.backend, self.id);
        if !std::thread::panicking() {
            r.expect("Lun::drop");
        }
    }
}

#[cfg(test)]
mod t {
    use super::*;

    use std::sync::Mutex;

    /// Serialize ioc::ctl_lun_req calls and expectations
    static CTL_LUN_REQ_MTX: Mutex<()> = Mutex::new(());

    mod lunreq_create {
        use super::*;

        /// Add the simplest possible LUN. Test that we pass a correctly formatted request to the
        /// kernel.
        #[test]
        fn basic() {
            let _m = CTL_LUN_REQ_MTX.lock().unwrap();

            let lun = crate::conf::Lun {
                backend: crate::conf::Backend::Ramdisk,
                blocksize: Some(2048),
                ctl_lun: Some(0),
                device_id: String::from("ramdisk0"),
                device_type: crate::conf::DeviceType::Disk,
                options: Default::default(),
                path: Default::default(),
                serial: None,
                size: Some(131072)
            };
            let dev_ctl = fs::File::open("/dev/null").unwrap();

            let ctx = ioc::ctl_lun_req_context();
            ctx.expect()
                .withf(|_fd, req| unsafe {
                    let flags = ffi::ctl_backend_lun_flags::CTL_LUN_FLAG_ID_REQ |
                        ffi::ctl_backend_lun_flags::CTL_LUN_FLAG_DEV_TYPE |
                        ffi::ctl_backend_lun_flags::CTL_LUN_FLAG_DEVID;
                    let ubackend = &*(&(**req).backend as *const [i8] as *const [u8]);
                    ubackend[0..8] == b"ramdisk\0"[0..8] &&
                    (**req).reqtype == ffi::ctl_lunreq_type::CTL_LUNREQ_CREATE &&
                    (**req).reqdata.create.flags == flags &&
                    (**req).reqdata.create.device_type == 0 &&
                    (**req).reqdata.create.lun_size_bytes == 131072 &&
                    (**req).reqdata.create.blocksize_bytes == 2048 &&
                    (&(**req).reqdata.create.device_id)[0..9] == b"ramdisk0\0"[0..9]
                })
                .returning(|_fd, req| {
                    unsafe{(*req).status = ffi::ctl_lun_status::CTL_LUN_OK};
                    Ok(0)
                });

            Lun::lunreq_create(&dev_ctl, "foo", &lun).unwrap();
        }
    }

    mod lunreq_rm {
        use super::*;

        /// Remove a LUN. Test that we pass a correctly formatted request to the kernel.
        #[test]
        fn ok() {
            let _m = CTL_LUN_REQ_MTX.lock().unwrap();
            let dev_ctl = fs::File::open("/dev/null").unwrap();

            let ctx = ioc::ctl_lun_req_context();
            ctx.expect()
                .withf(|_fd, req| unsafe {
                    let ubackend = &*(&(**req).backend as *const [i8] as *const [u8]);
                    ubackend[0..8] == b"ramdisk\0"[0..8] &&
                    (**req).reqtype == ffi::ctl_lunreq_type::CTL_LUNREQ_RM &&
                    (**req).reqdata.rm.lun_id == 42
                })
                .returning(|_fd, req| {
                    unsafe{(*req).status = ffi::ctl_lun_status::CTL_LUN_OK};
                    Ok(0)
                });

            Lun::lunreq_rm(&dev_ctl, conf::Backend::Ramdisk, 42).unwrap();
        }
    }
}
