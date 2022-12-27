FROM rust:latest

RUN apt-get update -qq
RUN apt-get install -y -qqq git

RUN rustup component add rustfmt clippy

# msgdiff integration test
RUN wget https://github.com/josephburnett/jd/releases/download/v1.6.1/jd-amd64-linux -O /usr/local/bin/jd
RUN chmod +x /usr/local/bin/jd
RUN mkdir -p /lmdb/msgdiff-json
RUN wget https://gitlab.nic.cz/knot/respdiff-rs/uploads/dcf869ac5e9a1dbec50f855af72f5262/data.mdb -O /lmdb/msgdiff-json/data.mdb
