use crate::{error::Error, DiffCriteria};
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::net::IpAddr;
use std::path::PathBuf;

/// Configuration file representation
#[derive(Deserialize, PartialEq, Debug, Clone)]
pub struct Config {
    pub sendrecv: SendRecvConfig,
    pub diff: DiffConfig,
    pub report: ReportConfig,
    #[serde(deserialize_with = "servers_from_namelist")]
    pub servers: Vec<String>,
    #[serde(flatten)]
    pub server_data: HashMap<String, ServerConfig>,
}
impl TryFrom<&Option<PathBuf>> for Config {
    type Error = Error;

    fn try_from(path: &Option<PathBuf>) -> Result<Self, Self::Error> {
        let path = match path {
            Some(path) => path.clone(),
            None => PathBuf::from("respdiff.cfg"),
        };
        let file = File::open(path).map_err(Error::ConfigFile)?;
        let buf = BufReader::new(file);
        serde_ini::from_bufread::<_, Config>(buf).map_err(Error::ConfigRead)
    }
}

fn servers_from_namelist<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let m: HashMap<String, String> = Deserialize::deserialize(deserializer)?;
    match m.get("names") {
        Some(namelist) => Ok(namelist
            .split(',')
            .map(|name| name.trim().to_string())
            .collect()),
        None => Err(serde::de::Error::custom(
            "[servers] section missing key 'names'",
        )),
    }
}

/// Sendrecv (transceiver) component configuration
#[derive(Deserialize, PartialEq, Debug, Copy, Clone)]
pub struct SendRecvConfig {
    pub timeout: f64,
    pub jobs: u64,
    pub time_delay_min: f64,
    pub time_delay_max: f64,
    pub max_timeouts: Option<u64>,
}

/// Single server configuration
#[derive(Deserialize, PartialEq, Debug, Copy, Clone)]
pub struct ServerConfig {
    pub ip: IpAddr,
    #[serde(deserialize_with = "port_from_str")]
    pub port: u16,
    pub transport: TransportProtocol,
}

fn port_from_str<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}

/// Transport protocol used to send/receive queries
#[derive(Deserialize, PartialEq, Debug, Copy, Clone)]
#[serde(try_from = "String")]
pub enum TransportProtocol {
    Udp,
    Tcp,
    Tls,
}

impl TryFrom<String> for TransportProtocol {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_ref() {
            "udp" => Ok(TransportProtocol::Udp),
            "tcp" => Ok(TransportProtocol::Tcp),
            "tls" => Ok(TransportProtocol::Tls),
            _ => Err(Error::UnknownTransportProtocol(value.to_string())),
        }
    }
}

/// Msgdiff configuration
#[derive(Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct DiffConfig {
    pub target: String,
    #[serde(deserialize_with = "criteria_from_list")]
    pub criteria: Vec<DiffCriteria>,
}

impl TryFrom<&str> for DiffCriteria {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "opcode" => Ok(DiffCriteria::Opcode),
            "rcode" => Ok(DiffCriteria::Rcode),
            "flags" => Ok(DiffCriteria::Flags),
            "question" => Ok(DiffCriteria::Question),
            "answertypes" => Ok(DiffCriteria::AnswerTypes),
            "answerrrsigs" => Ok(DiffCriteria::AnswerRrsigs),
            //"authority" => Ok(DiffCriteria::Authority),
            //"additional" => Ok(DiffCriteria::Additional),
            //"edns" => Ok(DiffCriteria::Edns),
            //"nsid" => Ok(DiffCriteria::Nsid),
            _ => Err(Error::UnknownDiffCriteria(value.to_string())),
        }
    }
}

impl TryFrom<String> for DiffCriteria {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.try_into()
    }
}

fn criteria_from_list<'de, D>(deserializer: D) -> Result<Vec<DiffCriteria>, D::Error>
where
    D: Deserializer<'de>,
{
    let criterialist: String = Deserialize::deserialize(deserializer)?;
    let criteria: Result<Vec<DiffCriteria>, _> = criterialist
        .split(',')
        .map(|criteria| criteria.trim().try_into())
        .collect();
    criteria.map_err(serde::de::Error::custom)
}

/// DiffReport configuration
#[derive(Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct ReportConfig {
    #[serde(deserialize_with = "field_weights_from_list")]
    pub field_weights: Vec<FieldWeight>,
}

/// Field weights used to produce DiffReport
#[derive(Deserialize, PartialEq, Eq, Debug, Copy, Clone)]
#[serde(try_from = "String")]
pub enum FieldWeight {
    Timeout,
    Malformed,
    Opcode,
    Question,
    Rcode,
    Flags,
    AnswerTypes,
    AnswerRrsigs,
    Answer,
    Authority,
    Additional,
    Edns,
    Nsid,
}

impl TryFrom<&str> for FieldWeight {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "timeout" => Ok(FieldWeight::Timeout),
            "malformed" => Ok(FieldWeight::Malformed),
            "opcode" => Ok(FieldWeight::Opcode),
            "question" => Ok(FieldWeight::Question),
            "rcode" => Ok(FieldWeight::Rcode),
            "flags" => Ok(FieldWeight::Flags),
            "answertypes" => Ok(FieldWeight::AnswerTypes),
            "answerrrsigs" => Ok(FieldWeight::AnswerRrsigs),
            "answer" => Ok(FieldWeight::Answer),
            "authority" => Ok(FieldWeight::Authority),
            "additional" => Ok(FieldWeight::Additional),
            "edns" => Ok(FieldWeight::Edns),
            "nsid" => Ok(FieldWeight::Nsid),
            _ => Err(Error::UnknownFieldWeight(value.to_string())),
        }
    }
}

impl TryFrom<String> for FieldWeight {
    type Error = Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.try_into()
    }
}

fn field_weights_from_list<'de, D>(deserializer: D) -> Result<Vec<FieldWeight>, D::Error>
where
    D: Deserializer<'de>,
{
    let weightlist: String = Deserialize::deserialize(deserializer)?;
    let field_weights: Result<Vec<FieldWeight>, _> = weightlist
        .split(',')
        .map(|field_weight| field_weight.trim().try_into())
        .collect();
    field_weights.map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_INPUT: &'static str = "
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
transport = udp

[diff]
# symbolic name of server under test
# other servers are used as reference when comparing answers from the target
target = cznic

# fields and comparison methods used when comparing two DNS messages
criteria = opcode, rcode, flags, question, answertypes, answerrrsigs
# other supported criteria values: authority, additional, edns, nsid

[report]
# diffsum reports mismatches in field values in this order
# if particular message has multiple mismatches, it is counted only once into category with highest weight
field_weights = timeout, malformed, opcode, question, rcode, flags, answertypes, answerrrsigs, answer, authority, additional, edns, nsid
";

    fn expected() -> Config {
        Config {
            sendrecv: SendRecvConfig {
                timeout: 16.0,
                jobs: 16,
                time_delay_min: 0.0,
                time_delay_max: 0.0,
                max_timeouts: Some(10),
            },
            diff: DiffConfig {
                target: "cznic".to_string(),
                criteria: vec![
                    DiffCriteria::Opcode,
                    DiffCriteria::Rcode,
                    DiffCriteria::Flags,
                    DiffCriteria::Question,
                    DiffCriteria::AnswerTypes,
                    DiffCriteria::AnswerRrsigs,
                ],
            },
            report: ReportConfig {
                field_weights: vec![
                    FieldWeight::Timeout,
                    FieldWeight::Malformed,
                    FieldWeight::Opcode,
                    FieldWeight::Question,
                    FieldWeight::Rcode,
                    FieldWeight::Flags,
                    FieldWeight::AnswerTypes,
                    FieldWeight::AnswerRrsigs,
                    FieldWeight::Answer,
                    FieldWeight::Authority,
                    FieldWeight::Additional,
                    FieldWeight::Edns,
                    FieldWeight::Nsid,
                ],
            },
            servers: vec![
                "google".to_string(),
                "cloudflare".to_string(),
                "cznic".to_string(),
            ],
            server_data: [
                (
                    "cznic",
                    ServerConfig {
                        ip: "185.43.135.1".parse().unwrap(),
                        port: 53,
                        transport: TransportProtocol::Udp,
                    },
                ),
                (
                    "google",
                    ServerConfig {
                        ip: "8.8.8.8".parse().unwrap(),
                        port: 53,
                        transport: TransportProtocol::Tcp,
                    },
                ),
                (
                    "cloudflare",
                    ServerConfig {
                        ip: "1.1.1.1".parse().unwrap(),
                        port: 853,
                        transport: TransportProtocol::Tls,
                    },
                ),
            ]
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_owned()))
            .collect(),
        }
    }

    #[test]
    fn test_de() {
        assert_eq!(
            expected(),
            serde_ini::from_str::<Config>(TEST_INPUT).unwrap()
        );
    }
}
