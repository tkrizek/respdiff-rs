/// Module for asynchronously transmitting queries.
use async_std::{
    prelude::*,
    task,
    net::{ SocketAddr, ToSocketAddrs, UdpSocket },
};
use futures::channel::mpsc;
use respdiff::{
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

enum Response {
    Timeout,
    Data { delay: Duration, wire: Vec<u8> },
}

async fn transmit_query(
    query: Query,
    addrs: Vec<SocketAddr>,
    timeout: Duration,
    sink: Sender<Vec<Response>>
) {
    // start timer
    // send to each server (should be pretty much instant)
    // wait for all answers
    // push to sink
}

async fn recv_loop(replies: Receiver<Vec<Response>>) {}
