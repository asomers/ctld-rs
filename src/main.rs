use std::{
    ffi::{CStr, OsStr},
    fs,
    io,
    os::unix::ffi::OsStrExt,
    path::PathBuf,
    sync::atomic::{AtomicU16, Ordering}
};

use anyhow::{Context, Result};
use clap::Parser;

mod conf;
use crate::conf::Conf;
mod ffi;
mod kernel;

static LAST_PORTAL_GROUP_TAG: AtomicU16 = AtomicU16::new(0xff);

#[derive(Debug, Default, clap::Parser)]
struct Cli {
    /// config file path
    #[clap(default_value = "/etc/ctl.conf", short = 'f')]
    config: PathBuf,
    /// test the configuration file for validity and exit
    #[clap(short = 't')]
    test: bool
}

/// Apply an initial configuration to the running kernel
fn apply_conf(
    ctl_fd: &fs::File,
    klun_list: &kernel::Ctllunlist,
    kport_list: &kernel::Ctlportlist,
    conf: &mut Conf) -> Result<()>
{
    assert!(klun_list.lun.is_empty(), "Handling preexisting LUNs is TODO");
    for kport in kport_list.targ_port.iter() {
        if !["camsim", "tpc", "ioctl"].contains(&kport.frontend_type.as_str()) {
            panic!("Handling preexisting ports is TODO");
        }
    }
    // Go through the new portal groups, assigning tags
    for (_name, pg) in conf.portal_groups.iter_mut() {
        assert!(pg.tag.is_none(), "todo");
        pg.tag = Some(LAST_PORTAL_GROUP_TAG.fetch_add(1, Ordering::Relaxed));
    }

    // Add any LUNs from the config file
    for (name, lun) in conf.luns.iter() {
        kernel::add_lun(&ctl_fd, name.as_str(), lun)?;
    }
    todo!()
}

fn main() -> Result<()> {
    let cli: Cli = Cli::parse();

    let mut conf = Conf::open(&cli.config)?;
    dbg!(&conf);
    if cli.test {
        return Ok(());
    }

    // TODO: open pidfile
    // TODO: set loglevel based on conf.debug

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

    apply_conf(&ctl_fd, &klun_list, &kport_list, &mut conf)?;

    Ok(())
}
