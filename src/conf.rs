// TODO:
// parse socketaddrs with or without port
// parse initiator-portal as a netmask

use std::{
    collections::HashMap,
    io::Read,
    net::SocketAddr,
    path::{Path, PathBuf}
};

use anyhow::{Context, Result, anyhow};
use serde_derive::{Deserialize};
use strum::{EnumString, IntoStaticStr};
use uclicious::*;

#[derive(Clone, Copy, Debug, Default, Eq, EnumString, PartialEq)]
enum AuthType {
    #[default]
    Unknown,
    #[strum(serialize = "none")]
    None,
    #[strum(serialize = "deny")]
    Deny,
    #[strum(serialize = "chap")]
    Chap,
    #[strum(serialize = "chap-mutual")]
    ChapMutual
}

#[derive(Clone, Copy, Debug, Default, Eq, EnumString, PartialEq)]
enum DiscoveryFilter {
    #[default]
    #[strum(serialize = "none")]
    None,
    #[strum(serialize = "portal")]
    Portal,
    #[strum(serialize = "portal-name")]
    PortalName,
    #[strum(serialize = "portal-name-auth")]
    PortalNameAuth
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, EnumString, IntoStaticStr, PartialEq)]
pub enum Backend {
    #[default]
    #[strum(serialize = "block")]
    #[serde(rename = "block")]
    Block,
    #[strum(serialize = "ramdisk")]
    #[serde(rename = "ramdisk")]
    Ramdisk
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, EnumString, PartialEq)]
pub enum DeviceType {
    #[default]
    #[strum(serialize = "disk", serialize = "direct", serialize = "0")]
    #[serde(rename = "0")]
    Disk = 0,
    #[strum(serialize = "processor", serialize = "3")]
    #[serde(rename = "3")]
    Processor = 3,
    #[strum(serialize = "cd", serialize = "cdrom", serialize = "dvd", serialize = "dvdrom", serialize = "5")]
    #[serde(rename = "5")]
    Cd = 5
}

#[derive(Clone, Debug, Uclicious)]
#[ucl(skip_builder)]
struct AuthGroup {
    #[ucl(path = "auth-type", default, from_str)]
    auth_type: AuthType,
    #[ucl(default)]
    chap: Vec<Chap>,
    #[ucl(default, path = "chap-mutual")]
    chap_mutual: Vec<ChapMutual>,
    #[ucl(default, path = "initiator-name")]
    intiator_name: Option<String>,
    #[ucl(path = "initiator-portal", default)]
    initiator_portal: Vec<String>
}

impl AuthGroup {
    fn validate(&self) -> Result<()> {
        if self.chap.len() > 0 && self.chap_mutual.len() > 0 {
            return Err(anyhow!("Cannot specify both chap and chap-mutual for the same auth-group"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Uclicious)]
#[ucl(skip_builder)]
struct Chap {
    user: String,
    secret: String
}

#[derive(Clone, Debug, Uclicious)]
#[ucl(skip_builder)]
struct ChapMutual {
    user: String,
    secret: String,
    #[ucl(path = "mutual-user")]
    mutual_user: String,
    #[ucl(path = "mutual-secret")]
    mutual_secret: String,
}

#[derive(Clone, Debug, Uclicious)]
#[ucl(skip_builder)]
pub struct PortalGroup {
    #[ucl(path = "discovery-auth-group")]
    discovery_auth_group: String,
    #[ucl(path = "discovery-filter", default, from_str)]
    discovery_filter: DiscoveryFilter,
    // TODO: allow listen to be specified with or without a port number
    #[ucl(from_str)]
    listen: SocketAddr,
    // listen-iser is not implemented
    #[ucl(default)]
    offload: Option<String>,
    #[ucl(default, path = "option")]
    options: HashMap<String, String>,
    #[ucl(default)]
    redirect: Option<String>,
    #[ucl(default)]
    pub tag: Option<u16>,
    #[ucl(default)]
    foreign: bool,
    // TODO: parse the custom constants for DSCP, like "CSx"
    #[ucl(default)]
    dscp: Option<i32>,
    #[ucl(default)]
    pcp: Option<i32>
}

#[derive(Clone, Debug, Uclicious)]
#[ucl(skip_builder)]
pub struct Lun {
    #[ucl(default, from_str)]
    pub backend: Backend,
    #[ucl(default)]
    pub blocksize: Option<u32>,
    #[ucl(default)]
    pub ctl_lun: Option<u32>,
    #[ucl(path = "device-id")]
    pub device_id: String,
    #[ucl(default, path = "device-type", from_str)]
    pub device_type: DeviceType,
    #[ucl(default, path = "option")]
    pub options: HashMap<String, String>,
    pub path: PathBuf,
    #[ucl(default)]
    pub serial: Option<String>,
    /// Must be specified for ramdisk-backed LUNs.  Optional for block-backed.
    #[ucl(default)]
    pub size: Option<u64>,
}

#[derive(Clone, Debug, Uclicious)]
#[ucl(skip_builder)]
struct TargetLun {
    number: u64,
    name: String
}

#[derive(Clone, Debug, Uclicious)]
#[ucl(skip_builder)]
struct TargetPortalGroup {
    name: String,
    #[ucl(default, path = "ag-name")]
    ag_name: Option<String>
}

#[derive(Clone, Debug, Uclicious)]
#[ucl(skip_builder)]
struct Target {
    #[ucl(default)]
    alias: Option<String>,
    #[ucl(path = "auth-group")]
    auth_group: String,
    #[ucl(path = "auth-type", default, from_str)]
    auth_type: AuthType,
    #[ucl(default)]
    chap: Vec<Chap>,
    #[ucl(default, path = "chap-mutual")]
    chap_mutual: Vec<ChapMutual>,
    #[ucl(default, path = "initiator-name")]
    intiator_name: Option<String>,
    #[ucl(path = "initiator-portal", default)]
    initiator_portal: Vec<String>,
    #[ucl(path = "portal-group")]
    portal_group: TargetPortalGroup,
    #[ucl(default)]
    port: Option<String>,
    #[ucl(default)]
    redirect: Option<String>,
    lun: Vec<TargetLun>,
}

/// The UCL configuration file format
#[derive(Debug, Uclicious)]
pub struct Conf {
    #[ucl(path = "auth-group")]
    auth_groups: HashMap<String, AuthGroup>,
    #[ucl(default = "0")]
    debug: i32,
    #[ucl(default = "30")]
    maxproc: i32,
    #[ucl(default = "PathBuf::from(\"/var/run/ctld.pid\")")]
    pidfile: PathBuf,
    #[ucl(path = "portal-group")]
    pub portal_groups: HashMap<String, PortalGroup>,
    #[ucl(path = "lun")]
    pub luns: HashMap<String, Lun>,
    #[ucl(path = "target")]
    targets: HashMap<String, Target>,
    #[ucl(default = "60")]
    timeout: i32,
    #[ucl(default, path = "isns-server")]
    isns_server: Vec<SocketAddr>,
    #[ucl(path = "isns-period", default = "900")]
    isns_period: i32,
    #[ucl(path = "isns-timeout", default = "5")]
    isns_timeout: i32
}

impl Conf {
    pub fn open<P: AsRef<Path>>(p: P) -> Result<Self> {
        let mut f = std::fs::File::open(p).context("opening config file")?;
        let mut contents = String::new();
        f.read_to_string(&mut contents).context("reading config file")?;
        let mut builder = Conf::builder().unwrap();
        builder.add_chunk_full(&contents, Priority::default(), DEFAULT_DUPLICATE_STRATEGY)
            .context("parsing config file")?;
        let conf: Conf = builder.build().map_err(|e| anyhow::Error::msg(format!("{}", e)))?;
        conf.validate()?;
        Ok(conf)
    }

    fn validate(&self) -> Result<()> {
        for (_, ag) in self.auth_groups.iter() {
            ag.validate()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod t {
    use super::*;

    use std::io::Write;

    use tempfile::NamedTempFile;

    /// It is an error to mix chap and chap-mutual entries for the same auth-group
    #[test]
    fn chap_and_chap_mutual() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"
auth-group ag0 {
    chap = [{
        user = foo
        secret = bar
    }]
    chap-mutual = [{
        user = foo
        secret = bar
        mutual-user = \"mutualfoo\"
        mutual-secret = \"mutualbar\"
    }]
}
portal-group  {}
lun {}
target {
}").unwrap();
        Conf::open(f.path()).unwrap_err();
    }
}
