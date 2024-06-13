// TODO:
// parse socketaddrs with or without port
// parse initiator-portal as a netmask

mod conf;
use crate::conf::Conf;

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let conf = Conf::open(&args[1]);
    dbg!(&conf);
}
