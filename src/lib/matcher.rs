use std::collections::HashMap;
use crate::database::answersdb::ServerReply;  // TODO weird location

#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub enum MatchCriteria {
    Timeout,
    Malformed,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Mismatch {
    TimeoutExpected,
    TimeoutGot,
    MalformedExpected,
    MalformedGot,
    MalformedSame,
}

pub fn compare(
    expected: ServerReply,
    got: ServerReply,
    criteria: Vec<MatchCriteria>,
    ) -> HashMap<MatchCriteria, Mismatch>
{
    HashMap::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compare() {
        assert_eq!(compare(ServerReply::Timeout, ServerReply::Timeout, vec![]).len(), 0);
        assert_eq!(compare(ServerReply::Timeout, ServerReply::Timeout, vec![MatchCriteria::Timeout]).len(), 0);

        let res = compare(ServerReply::Malformed, ServerReply::Timeout, vec![MatchCriteria::Timeout]);
        assert_eq!(res.len(), 1);
        assert_eq!(res.get(&MatchCriteria::Timeout), Some(&Mismatch::TimeoutGot));
    }
}
