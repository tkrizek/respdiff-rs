/// Module for asynchronously transmitting queries.
use async_std::{
    prelude::*,
    io,
    task,
    net::{ SocketAddr, ToSocketAddrs, UdpSocket },
};
use futures::channel::mpsc;
use futures::sink::SinkExt;
use crate::{
    config::ServerConfig,
    database::queriesdb::Query,
};
use std::time::{ Duration, Instant };

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
    query: Query,  // TODO maybe wire is enough?
    addrs: Vec<SocketAddr>,
    timeout: Duration,
    mut sink: Sender<Vec<Response>>
) -> Result<(), io::Error> {
    // start timer
    // https://docs.rs/async-std/1.9.0/async_std/io/fn.timeout.html
    // send to each server (should be pretty much instant)
    // wait for all answers
    // push to sink
    let query = query.wire;

    let addr = addrs[0]; // TODO
    let reply = io::timeout(timeout, async {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.connect(&addr).await?;
        socket.send(&query).await?;
        let since = Instant::now();
        let mut buf = vec![0; 64 * 1024];
        let n = socket.recv(&mut buf).await?;
        Ok(Response::Data {
            delay: since.elapsed(),
            wire: buf[0..n].to_vec(),
        })
    }).await;

    match reply {  // TODO error handling for channel?
        Ok(reply) => sink.send(vec![reply]).await,
        Err(_) => sink.send(vec![Response::Timeout]).await,
    };
    Ok(())
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
            let res = transmit_query(query, vec![addr], Duration::from_millis(100), sender).await.unwrap();
            assert_eq!(receiver.next().await, Some(vec![Response::Timeout]));
            assert_eq!(receiver.next().await, None);
        }).await;
        Ok(())
    }
}
