use serde::Deserialize;
use std::path::Path;
use crate::{Result, RespdiffError};

#[derive(Deserialize, PartialEq, Debug)]
pub struct Config {
    test: u64,
    sendrecv: SendRecvConfig,
}

#[derive(Deserialize, PartialEq, Debug)]
pub struct SendRecvConfig {
    timeout: f64,
    jobs: u64,
    time_delay_min: f64,
    time_delay_max: f64,
    max_timeouts: u64,
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
max_timeouts = 10";

    fn expected() -> Config {
        Config {
            test: 3,
            sendrecv: SendRecvConfig {
                timeout: 16.0,
                jobs: 16,
                time_delay_min: 0.0,
                time_delay_max: 0.0,
                max_timeouts: 10,
            },
        }
    }

    #[test]
    fn test_de() {
        assert_eq!(expected(), serde_ini::from_str::<Config>(TEST_INPUT).unwrap());
    }
}
