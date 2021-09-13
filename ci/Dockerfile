FROM rust:latest

RUN apt-get update -qq
RUN apt-get install -y -qqq git golang curl

RUN rustup component add rustfmt clippy

# msgdiff integration test
RUN go get github.com/josephburnett/jd
RUN mkdir -p /lmdb/msgdiff-json
RUN curl https://gitlab.nic.cz/knot/respdiff-rs/uploads/dcf869ac5e9a1dbec50f855af72f5262/data.mdb -o /lmdb/msgdiff-json/data.mdb
ENV PATH "$PATH:/root/go/bin"
