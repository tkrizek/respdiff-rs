use crate::config::DiffCriteria; // TODO move?
use crate::database::answersdb::{DnsReply, ServerReply}; // TODO weird location
use domain::base::{
    header::Flags,
    iana,
    name::{Dname, ToDname},
    question::Question,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Field {
    Timeout,
    Malformed,
    Opcode,
    Rcode,
    Flags,
    Question,
    AnswerTypes,
    AnswerRrsigs,
}

impl From<&Mismatch> for Field {
    fn from(mismatch: &Mismatch) -> Field {
        match mismatch {
            Mismatch::TimeoutExpected => Field::Timeout,
            Mismatch::TimeoutGot => Field::Timeout,
            Mismatch::MalformedExpected => Field::Malformed,
            Mismatch::MalformedGot => Field::Malformed,
            Mismatch::MalformedBoth => Field::Malformed,
            Mismatch::QuestionCount => Field::Question,
            Mismatch::AnswerTypes(_, _) => Field::AnswerTypes,
            Mismatch::AnswerRrsigs(_, _) => Field::AnswerRrsigs,
            Mismatch::Opcode(_, _) => Field::Opcode,
            Mismatch::Rcode(_, _) => Field::Rcode,
            Mismatch::Flags(_, _) => Field::Flags,
            Mismatch::Question(_, _) => Field::Question,
        }
    }
}

trait Matcher {
    fn mismatch(&self, expected: &DnsReply, got: &DnsReply) -> Option<Mismatch>;
}

impl Matcher for DiffCriteria {
    fn mismatch(&self, expected: &DnsReply, got: &DnsReply) -> Option<Mismatch> {
        match self {
            DiffCriteria::Opcode => {
                let expected = expected.message.header().opcode();
                let got = got.message.header().opcode();
                if expected != got {
                    return Some(Mismatch::Opcode(expected, got));
                }
            }
            DiffCriteria::Rcode => {
                let expected = expected.message.header().rcode();
                let got = got.message.header().rcode();
                if expected != got {
                    return Some(Mismatch::Rcode(expected, got));
                }
            }
            DiffCriteria::Flags => {
                let expected: Flags = expected.message.header().flags();
                let got: Flags = got.message.header().flags();
                if expected != got {
                    return Some(Mismatch::Flags(expected, got));
                }
            }
            DiffCriteria::Question => {
                let expected = {
                    if expected.message.question().count() != 1 {
                        return Some(Mismatch::QuestionCount);
                    }
                    if let Some(q) = expected.message.question().next() {
                        match q {
                            Ok(q) => q,
                            Err(_) => return Some(Mismatch::MalformedExpected),
                        }
                    } else {
                        return Some(Mismatch::QuestionCount);
                    }
                };
                let got = {
                    if got.message.question().count() != 1 {
                        return Some(Mismatch::QuestionCount);
                    }
                    if let Some(q) = got.message.question().into_iter().next() {
                        match q {
                            Ok(q) => q,
                            Err(_) => return Some(Mismatch::MalformedExpected),
                        }
                    } else {
                        return Some(Mismatch::QuestionCount);
                    }
                };

                if expected != got {
                    return Some(Mismatch::Question(
                        Question::new(
                            expected.qname().to_vec(),
                            expected.qtype(),
                            expected.qclass(),
                        ),
                        Question::new(got.qname().to_vec(), got.qtype(), got.qclass()),
                    ));
                }
            }
            DiffCriteria::AnswerTypes => {
                let expected = match expected.answer_rtypes() {
                    Ok(val) => val,
                    Err(_) => return Some(Mismatch::MalformedExpected),
                };
                let got = match got.answer_rtypes() {
                    Ok(val) => val,
                    Err(_) => return Some(Mismatch::MalformedGot),
                };
                if expected != got {
                    return Some(Mismatch::AnswerTypes(expected, got));
                }
            }
            DiffCriteria::AnswerRrsigs => {
                let expected = match expected.answer_rrsig_covered() {
                    Ok(val) => val,
                    Err(_) => return Some(Mismatch::MalformedExpected),
                };
                let got = match got.answer_rrsig_covered() {
                    Ok(val) => val,
                    Err(_) => return Some(Mismatch::MalformedGot),
                };
                if expected != got {
                    return Some(Mismatch::AnswerRrsigs(expected, got));
                }
            }
        }
        None
    }
}

fn answertypes_str(types: &BTreeSet<iana::rtype::Rtype>) -> String {
    types
        .iter()
        .map(|x| format!("{}", x))
        .collect::<Vec<String>>()
        .join(" ")
}

fn answerrrsigs_str(types: &BTreeSet<iana::rtype::Rtype>) -> String {
    types
        .iter()
        .map(|x| format!("RRSIG({})", x))
        .collect::<Vec<String>>()
        .join(" ")
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
    QuestionCount,
    Question(Question<Dname<Vec<u8>>>, Question<Dname<Vec<u8>>>),
    AnswerTypes(BTreeSet<iana::rtype::Rtype>, BTreeSet<iana::rtype::Rtype>),
    AnswerRrsigs(BTreeSet<iana::rtype::Rtype>, BTreeSet<iana::rtype::Rtype>),
}

impl Mismatch {
    pub fn expected(&self) -> String {
        match self {
            Mismatch::TimeoutExpected => String::from("timeout"),
            Mismatch::TimeoutGot => String::from("answer"),
            Mismatch::MalformedExpected => String::from("malformed"),
            Mismatch::MalformedGot => String::from("answer"),
            Mismatch::MalformedBoth => String::from("malformed"),
            Mismatch::QuestionCount => String::from("question"),
            Mismatch::AnswerTypes(exp, _) => answertypes_str(exp),
            Mismatch::AnswerRrsigs(exp, _) => answerrrsigs_str(exp),
            Mismatch::Opcode(exp, _) => exp.to_string(),
            Mismatch::Rcode(exp, _) => exp.to_string(),
            Mismatch::Flags(exp, _) => exp.to_string(),
            Mismatch::Question(exp, _) => exp.to_string(),
        }
    }

    pub fn got(&self) -> String {
        match self {
            Mismatch::TimeoutExpected => String::from("answer"),
            Mismatch::TimeoutGot => String::from("timeout"),
            Mismatch::MalformedExpected => String::from("answer"),
            Mismatch::MalformedGot => String::from("malformed"),
            Mismatch::MalformedBoth => String::from("malformed"),
            Mismatch::QuestionCount => String::from("questions"),
            Mismatch::AnswerTypes(_, got) => answertypes_str(got),
            Mismatch::AnswerRrsigs(_, got) => answerrrsigs_str(got),
            Mismatch::Opcode(_, got) => got.to_string(),
            Mismatch::Rcode(_, got) => got.to_string(),
            Mismatch::Flags(_, got) => got.to_string(),
            Mismatch::Question(_, got) => got.to_string(),
        }
    }
}

impl fmt::Display for Mismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} != {}", self.expected(), self.got())
    }
}

pub fn compare(
    expected: &ServerReply,
    got: &ServerReply,
    criteria: &[DiffCriteria],
) -> HashSet<Mismatch> {
    let mut mismatches = HashSet::new();

    match (expected, got) {
        (&ServerReply::Timeout, &ServerReply::Timeout) => {}
        (&ServerReply::Timeout, _) => {
            mismatches.insert(Mismatch::TimeoutExpected);
        }
        (_, &ServerReply::Timeout) => {
            mismatches.insert(Mismatch::TimeoutGot);
        }
        (&ServerReply::Malformed, &ServerReply::Malformed) => {
            mismatches.insert(Mismatch::MalformedBoth);
        }
        (&ServerReply::Malformed, _) => {
            mismatches.insert(Mismatch::MalformedExpected);
        }
        (_, &ServerReply::Malformed) => {
            mismatches.insert(Mismatch::MalformedGot);
        }
        (&ServerReply::Data(ref expected), &ServerReply::Data(ref got)) => {
            for crit in criteria {
                if let Some(mismatch) = crit.mismatch(expected, got) {
                    mismatches.insert(mismatch);
                }
            }
        }
    };

    mismatches
}

pub type FieldMismatches = HashMap<Mismatch, BTreeSet<u32>>;

#[cfg(test)]
mod tests {
    use super::*;
    use domain::base::{iana::rtype::Rtype, Message, MessageBuilder};
    use std::str::FromStr;
    use std::time::Duration;

    fn reply_noerror() -> ServerReply {
        reply_from_msg(MessageBuilder::new_vec().into_message())
    }

    fn reply_from_msg(message: Message<Vec<u8>>) -> ServerReply {
        ServerReply::Data(DnsReply {
            delay: Duration::from_micros(0),
            message: message,
        })
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

        let res = compare(
            &ServerReply::Timeout,
            &reply_noerror(),
            &vec![DiffCriteria::Opcode],
        );
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

        let crit = vec![DiffCriteria::Opcode];
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

        let crit = vec![DiffCriteria::Rcode];
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
        let res = compare(r1, r2, &vec![DiffCriteria::Opcode, DiffCriteria::Rcode]);
        assert_eq!(res.len(), 2);
        assert!(res.contains(&Mismatch::Opcode(Query, Status)));
        assert!(res.contains(&Mismatch::Rcode(NoError, ServFail)));
    }

    #[test]
    fn compare_flags() {
        let crit = vec![DiffCriteria::Flags];
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
            Flags::from_str("").unwrap(),
            Flags::from_str("AA").unwrap()
        )));
    }

    #[test]
    fn compare_question() {
        let crit = vec![DiffCriteria::Question];
        let mut msg1 = MessageBuilder::new_vec().question();
        msg1.push(Question::new_in(Dname::root_vec(), Rtype::A))
            .unwrap();
        let r1 = &reply_from_msg(msg1.into_message());
        let res = compare(r1, r1, &crit);
        assert_eq!(res.len(), 0);

        let mut msg2 = MessageBuilder::new_vec().question();
        msg2.push(Question::new_in(Dname::root_vec(), Rtype::Aaaa))
            .unwrap();
        let r2 = &reply_from_msg(msg2.into_message());

        let res = compare(r1, r2, &crit);
        assert_eq!(res.len(), 1);
        assert!(res.contains(&Mismatch::Question(
            Question::new_in(Dname::root_vec(), Rtype::A),
            Question::new_in(Dname::root_vec(), Rtype::Aaaa),
        )));
    }

    #[test]
    fn compare_answertypes() {
        use domain::base::iana::rtype::Rtype;
        use domain::rdata::{Aaaa, A};
        use std::net::Ipv6Addr;

        let crit = vec![DiffCriteria::AnswerTypes];
        let mut msg1 = MessageBuilder::new_vec().answer();
        msg1.push((Dname::root_ref(), 86400, A::from_octets(192, 0, 2, 1)))
            .unwrap();
        let r1 = &reply_from_msg(msg1.into_message());
        let res = compare(r1, r1, &crit);
        assert_eq!(res.len(), 0);

        let mut msg2 = MessageBuilder::new_vec().answer();
        msg2.push((
            Dname::vec_from_str("test.").unwrap(),
            3600,
            A::from_octets(192, 0, 2, 2),
        ))
        .unwrap();
        msg2.push((
            Dname::vec_from_str("test.").unwrap(),
            3600,
            A::from_octets(192, 0, 2, 3),
        ))
        .unwrap();
        let r2 = &reply_from_msg(msg2.into_message());
        let res = compare(r1, r2, &crit);
        assert_eq!(res.len(), 0); // ensure only rtype is compared and repetition doesn't matter

        let mut msg3 = MessageBuilder::new_vec().answer();
        msg3.push((Dname::root_ref(), 86400, Aaaa::new(Ipv6Addr::LOCALHOST)))
            .unwrap();
        let r3 = &reply_from_msg(msg3.into_message());
        let res = compare(r2, r3, &crit);
        assert_eq!(res.len(), 1);
        assert!(res.contains(&Mismatch::AnswerTypes(
            [Rtype::A].iter().cloned().collect(),
            [Rtype::Aaaa].iter().cloned().collect()
        )));
    }

    #[test]
    fn compare_answerrrsigtypes() {
        use domain::base::iana::rtype::Rtype;
        use domain::rdata::{Rrsig, A};

        let crit = vec![DiffCriteria::AnswerRrsigs];
        let mut msg1 = MessageBuilder::new_vec().answer();
        msg1.push((Dname::root_ref(), 86400, A::from_octets(192, 0, 2, 1)))
            .unwrap();
        msg1.push((
            Dname::root_ref(),
            86400,
            Rrsig::new(
                Rtype::Txt,
                domain::base::iana::secalg::SecAlg::EcdsaP384Sha384,
                1,
                1,
                domain::base::serial::Serial::now(),
                domain::base::serial::Serial::now(),
                1,
                Dname::root_ref(),
                &[0],
            ),
        ))
        .unwrap();
        let r1 = &reply_from_msg(msg1.into_message());
        let res = compare(r1, r1, &crit);
        assert_eq!(res.len(), 0);

        let mut msg2 = MessageBuilder::new_vec().answer();
        msg2.push((
            Dname::vec_from_str("test.").unwrap(),
            3600,
            A::from_octets(192, 0, 2, 2),
        ))
        .unwrap();
        let r2 = &reply_from_msg(msg2.into_message());
        let res = compare(r1, r2, &crit);
        assert_eq!(res.len(), 1);
        assert!(res.contains(&Mismatch::AnswerRrsigs(
            [Rtype::Txt].iter().cloned().collect(),
            [].iter().cloned().collect(),
        )));
    }
}
