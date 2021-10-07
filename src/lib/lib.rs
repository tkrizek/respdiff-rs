use domain::base::{iana::rtype::Rtype, octets::ParseError, Message};
use domain::rdata::Rrsig;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fmt;
use std::time::Duration;

/// Configuration file.
pub mod config;
/// Utilities for working with LMDB database.
pub mod database;
/// JSON data format.
pub mod dataformat;
/// Respdiff errors.
pub mod error;
/// Logic for comparing DNS messages.
pub mod matcher;
/// Sending DNS queries and receving reponses (async).
pub mod transceive;

// -------- Types ---------

/// 32 bit integer representing a key under which the query is stored in LMDB.
pub type QKey = u32;

/// DNS message reply from a server.
#[derive(Clone)]
pub struct DnsReply {
    /// The time it took for the reply to arrive after sending the query.
    pub delay: Duration,
    /// The DNS message received in the the reply.
    ///
    /// The content is only guaranteed to have a DNS header, but the message itself wasn't
    /// parsed and isn't guaranteed to be a valid DNS message.
    pub message: Message<Vec<u8>>,
}

/// Response from a server.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ServerResponse {
    /// No response was received from the server in time.
    Timeout,
    /// Response was received, but it isn't a DNS message.
    Malformed,
    /// Response was received and it seems to be a DNS message.
    Data(DnsReply),
}

/// A set of responses from all servers for a particular query.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ServerResponseList {
    /// Query identifier.
    pub key: QKey,
    /// List of responses in the exact order as read from LMDB.
    pub replies: Vec<ServerResponse>,  // TODO maybe rename -> responses
}

/// Criteria used to compare answers.
#[derive(Deserialize, PartialEq, Eq, Debug, Copy, Clone)]
#[serde(try_from = "String")]
pub enum DiffCriteria {
    Opcode,
    Rcode,
    Flags,
    Question,
    AnswerTypes,
    AnswerRrsigs,
    // FIXME these have not been implemented, since we don't use them
    // Authority,
    // Additional,
    // Edns,
    // Nsid,
}

// ----- DnsReply --------

impl DnsReply {
    /// Return list of unique non-RRSIG record types present in answer.
    pub fn answer_rtypes(&self) -> Result<BTreeSet<Rtype>, ParseError> {
        let mut rtypes = BTreeSet::new();
        for rr in self.message.answer()? {
            let rtype = rr?.rtype();
            if rtype != Rtype::Rrsig {
                rtypes.insert(rtype);
            }
        }
        Ok(rtypes)
    }
    /// Return list of unique types that are covered by any RRSIG in answer.
    pub fn answer_rrsig_covered(&self) -> Result<BTreeSet<Rtype>, ParseError> {
        let mut covered = BTreeSet::new();
        for rr in self.message.answer()? {
            let rr = rr?;
            if rr.rtype() == Rtype::Rrsig {
                if let Some(sig) = rr.into_record::<Rrsig<_, _>>()? {
                    covered.insert(sig.data().type_covered());
                }
            }
        }
        Ok(covered)
    }
}
impl PartialEq for DnsReply {
    fn eq(&self, other: &Self) -> bool {
        self.delay == other.delay && self.message.as_octets() == other.message.as_octets()
    }
}
impl Eq for DnsReply {}
impl fmt::Debug for DnsReply {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DnsReply")
            .field("delay", &self.delay)
            .field("msgid", &self.message.header().id())
            .finish_non_exhaustive()
    }
}
