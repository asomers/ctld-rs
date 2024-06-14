use std::{
    ffi::{CStr, OsStr},
    fs,
    io,
    os::unix::ffi::OsStrExt
};

use anyhow::{Context, Result};

mod conf;
use crate::conf::Conf;
mod ffi;
mod kernel;

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    let conf = Conf::open(&args[1])?;
    dbg!(&conf);

    let ctl_dev_path = {
        let cstr = CStr::from_bytes_until_nul(ffi::CTL_DEFAULT_DEV)
            .context("config file path is not a valid CStr")?;
        OsStr::from_bytes(cstr.to_bytes())
    };
    let ctl_fd = fs::File::open(&ctl_dev_path).context("opening ctl device file")?;

    let klun_list = kernel::Ctllunlist::from_kernel(&ctl_fd).context("getting LUN list")?;
    dbg!(&klun_list);
    let kport_list = kernel::Ctlportlist::from_kernel(&ctl_fd).context("getting port list")?;
    dbg!(&kport_list);

    Ok(())
}
