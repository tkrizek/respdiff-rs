use serde::{Serialize, Deserialize};
use std::collections::{BTreeSet, BTreeMap};
use crate::matcher::{Field, Mismatch};

// TODO: refactor: make nested types module-private and only used in this module for serde purposes

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub start_time: u64,
    pub end_time: u64,
    pub total_queries: u64,
    pub total_answers: u64,
    pub other_disagreements: OtherDisagreements,
    pub target_disagreements: TargetDisagreements,
    pub summary: Option<()>,
    pub reprodata: Option<()>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct OtherDisagreements {
    pub queries: BTreeSet<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TargetDisagreements {
    pub fields: BTreeMap<Field, FieldDisagreements>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FieldDisagreements {
    pub mismatches: Vec<Disagreement>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Disagreement {
    pub exp_val: String,
    pub got_val: String,
    pub queries: Vec<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const JSON_FORMAT: &'static str = r#"
{
  "start_time": 1628173617,
  "end_time": 1628174644,
  "total_queries": 100,
  "total_answers": 99,
  "other_disagreements": {
    "queries": [ 22, 64, 93 ]
  },
  "target_disagreements": {
    "fields": {
      "rcode": {
        "mismatches": [
          {
            "exp_val": "NOERROR",
            "got_val": "SERVFAIL",
            "queries": [ 6, 16 ]
          },
          {
            "exp_val": "NXDOMAIN",
            "got_val": "SERVFAIL",
            "queries": [ 43 ]
          }
        ]
      },
      "flags": {
        "mismatches": [
          {
            "exp_val": "QR RD RA AD",
            "got_val": "QR RD RA",
            "queries": [ 32, 33, 46, 85 ]
          }
        ]
      }
    }
  },
  "summary": null,
  "reprodata": null
}
"#;

    fn expected() -> Report {
        Report {
            start_time: 1628173617,
            end_time: 1628174644,
            total_queries: 100,
            total_answers: 99,
            other_disagreements: OtherDisagreements {
                queries: [22, 64, 93].iter().cloned().collect::<BTreeSet<u32>>(),
            },
            target_disagreements: TargetDisagreements {
                fields: [
                    (
                        Field::Rcode,
                        FieldDisagreements {
                            mismatches: vec![
                                Disagreement {
                                    exp_val: "NOERROR".to_string(),
                                    got_val: "SERVFAIL".to_string(),
                                    queries: vec![6, 16]
                                },
                                Disagreement {
                                    exp_val: "NXDOMAIN".to_string(),
                                    got_val: "SERVFAIL".to_string(),
                                    queries: vec![43]
                                },
                            ],
                        }
                    ), (
                        Field::Flags,
                        FieldDisagreements {
                            mismatches: vec![
                                Disagreement {
                                    exp_val: "QR RD RA AD".to_string(),
                                    got_val: "QR RD RA".to_string(),
                                    queries: vec![32, 33, 46, 85]
                                },
                            ],
                        }
                    )].iter().cloned().collect(),
            },
            summary: None,
            reprodata: None,
        }
    }

    #[test]
    fn report_serde() {
        let deser = serde_json::from_str::<Report>(JSON_FORMAT).unwrap();
        assert_eq!(expected(), deser);
        let ser = serde_json::to_string(&expected()).unwrap();
        let deser = serde_json::from_str::<Report>(&ser).unwrap();
        assert_eq!(expected(), deser);
    }
}
