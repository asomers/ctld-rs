//! Helper utility to dump the kernel's XML config
use std::{
    ffi::{CStr, OsStr},
    fs,
    io,
    os::unix::ffi::OsStrExt,
};

use anyhow::{Context, Result};
use clap::Parser;

use ctld::conf;
use ctld::ffi;
use ctld::kconf;

#[derive(Debug, Default, clap::Parser)]
struct Cli {
    /// dump the kernel's LUN list
    #[clap(short = 'l')]
    lun: bool,
    /// dump the kernel's port list
    #[clap(short = 'p')]
    port: bool
}

fn main() -> Result<()> {
    let cli: Cli = Cli::parse();

    if cli.lun {
        let xml = kconf::Ctllunlist::as_xml().context("getting LUN list")?;
        println!("{}", xml);
    }

    if cli.port {
        let xml = kconf::Ctlportlist::as_xml().context("getting port list")?;
        println!("{}", xml);
    }

    Ok(())
}

