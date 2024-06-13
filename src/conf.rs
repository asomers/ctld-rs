// TODO:
// parse socketaddrs with or without port
// parse initiator-portal as a netmask

use std::{
    collections::HashMap,
    io::{self, Read},
    net::SocketAddr,
    path::{Path, PathBuf}
};

use strum::EnumString;
use uclicious::*;

#[derive(Clone, Copy, Debug, Default, Eq, EnumString, PartialEq)]
enum AuthType {
    #[default]
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

#[derive(Clone, Copy, Debug, Default, Eq, EnumString, PartialEq)]
enum Backend {
    #[default]
    #[strum(serialize = "block")]
    Block,
    #[strum(serialize = "ramdisk")]
    Ramdisk
}

#[derive(Clone, Copy, Debug, Default, Eq, EnumString, PartialEq)]
enum DeviceType {
    #[default]
    #[strum(serialize = "disk", serialize = "direct", serialize = "0")]
    Disk,
    #[strum(serialize = "processor", serialize = "3")]
    Processor,
    #[strum(serialize = "cd", serialize = "cdrom", serialize = "dvd", serialize = "dvdrom", serialize = "5")]
    Cd
}

#[derive(Clone, Debug, Uclicious)]
#[ucl(skip_builder)]
struct AuthGroup {
    #[ucl(path = "auth-type", from_str)]
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
struct PortalGroup {
    #[ucl(path = "discovery-auth-group")]
    discovery_auth_group: String,
    #[ucl(path = "discovery-filter", default, from_str)]
    discovery_filter: DiscoveryFilter,
    #[ucl(from_str)]
    listen: SocketAddr,
    // listen-iser is not implemented
    #[ucl(default)]
    offload: Option<String>,
    #[ucl(default)]
    option: HashMap<String, String>,
    #[ucl(default)]
    redirect: Option<String>,
    #[ucl(default)]
    tag: Option<u16>,
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
struct Lun {
    #[ucl(default, from_str)]
    backend: Backend,
    #[ucl(default)]
    blocksize: Option<i32>,
    #[ucl(default)]
    ctl_lun: Option<i32>,
    #[ucl(path = "device-id")]
    device_id: String,
    #[ucl(default, path = "device-type", from_str)]
    device_type: DeviceType,
    #[ucl(default)]
    option: HashMap<String, String>,
    path: PathBuf,
    #[ucl(default)]
    serial: Option<String>,
    #[ucl(default)]
    size: u64,
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
    auth_group: HashMap<String, AuthGroup>,
    #[ucl(default = "0")]
    debug: i32,
    #[ucl(default = "30")]
    maxproc: i32,
    #[ucl(default = "PathBuf::from(\"/var/run/ctld.pid\")")]
    pidfile: PathBuf,
    #[ucl(path = "portal-group")]
    portal_group: HashMap<String, PortalGroup>,
    lun: HashMap<String, Lun>,
    target: HashMap<String, Target>,
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
    pub fn open<P: AsRef<Path>>(p: P) -> io::Result<Self> {
        let mut f = std::fs::File::open(p)?;
        let mut contents = String::new();
        f.read_to_string(&mut contents)?;
        // TODO: use anyerror here
        let mut builder = Conf::builder().unwrap();
        builder.add_chunk_full(&contents, Priority::default(), DEFAULT_DUPLICATE_STRATEGY).unwrap();
        let conf: Conf = builder.build().unwrap();
        Ok(conf)
    }

    /// Create a Conf structure reflecting the kernel's current configuration.
    pub fn from_xml(xml: &str) -> io::Result<()> {
        todo!()
    }
}
