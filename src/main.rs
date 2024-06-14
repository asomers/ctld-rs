use std::{
    ffi::{CStr, OsStr},
    fs,
    io,
    os::unix::ffi::OsStrExt,
    path::PathBuf
};

use anyhow::{Context, Result};
use clap::Parser;

mod conf;
use crate::conf::Conf;
mod ffi;
mod kernel;

#[derive(Debug, Default, clap::Parser)]
struct Cli {
    /// config file path
    #[clap(default_value = "/etc/ctl.conf", short = 'f')]
    config: PathBuf,
    /// test the configuration file for validity and exit
    #[clap(short = 't')]
    test: bool
}

fn main() -> Result<()> {
    let cli: Cli = Cli::parse();

    let conf = Conf::open(&cli.config)?;
    dbg!(&conf);
    if cli.test {
        return Ok(());
    }

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
