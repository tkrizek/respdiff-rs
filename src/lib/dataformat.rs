use crate::{
    matcher::{Field, FieldMismatches},
    QKey,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// JSON datafile report
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct Report {
    pub start_time: u32,
    pub end_time: u32,
    pub total_queries: u64,
    pub total_answers: u64,
    other_disagreements: OtherDisagreements,
    target_disagreements: TargetDisagreements,
    pub summary: Option<()>,
    pub reprodata: Option<()>,
}

impl Report {
    /// Create new Report
    pub fn new() -> Self {
        Default::default()
    }

    /// Return a set of queries on which other servers (besides target) disagree
    /// (upstream_unstable)
    pub fn others_disagree(&self) -> BTreeSet<QKey> {
        self.other_disagreements.queries.clone()
    }

    /// Set a set of queries that others disagree on
    pub fn set_others_disagree(&mut self, queries: &BTreeSet<QKey>) {
        self.other_disagreements.queries = queries.clone();
    }

    // FIXME: no way to retrieve target_disagrees - not needed right now

    /// Return a collection of target mismatches for each field.
    pub fn set_target_disagrees(&mut self, dis: BTreeMap<Field, FieldMismatches>) {
        self.target_disagreements.fields = BTreeMap::new();
        for (field, fmismatches) in dis {
            let mut items: Vec<MismatchQueries> = Vec::new();
            for (mismatch, queries) in fmismatches {
                let mmqueries = MismatchQueries {
                    exp_val: mismatch.expected(),
                    got_val: mismatch.got(),
                    queries: queries.into_iter().collect(),
                };
                items.push(mmqueries);
            }
            self.target_disagreements
                .fields
                .insert(field, FieldDisagreements { mismatches: items });
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
struct OtherDisagreements {
    queries: BTreeSet<QKey>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
struct TargetDisagreements {
    fields: BTreeMap<Field, FieldDisagreements>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
struct FieldDisagreements {
    mismatches: Vec<MismatchQueries>,
}

/// Collection of queries that share the same mismatch.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct MismatchQueries {
    pub exp_val: String,
    pub got_val: String,
    pub queries: Vec<QKey>,
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
                queries: [22, 64, 93].iter().cloned().collect::<BTreeSet<QKey>>(),
            },
            target_disagreements: TargetDisagreements {
                fields: [
                    (
                        Field::Rcode,
                        FieldDisagreements {
                            mismatches: vec![
                                MismatchQueries {
                                    exp_val: "NOERROR".to_string(),
                                    got_val: "SERVFAIL".to_string(),
                                    queries: vec![6, 16],
                                },
                                MismatchQueries {
                                    exp_val: "NXDOMAIN".to_string(),
                                    got_val: "SERVFAIL".to_string(),
                                    queries: vec![43],
                                },
                            ],
                        },
                    ),
                    (
                        Field::Flags,
                        FieldDisagreements {
                            mismatches: vec![MismatchQueries {
                                exp_val: "QR RD RA AD".to_string(),
                                got_val: "QR RD RA".to_string(),
                                queries: vec![32, 33, 46, 85],
                            }],
                        },
                    ),
                ]
                .iter()
                .cloned()
                .collect(),
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
