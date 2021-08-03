use serde::Deserialize;
use std::convert::TryFrom;
use std::path::Path;
use std::net::IpAddr;
use crate::RespdiffError;

#[derive(Deserialize, PartialEq, Debug)]
pub struct Config {
    sendrecv: SendRecvConfig,
    cznic: ServerConfig,  // TODO use actual server array
}

#[derive(Deserialize, PartialEq, Debug)]
pub struct SendRecvConfig {
    timeout: f64,
    jobs: u64,
    time_delay_min: f64,
    time_delay_max: f64,
    max_timeouts: u64,
}

#[derive(Deserialize, PartialEq, Debug)]
pub struct ServerConfig {
    ip: IpAddr,
    port: u16,
    transport: TransportProtocol,
}

#[derive(Deserialize, PartialEq, Debug)]
#[serde(try_from = "String")]
pub enum TransportProtocol {
    Udp,
    Tcp,
    Tls,
}

impl TryFrom<String> for TransportProtocol {
    type Error = RespdiffError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_ref() {
            "udp" => Ok(TransportProtocol::Udp),
            "tcp" => Ok(TransportProtocol::Tcp),
            "tls" => Ok(TransportProtocol::Tls),
            _ => Err(RespdiffError::InvalidTransportProtocol(value.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_INPUT: &'static str = "
test = 3
[sendrecv]
# in seconds (float)
timeout = 16
# number of queries to run simultaneously
jobs = 16
# in seconds (float); delay each query by a random time (uniformly distributed) between min and max; set max to 0 to disable
time_delay_min = 0
time_delay_max = 0
# number of maximum consecutive timeouts received from a single resolver before exiting
max_timeouts = 10

[servers]
names = google, cloudflare, cznic
# symbolic names of DNS servers under test
# separate multiple values by ,

# each symbolic name in [servers] section refers to config section
# containing IP address and port of particular server
[google]
ip = 8.8.8.8
port = 53
transport = tcp
# optional graph color: common names or hex (#00FFFF) allowed
graph_color = cyan
# optional restart script to clean cache and restart resolver, used by diffrepro
# restart_script = /usr/local/bin/restart-kresd

[cloudflare]
ip = 1.1.1.1
port = 853
transport = tls

[cznic]
ip = 185.43.135.1
port = 53
transport = udp";

    fn expected() -> Config {
        Config {
            sendrecv: SendRecvConfig {
                timeout: 16.0,
                jobs: 16,
                time_delay_min: 0.0,
                time_delay_max: 0.0,
                max_timeouts: 10,
            },
            cznic: ServerConfig {
                ip: "185.43.135.1".parse().unwrap(),
                port: 53,
                transport: TransportProtocol::Udp,
            },
        }
    }

    #[test]
    fn test_de() {
        assert_eq!(expected(), serde_ini::from_str::<Config>(TEST_INPUT).unwrap());
    }
}
