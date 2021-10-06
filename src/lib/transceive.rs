use crate::{config::ServerConfig, database::queriesdb::Query};
/// Module for asynchronously transmitting queries.
use async_std::{
    io,
    net::{SocketAddr, UdpSocket},
    prelude::*,
    task,
};
use futures::channel::mpsc;
use futures::sink::SinkExt;
use futures::stream::FuturesUnordered;
use std::mem;
use std::time::{Duration, Instant};

pub type Sender<T> = mpsc::UnboundedSender<T>;
pub type Receiver<T> = mpsc::UnboundedReceiver<T>;
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub async fn send_loop(
    queries: Vec<Query>,
    servers: Vec<ServerConfig>,
    sink: Sender<Vec<RawResponse>>,
    timeout: Duration,
    qps: u32,
) -> Result<()> {
    let addrs: Vec<_> = servers
        .iter()
        .map(|sconf| SocketAddr::new(sconf.ip, sconf.port))
        .collect();
    let delay = Duration::from_secs_f64(1. / qps as f64);

    for query in queries {
        task::spawn(transmit_query(
            query.wire,
            addrs.clone(),
            sink.clone(),
            timeout.clone(),
        ));
        task::sleep(delay).await; // not exactly precise
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub enum RawResponse {
    Timeout,
    Data { delay: Duration, wire: Vec<u8> },
}

impl PartialEq for RawResponse {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (RawResponse::Timeout, RawResponse::Timeout) => true,
            (RawResponse::Timeout, RawResponse::Data { .. }) => false,
            (RawResponse::Data { .. }, RawResponse::Timeout) => false,
            (RawResponse::Data { wire: w1, .. }, RawResponse::Data { wire: w2, .. }) => w1 == w2,
        }
    }
}
impl Eq for RawResponse {}

async fn transmit_query(
    qwire: Vec<u8>,
    addrs: Vec<SocketAddr>,
    mut sink: Sender<Vec<RawResponse>>,
    timeout: Duration,
) -> Result<()> {
    let mut futures = FuturesUnordered::new();

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
            Ok((
                i,
                RawResponse::Data {
                    delay: since.elapsed(),
                    wire: buf[0..n].to_vec(),
                },
            ))
        });
        futures.push(reply);
    }

    let mut replies = vec![RawResponse::Timeout; addrs.len()];
    while let Some(res) = futures.next().await {
        if let Ok((i, reply)) = res {
            let _ = mem::replace(&mut replies[i], reply);
        }
    }

    sink.send(replies).await?;
    Ok(())
}

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
                sender.clone(),
                Duration::from_millis(10),
            )
            .await
            .is_ok());
            assert!(transmit_query(
                query.wire.clone(),
                vec![addr],
                sender.clone(),
                Duration::from_millis(10),
            )
            .await
            .is_ok());
            drop(sender);
            assert_eq!(
                receiver.next().await,
                Some(vec![RawResponse::Data {
                    delay: Duration::from_secs(0),
                    wire: query.wire.clone()
                }])
            );
            assert_eq!(receiver.next().await, Some(vec![RawResponse::Timeout]));
            assert_eq!(receiver.next().await, None);
        });
        let echo = task::spawn(udp_echo_once(socket));

        let mut futures = FuturesUnordered::new();
        futures.push(transmission);
        futures.push(echo);
        while let Some(_) = futures.next().await {};

        Ok(())
    }
}
