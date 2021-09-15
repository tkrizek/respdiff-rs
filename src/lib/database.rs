use lmdb::{Database, DatabaseFlags, Environment, Error as LmdbError};
use std::path::Path;

use crate::{Error, Result};

// Version string of supported respdiff db.
const BIN_FORMAT_VERSION: &str = "2018-05-21";

// Create an LMDB Environment. Only a single instance can exist in a process.
pub fn open_env(dir: &Path) -> Result<Environment> {
    Ok(Environment::new()
        .set_max_dbs(5)
        .set_map_size(10 * 1024_usize.pow(3)) // 10 G
        .set_max_readers(384) // TODO: may need increasing?
        .open(dir)?)
}

// Create or open an LMDB database.
pub fn open_db(env: &Environment, name: &str, create: bool) -> Result<Database> {
    if create {
        Ok(env.create_db(Some(name), DatabaseFlags::empty())?)
    } else {
        Ok(env.open_db(Some(name))?)
    }
}

// Check if database exists already.
pub fn exists_db(env: &Environment, name: &str) -> Result<bool> {
    match env.open_db(Some(name)) {
        Ok(_) => Ok(true),
        Err(LmdbError::NotFound) => Ok(false),
        Err(e) => Err(Error::Database(e)),
    }
}

// Functions to work with the "meta" database.
pub mod metadb {
    use crate::{error::DbFormatError, Error, Result};
    use byteorder::{ByteOrder, LittleEndian};
    use lmdb::{Database, RoTransaction, RwTransaction, Transaction, WriteFlags};
    use std::convert::TryInto;
    use std::time::SystemTime;

    pub const NAME: &str = "meta";

    pub fn write_version(db: Database, txn: &mut RwTransaction) -> Result<()> {
        Ok(txn.put(
            db,
            b"version",
            &super::BIN_FORMAT_VERSION,
            WriteFlags::empty(),
        )?)
    }

    pub fn write_start_time(db: Database, txn: &mut RwTransaction) -> Result<()> {
        let duration = match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            Ok(val) => val,
            Err(_) => return Err(Error::Time),
        };
        let ts: u32 = match duration.as_secs().try_into() {
            Ok(val) => val,
            Err(_) => return Err(Error::Time),
        };
        let mut bytes = [0; 4];
        LittleEndian::write_u32(&mut bytes, ts);
        Ok(txn.put(db, b"start_time", &bytes, WriteFlags::empty())?)
    }

    pub fn read_start_time(db: Database, txn: &RoTransaction) -> Result<u32> {
        let time = txn.get(db, b"start_time")?;
        Ok(LittleEndian::read_u32(time))
    }

    pub fn read_end_time(db: Database, txn: &RoTransaction) -> Result<u32> {
        let time = txn.get(db, b"end_time")?;
        Ok(LittleEndian::read_u32(time))
    }

    pub fn check_version(db: Database, txn: &RoTransaction) -> Result<String> {
        let version = txn.get(db, b"version")?;
        let version = String::from_utf8(version.to_vec())?;

        if version == super::BIN_FORMAT_VERSION {
            Ok(version)
        } else {
            Err(DbFormatError::Unsupported.into())
        }
    }

    pub fn write_servers(
        db: Database,
        txn: &mut RwTransaction,
        servers: Vec<String>,
    ) -> Result<()> {
        let mut bytes = [0; 4];
        LittleEndian::write_u32(&mut bytes, servers.len() as u32);
        txn.put(db, b"servers", &bytes, WriteFlags::empty())?;

        for (i, name) in servers.into_iter().enumerate() {
            txn.put(db, &format!("name{}", i), &name, WriteFlags::empty())?;
        }
        Ok(())
    }
}

pub mod queriesdb {
    use byteorder::{ByteOrder, LittleEndian};
    use log::warn;
    use std::convert::TryFrom;

    pub const NAME: &str = "queries";

    #[derive(Debug)]
    pub struct Query {
        pub key: u32,
        pub wire: Vec<u8>,
    }

    impl TryFrom<lmdb::Result<(&[u8], &[u8])>> for Query {
        type Error = lmdb::Error;

        fn try_from(item: lmdb::Result<(&[u8], &[u8])>) -> Result<Self, Self::Error> {
            match item {
                Ok((key, val)) => Ok(Query {
                    key: LittleEndian::read_u32(key),
                    wire: val.to_vec(),
                }),
                Err(e) => {
                    warn!("failed to read query from db");
                    Err(e)
                }
            }
        }
    }
}

pub mod answersdb {
    use crate::error::DbFormatError;
    use byteorder::{ByteOrder, LittleEndian};
    use domain::base::{iana::rtype::Rtype, octets::ParseError, Message};
    use domain::rdata::Rrsig;
    use std::collections::BTreeSet;
    use std::convert::TryFrom;
    use std::fmt;
    use std::time::Duration;
    pub const NAME: &str = "answers";

    #[derive(Debug, PartialEq, Eq, Clone)]
    pub enum ServerReply {
        Timeout,
        Malformed,
        Data(DnsReply),
    }

    #[derive(Clone)]
    pub struct DnsReply {
        pub delay: Duration,
        pub message: Message<Vec<u8>>,
    }
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

    #[derive(Debug, PartialEq, Eq)]
    pub struct ServerReplyList {
        pub key: u32,
        pub replies: Vec<ServerReply>,
    }

    impl TryFrom<(&[u8], &[u8])> for ServerReplyList {
        type Error = DbFormatError;

        fn try_from(item: (&[u8], &[u8])) -> Result<Self, Self::Error> {
            let mut replies: Vec<ServerReply> = vec![];
            let (key, buf) = item;
            if key.len() != 4 {
                return Err(DbFormatError::ReplyInvalidData);
            }

            let mut i = 0;
            while (i + 6) <= buf.len() {
                let delay = LittleEndian::read_u32(&buf[i..i + 4]);
                i += 4;
                let len = LittleEndian::read_u16(&buf[i..i + 2]) as usize;
                i += 2;

                if delay == u32::MAX {
                    if len != 0 {
                        return Err(DbFormatError::ReplyInvalidData);
                    } else {
                        replies.push(ServerReply::Timeout);
                        continue;
                    }
                }

                if i + len > buf.len() {
                    return Err(DbFormatError::ReplyMissingData);
                }

                let wire: Vec<u8> = Vec::from(&buf[i..i + len]);
                i += len;

                match Message::from_octets(wire) {
                    Ok(msg) => {
                        replies.push(ServerReply::Data(DnsReply {
                            delay: Duration::from_micros(delay as u64),
                            message: msg,
                        }));
                    }
                    Err(_) => {
                        replies.push(ServerReply::Malformed);
                    }
                }
            }

            if i == buf.len() {
                Ok(ServerReplyList {
                    key: LittleEndian::read_u32(key),
                    replies,
                })
            } else {
                Err(DbFormatError::ReplyMissingData)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::DbFormatError;
    use lmdb::{Error as LmdbError, Transaction};
    use std::convert::TryFrom;
    use tempdir::TempDir;

    #[test]
    fn metadb_version() {
        let dir = TempDir::new("test").unwrap();
        let env = open_env(dir.path()).unwrap();
        let db = open_db(&env, metadb::NAME, true).unwrap();

        let mut txn = env.begin_rw_txn().unwrap();
        metadb::write_version(db, &mut txn).unwrap();
        txn.commit().unwrap();

        let txn = env.begin_ro_txn().unwrap();
        let version = metadb::check_version(db, &txn).unwrap();
        assert_eq!(version, BIN_FORMAT_VERSION);
    }

    #[test]
    fn exists() {
        let dir = TempDir::new("test").unwrap();
        let env = open_env(dir.path()).unwrap();
        let _d1 = open_db(&env, "d1", true).unwrap();

        assert_eq!(exists_db(&env, "d1").unwrap(), true);
        assert_eq!(exists_db(&env, "x").unwrap(), false);

        // trigger DbsFull becuase we set db limit to 5
        let _d2 = open_db(&env, "d2", true).unwrap();
        let _d3 = open_db(&env, "d3", true).unwrap();
        let _d4 = open_db(&env, "d4", true).unwrap();
        let _d5 = open_db(&env, "d5", true).unwrap();

        assert_eq!(
            exists_db(&env, "x"),
            Err(Error::Database(LmdbError::DbsFull))
        );
    }

    #[test]
    fn parse_serverreplylist() {
        use answersdb::{DnsReply, ServerReply, ServerReplyList};
        use domain::base::Message;
        use std::time::Duration;

        let key = vec![0x42, 0x00, 0x00, 0x00];
        let empty = vec![];
        assert_eq!(
            ServerReplyList::try_from((key.as_slice(), empty.as_slice())),
            Ok(ServerReplyList {
                key: 0x42,
                replies: vec![]
            })
        );

        let timeout = vec![0xff, 0xff, 0xff, 0xff, 0x00, 0x00];
        assert_eq!(
            ServerReplyList::try_from((key.as_slice(), timeout.as_slice())),
            Ok(ServerReplyList {
                key: 0x42,
                replies: vec![ServerReply::Timeout,],
            })
        );

        let missingdata = vec![0x00, 0x00, 0x00, 0x00, 0x01, 0x00];
        assert_eq!(
            ServerReplyList::try_from((key.as_slice(), missingdata.as_slice())),
            Err(DbFormatError::ReplyMissingData.into())
        );

        let shortdata = vec![0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00];
        assert_eq!(
            ServerReplyList::try_from((key.as_slice(), shortdata.as_slice())),
            Ok(ServerReplyList {
                key: 0x42,
                replies: vec![ServerReply::Malformed],
            })
        );

        let wire = vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c,
        ];
        let header = vec![0x00, 0x00, 0x00, 0x00, 0x0c, 0x00];
        let mut data = header.to_owned();
        data.append(&mut wire.to_owned());
        let dnsreply = DnsReply {
            delay: Duration::from_micros(0),
            message: Message::from_octets(wire.to_owned()).unwrap(),
        };
        assert_eq!(
            ServerReplyList::try_from((key.as_slice(), data.as_slice())),
            Ok(ServerReplyList {
                key: 0x42,
                replies: vec![ServerReply::Data(dnsreply.to_owned())],
            })
        );

        data.append(&mut timeout.to_owned());
        let header3 = vec![0x01, 0x00, 0x00, 0x00, 0x0c, 0x00];
        data.append(&mut header3.to_owned());
        data.append(&mut wire.to_owned());
        assert_eq!(
            ServerReplyList::try_from((key.as_slice(), data.as_slice())),
            Ok(ServerReplyList {
                key: 0x42,
                replies: vec![
                    ServerReply::Data(dnsreply.to_owned()),
                    ServerReply::Timeout,
                    ServerReply::Data(DnsReply {
                        delay: Duration::from_micros(1),
                        message: Message::from_octets(wire.to_owned()).unwrap(),
                    }),
                ],
            })
        );
    }
}
