use crate::{config::ServerConfig, database::queriesdb::Query};
/// Module for asynchronously transmitting queries.
use async_std::{
    io,
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    prelude::*,
    task,
};
use futures::channel::mpsc;
use futures::sink::SinkExt;
use futures::stream::FuturesUnordered;
use std::time::{Duration, Instant};
use std::mem;

type Sender<T> = mpsc::UnboundedSender<T>;
type Receiver<T> = mpsc::UnboundedReceiver<T>;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

async fn qproduce_loop(qps: u32) {
    // let min_delay = 1. / qps;
    // let mut delay = Duration::from_secs(0);
    // let mut prev = Instant::now();
    // let mut last = Instant::now();
    // task::sleep(delay);
}

async fn send_loop(
    mut queries: Receiver<Query>,
    servers: Vec<ServerConfig>,
    mut sink: Sender<Vec<Response>>,
    timeout: Duration,
) {
    let addrs: Vec<_> = servers.iter().map(|sconf| {
        SocketAddr::new(sconf.ip, sconf.port)
    }).collect();

    while let Some(query) = queries.next().await {
        task::spawn(
            transmit_query(query.wire, addrs.clone(), timeout.clone(), sink.clone()));
    }
}

#[derive(Debug,Clone)]
enum Response {
    Timeout,
    Data { delay: Duration, wire: Vec<u8> },
}

impl PartialEq for Response {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Response::Timeout, Response::Timeout) => true,
            (Response::Timeout, Response::Data { .. }) => false,
            (Response::Data { .. }, Response::Timeout) => false,
            (Response::Data { wire: w1, .. }, Response::Data { wire: w2, .. }) => w1 == w2,
        }
    }
}
impl Eq for Response {}

async fn transmit_query(
    qwire: Vec<u8>,
    addrs: Vec<SocketAddr>,
    timeout: Duration,
    mut sink: Sender<Vec<Response>>,
) -> Result<()> {
    let mut futures  = FuturesUnordered::new();

    for (i, addr) in addrs.iter().enumerate() {
        let data = qwire.clone();
        let reply = io::timeout(timeout, async move {
            let bindaddr = match addr {
                SocketAddr::V4(..) => "0.0.0.0:0",
                SocketAddr::V6(..) => "::1:0",
            };
            let socket = UdpSocket::bind(bindaddr).await?;
            socket.connect(&addr).await?;
            socket.send(&data).await?;
            let since = Instant::now();
            let mut buf = vec![0; 64 * 1024];
            let n = socket.recv(&mut buf).await?;
            Ok((i, Response::Data {
                delay: since.elapsed(),
                wire: buf[0..n].to_vec(),
            }))
        });
        futures.push(reply);
    }

    let mut replies = vec![Response::Timeout; addrs.len()];
    while let Some(res) = futures.next().await {
        if let Ok((i, reply)) = res {
            mem::replace(&mut replies[i], reply);
        }
    };

    sink.send(replies).await?;
    Ok(())
}

async fn recv_loop(replies: Receiver<Vec<Response>>) {}

#[cfg(test)]
mod tests {
    use super::*;

    async fn udp_echo_once(socket: UdpSocket) {
        let mut buf = vec![0u8; 1024];
        let (recv, peer) = socket.recv_from(&mut buf).await.unwrap();
        socket.send_to(&buf[..recv], &peer).await.unwrap();
    }

    #[async_std::test]
    async fn test_transmit_query() -> Result<()> {
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let socket = UdpSocket::bind(addr).await.unwrap();
        let addr = socket.local_addr().unwrap();
        let query = Query {
            key: 42,
            wire: vec![
                0x21, 0x26, 0x01, 0x20, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x01, 0x00, 0x01,
            ], // . A
        };
        let transmission = task::spawn(async move {
            let (sender, mut receiver) = mpsc::unbounded();
            assert!(transmit_query(
                query.wire.clone(),
                vec![addr],
                Duration::from_millis(10),
                sender.clone()
            )
            .await
            .is_ok());
            assert!(transmit_query(
                query.wire.clone(),
                vec![addr],
                Duration::from_millis(10),
                sender.clone()
            )
            .await
            .is_ok());
            drop(sender);
            assert_eq!(
                receiver.next().await,
                Some(vec![Response::Data {
                    delay: Duration::from_secs(0),
                    wire: query.wire.clone()
                }])
            );
            assert_eq!(receiver.next().await, Some(vec![Response::Timeout]));
            assert_eq!(receiver.next().await, None);
        });
        let echo = task::spawn(udp_echo_once(socket));

        let mut futures = FuturesUnordered::new();
        futures.push(transmission);
        futures.push(echo);
        task::block_on(async {
            while let Some(_) = futures.next().await {
            }
        });

        Ok(())
    }
}
