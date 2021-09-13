msgdiff integration test
========================

1. obtain LMDB with answers to compare from https://gitlab.nic.cz/knot/respdiff-rs/uploads/dcf869ac5e9a1dbec50f855af72f5262/data.mdb
2. run msgdiff-rs with respdiff.cfg from this dir
3. compare JSON output with JSON in this file (note: arrays should be treated as sets: i.e. use "jd -sets" tool)

queries & answers: https://gitlab.nic.cz/knot/respdiff-rs/-/snippets/1426
jd: https://github.com/josephburnett/jd
