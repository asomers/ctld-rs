//! Helper utility to dump the kernel's XML config
use std::{
    ffi::{CStr, OsStr},
    fs,
    io,
    os::unix::ffi::OsStrExt,
};

use clap::Parser;

mod ffi;
mod kernel;

#[derive(Debug, Default, clap::Parser)]
struct Cli {
    /// dump the kernel's LUN list
    #[clap(short = 'l')]
    lun: bool,
    /// dump the kernel's port list
    #[clap(short = 'p')]
    port: bool
}

fn main() -> io::Result<()> {
    let cli: Cli = Cli::parse();

    let ctl_dev_path = {
        let cstr = CStr::from_bytes_until_nul(ffi::CTL_DEFAULT_DEV).unwrap();
        OsStr::from_bytes(cstr.to_bytes())
    };
    let ctl_fd = fs::File::open(&ctl_dev_path)?;

    if cli.lun {
        let xml = kernel::get_lun_list(&ctl_fd)?;
        println!("{}", xml);
    }

    if cli.port {
        let xml = kernel::get_port_list(&ctl_fd)?;
        println!("{}", xml);
    }

    Ok(())
}

