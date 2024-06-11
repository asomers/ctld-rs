use std::{
    collections::HashMap,
    io::{self, Read},
    net::SocketAddr,
    path::PathBuf
};

use strum::EnumString;
use uclicious::*;

#[derive(Clone, Copy, Debug, Eq, EnumString, PartialEq)]
enum AuthType {
    #[strum(serialize = "none")]
    None,
    #[strum(serialize = "deny")]
    Deny,
    #[strum(serialize = "chap")]
    Chap,
    #[strum(serialize = "chap-mutual")]
    ChapMutual
}

#[derive(Clone, Debug, Uclicious)]
#[ucl(skip_builder)]
struct AuthGroup {
    #[ucl(path = "auth-type", from_str)]
    auth_type: AuthType,
    #[ucl(path = "initiator-portal")]
    initiator_portal: Vec<String>
}

#[derive(Clone, Debug, Uclicious)]
#[ucl(skip_builder)]
struct PortalGroup {
    #[ucl(path = "discovery-auth-group")]
    discovery_auth_group: String,
    // Note: real ctld allows listen to be specified with or without a port
    #[ucl(from_str)]
    listen: SocketAddr
}

#[derive(Clone, Debug, Uclicious)]
#[ucl(skip_builder)]
struct Lun {
    blocksize: i32,
    #[ucl(path = "device-id")]
    device_id: String,
    path: PathBuf,
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
    #[ucl(path = "auth-group")]
    auth_group: String,
    #[ucl(path = "portal-group")]
    portal_group: TargetPortalGroup,
    lun: Vec<TargetLun>
}

/// The UCL configuration file format
#[derive(Debug, Uclicious)]
struct Conf {
    #[ucl(default = "PathBuf::from(\"/var/run/ctld.pid\")")]
    pidfile: PathBuf,
    #[ucl(path = "auth-group")]
    auth_group: HashMap<String, AuthGroup>,
    #[ucl(path = "portal-group")]
    portal_group: HashMap<String, PortalGroup>,
    lun: HashMap<String, Lun>,
    target: HashMap<String, Target>,
}

fn main() {
    let mut f = std::fs::File::open("/usr/home/somers/src/rust/ctld/ctl.conf").unwrap();
    let mut contents = String::new();
    f.read_to_string(&mut contents).unwrap();
    let mut builder = Conf::builder().unwrap();
    builder.add_chunk_full(&contents, Priority::default(), DEFAULT_DUPLICATE_STRATEGY).unwrap();
    let conf: Conf = builder.build().unwrap();
    dbg!(&conf);
}
