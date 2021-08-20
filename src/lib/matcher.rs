use std::collections::HashSet;
use std::convert::From;
use crate::database::answersdb::{DnsReply, ServerReply};  // TODO weird location
use domain::base::{iana, Question};

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub enum Field {
    Timeout,
    Malformed,
    Opcode,
    Rcode,
    Flags,
    Question,
    // TODO other fields
}

trait Matcher {
    fn mismatch(&self, expected: &DnsReply, got: &DnsReply) -> Option<Mismatch>;
}

impl Matcher for Field {
    fn mismatch(&self, expected: &DnsReply, got: &DnsReply) -> Option<Mismatch> {
        match self {
            Field::Opcode => {
                let expected = expected.message.header().opcode();
                let got = got.message.header().opcode();
                if expected != got {
                    return Some(Mismatch::Opcode(expected, got));
                }
            },
            Field::Rcode => {
                let expected = expected.message.header().rcode();
                let got = got.message.header().rcode();
                if expected != got {
                    return Some(Mismatch::Rcode(expected, got));
                }
            },
            Field::Flags => {
                let expected: Flags = expected.message.header().into();
                let got: Flags = got.message.header().into();
                if expected != got {
                    return Some(Mismatch::Flags(expected, got));
                }
            },
            Field::Question => {
                let mut exp_qs = Vec::new();
                let mut got_qs = Vec::new();
                for q in expected.message.question() {
                    match q {
                        Ok(q) => exp_qs.push(q.to_string()),
                        Err(_) => return Some(Mismatch::MalformedExpected),
                    }
                }
                for q in got.message.question() {
                    match q {
                        Ok(q) => got_qs.push(q.to_string()),
                        Err(_) => return Some(Mismatch::MalformedGot),
                    }
                }
                exp_qs.sort_unstable();
                got_qs.sort_unstable();
                if expected != got {
                    return Some(Mismatch::Question(
                        format!("{:?}", exp_qs),
                        format!("{:?}", got_qs)));
                }
            },
            Field::Timeout => {},
            Field::Malformed => {},
        }
        None
    }
}

#[derive(Default, Eq, PartialEq, Copy, Clone, Debug, Hash)]
pub struct Flags {
    qr: bool,
    aa: bool,
    tc: bool,
    rd: bool,
    ra: bool,
    ad: bool,
    cd: bool,
}

impl From<domain::base::Header> for Flags {
    fn from(header: domain::base::Header) -> Flags {
        Flags {
            qr: header.qr(),
            aa: header.aa(),
            tc: header.tc(),
            rd: header.rd(),
            ra: header.ra(),
            ad: header.ad(),
            cd: header.cd(),
        }
    }
}

impl From<&str> for Flags {
    fn from(repr: &str) -> Flags {
        let mut flags: Flags = Default::default();
        let tokens: Vec<String> = repr.split(' ').map(|x| x.to_uppercase()).collect();
        for token in tokens {
            let token: &str = &token;
            match token {
                "QR" => flags.qr = true,
                "AA" => flags.aa = true,
                "TC" => flags.tc = true,
                "RD" => flags.rd = true,
                "RA" => flags.ra = true,
                "AD" => flags.ad = true,
                "CD" => flags.cd = true,
                _ => {},
            }
        }
        flags
    }
}

impl From<Flags> for String {
    fn from(flags: Flags) -> String {
        let mut tokens = vec![];
        if flags.qr {
            tokens.push("QR");
        }
        if flags.aa {
            tokens.push("AA");
        }
        if flags.tc {
            tokens.push("TC");
        }
        if flags.rd {
            tokens.push("RD");
        }
        if flags.ra {
            tokens.push("RA");
        }
        if flags.ad {
            tokens.push("AD");
        }
        if flags.cd {
            tokens.push("CD");
        }
        tokens.join(" ")
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum Mismatch {
    TimeoutExpected,
    TimeoutGot,
    MalformedExpected,
    MalformedGot,
    MalformedBoth,
    Opcode(iana::opcode::Opcode, iana::opcode::Opcode),
    Rcode(iana::rcode::Rcode, iana::rcode::Rcode),
    Flags(Flags, Flags),
    Question(String, String),
}

pub fn compare(
    expected: &ServerReply,
    got: &ServerReply,
    criteria: &Vec<Field>,
    ) -> HashSet<Mismatch>
{
    let mut mismatches = HashSet::new();

    match (expected, got) {
        (&ServerReply::Timeout, &ServerReply::Timeout) => {},
        (&ServerReply::Timeout, _) => {
            mismatches.insert(Mismatch::TimeoutExpected);
        },
        (_, &ServerReply::Timeout) => {
            mismatches.insert(Mismatch::TimeoutGot);
        },
        (&ServerReply::Malformed, &ServerReply::Malformed) => {
            mismatches.insert(Mismatch::MalformedBoth);
        },
        (&ServerReply::Malformed, _) => {
            mismatches.insert(Mismatch::MalformedExpected);
        }
        (_, &ServerReply::Malformed) => {
            mismatches.insert(Mismatch::MalformedGot);
        },
        (&ServerReply::Data(ref expected), &ServerReply::Data(ref got)) => {
            for field in criteria {
                if let Some(mismatch) = field.mismatch(expected, got) {
                    mismatches.insert(mismatch);
                }
            }
        },
    };

    mismatches
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::base::MessageBuilder;
    use std::time::Duration;

    fn reply_noerror() -> ServerReply {
        ServerReply::Data(DnsReply{
            delay: Duration::from_micros(0),
            message: MessageBuilder::new_vec().into_message(),
        })
    }

    #[test]
    fn flags_str() {
        let flags = Flags {
            qr: true,
            ..Default::default()
        };
        let repr = "QR";
        assert_eq!(repr, String::from(flags));
        assert_eq!(Flags::from(repr), flags);
        let flags = Flags {
            qr: true,
            aa: true,
            tc: true,
            rd: true,
            ra: true,
            ad: true,
            cd: true,
        };
        let repr = "QR AA TC RD RA AD CD";
        assert_eq!(repr, String::from(flags));
        assert_eq!(Flags::from(repr), flags);
    }

    #[test]
    fn compare_timeout() {
        let crit = vec![];
        let res = compare(&ServerReply::Timeout, &ServerReply::Timeout, &crit);
        assert_eq!(res.len(), 0);

        let res = compare(&ServerReply::Malformed, &ServerReply::Timeout, &crit);
        assert_eq!(res.len(), 1);
        assert!(res.contains(&Mismatch::TimeoutGot));

        let res = compare(&ServerReply::Timeout, &ServerReply::Malformed, &crit);
        assert_eq!(res.len(), 1);
        assert!(res.contains(&Mismatch::TimeoutExpected));

        let res = compare(&ServerReply::Timeout, &reply_noerror(), &vec![Field::Opcode]);
        assert_eq!(res.len(), 1);
        assert!(res.contains(&Mismatch::TimeoutExpected));
    }

    #[test]
    fn compare_malformed() {
        let crit = vec![];
        let res = compare(&ServerReply::Malformed, &ServerReply::Malformed, &crit);
        assert_eq!(res.len(), 1);
        assert!(res.contains(&Mismatch::MalformedBoth));

        let res = compare(&reply_noerror(), &ServerReply::Malformed, &crit);
        assert_eq!(res.len(), 1);
        assert!(res.contains(&Mismatch::MalformedGot));

        let res = compare(&ServerReply::Malformed, &reply_noerror(), &crit);
        assert_eq!(res.len(), 1);
        assert!(res.contains(&Mismatch::MalformedExpected));
    }

    #[test]
    fn compare_opcode() {
        use iana::opcode::Opcode::*;

        let crit = vec![Field::Opcode];
        let r1 = &reply_noerror();
        let res = compare(r1, r1, &crit);
        assert_eq!(res.len(), 0);

        let r2 = &mut r1.to_owned();
        if let ServerReply::Data(ref mut dns) = r2 {
            dns.message.header_mut().set_opcode(Status);
        };
        let res = compare(r1, r2, &crit);
        assert_eq!(res.len(), 1);
        assert!(res.contains(&Mismatch::Opcode(Query, Status)));
        let res = compare(r2, r1, &crit);
        assert!(res.contains(&Mismatch::Opcode(Status, Query)));
        let res = compare(r1, r2, &vec![]);
        assert_eq!(res.len(), 0);
    }

    #[test]
    fn compare_rcode() {
        use iana::rcode::Rcode::*;

        let crit = vec![Field::Rcode];
        let r1 = &reply_noerror();
        let res = compare(r1, r1, &crit);
        assert_eq!(res.len(), 0);

        let r2 = &mut r1.to_owned();
        if let ServerReply::Data(ref mut dns) = r2 {
            dns.message.header_mut().set_rcode(ServFail);
        };
        let res = compare(r1, r2, &crit);
        assert_eq!(res.len(), 1);
        assert!(res.contains(&Mismatch::Rcode(NoError, ServFail)));
        let res = compare(r2, r1, &crit);
        assert!(res.contains(&Mismatch::Rcode(ServFail, NoError)));
        let res = compare(r1, r2, &vec![]);
        assert_eq!(res.len(), 0);
    }

    #[test]
    fn compare_opcode_rcode() {
        use iana::opcode::Opcode::*;
        use iana::rcode::Rcode::*;

        let r1 = &reply_noerror();
        let r2 = &mut r1.to_owned();
        if let ServerReply::Data(ref mut dns) = r2 {
            dns.message.header_mut().set_rcode(ServFail);
            dns.message.header_mut().set_opcode(Status);
        };

        let res = compare(r1, r2, &vec![]);
        assert_eq!(res.len(), 0);
        let res = compare(r1, r2, &vec![Field::Opcode, Field::Rcode]);
        assert_eq!(res.len(), 2);
        assert!(res.contains(&Mismatch::Opcode(Query, Status)));
        assert!(res.contains(&Mismatch::Rcode(NoError, ServFail)));
    }

    #[test]
    fn compare_flags() {
        let crit = vec![Field::Flags];
        let r1 = &reply_noerror();
        let res = compare(r1, r1, &crit);
        assert_eq!(res.len(), 0);

        let r2 = &mut r1.to_owned();
        if let ServerReply::Data(ref mut dns) = r2 {
            dns.message.header_mut().set_aa(true);
        };
        let res = compare(r1, r2, &crit);
        assert_eq!(res.len(), 1);
        assert!(res.contains(&Mismatch::Flags(
            "".into(),
            "AA".into())));
    }

    // TODO compare_question
}
