/// Module for asynchronously transmitting queries.
use async_std::{
    prelude::*,
    task,
    net::{ SocketAddr, ToSocketAddrs, UdpSocket },
};
use futures::channel::mpsc;
use futures::sink::SinkExt;
use crate::{
    config::ServerConfig,
    database::queriesdb::Query,
};
use std::time::Duration;

type Sender<T> = mpsc::UnboundedSender<T>;
type Receiver<T> = mpsc::UnboundedReceiver<T>;

async fn send_loop(
    queries: Receiver<Query>,
    servers: Vec<ServerConfig>,
    timeout: Duration,
    qps: u32,
) {
    // convert servers to addrs
}

#[derive(Debug, PartialEq, Eq)]
enum Response {
    Timeout,
    Data { delay: Duration, wire: Vec<u8> },
}

async fn transmit_query(
    query: Query,
    addrs: Vec<SocketAddr>,
    timeout: Duration,
    mut sink: Sender<Vec<Response>>
) {
    // start timer
    // https://docs.rs/async-std/1.9.0/async_std/io/fn.timeout.html
    // send to each server (should be pretty much instant)
    // wait for all answers
    // push to sink
    sink.send(vec![]).await;
}

async fn recv_loop(replies: Receiver<Vec<Response>>) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_transmit_query() -> std::io::Result<()> {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let query = Query {
            key: 42,
            wire: vec![0x00, 0x01],
        };
        task::spawn(async move {
            let sock = UdpSocket::bind(addr).await.unwrap();
            let addr = sock.local_addr().unwrap();
            let (sender, mut receiver) = mpsc::unbounded();
            transmit_query(query, vec![addr], Duration::from_millis(100), sender).await;
            assert_eq!(receiver.next().await, Some(vec![]));
            assert_eq!(receiver.next().await, None);
        }).await;
        Ok(())
    }
}
