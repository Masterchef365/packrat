#!/usr/bin/env bash

# From: https://github.com/kixelated/web-transport-rs/blob/main/web-transport-quinn/cert/generate
#
# Generate a self-signed certificate for localhost.
# This is only valid for 10 days so we can use serverCertificateHashes to avoid a CA (bugged).
# https://developer.mozilla.org/en-US/docs/Web/API/WebTransport/WebTransport#servercertificatehashes
mkdir certs &&\
cp localhost.conf certs &&\
pushd certs &&\
openssl ecparam -genkey -name prime256v1 -out localhost.key &&\
openssl req -x509 -sha256 -nodes -days 10 -key localhost.key -out localhost.crt -config localhost.conf -extensions 'v3_req' &&\

# Generate a hex-encoded (easy to parse) SHA-256 hash of the certificate.
openssl x509 -in localhost.crt -outform der | openssl dgst -sha256 -binary | xxd -p -c 256 > localhost.hex
popd

cp ./certs/localhost.crt ./common/src/
cp ./certs/localhost.crt ./server/src/
cp ./certs/localhost.key ./server/src/
