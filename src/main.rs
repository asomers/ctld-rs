use std::{
    path::PathBuf,
    sync::atomic::{AtomicU16, Ordering}
};

use anyhow::{Context, Result};
use clap::Parser;

use ctld::kconf;
use ctld::kernel;
use ctld::conf::Conf;

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
    klun_list: &kconf::Ctllunlist,
    kport_list: &kconf::Ctlportlist,
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
        kernel::Lun::create(name.as_str(), lun)?;
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

    let klun_list = kconf::Ctllunlist::from_kernel().context("getting LUN list")?;
    dbg!(&klun_list);
    let kport_list = kconf::Ctlportlist::from_kernel().context("getting port list")?;
    dbg!(&kport_list);

    apply_conf(&klun_list, &kport_list, &mut conf)?;

    Ok(())
}
