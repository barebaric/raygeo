# Test certificates

`analyzer-test-cert.pem` / `analyzer-test-key.pem` is a self-signed
certificate for `localhost` and `127.0.0.1`, used only by the
analyzer listener tests to exercise the real TLS code path.  It is
not used anywhere in production code and carries no authority.
Regenerate with:

    openssl req -x509 -newkey rsa:2048 -nodes \
      -keyout analyzer-test-key.pem -out analyzer-test-cert.pem \
      -days 3650 -subj "/CN=localhost" \
      -addext "subjectAltName=DNS:localhost,IP:127.0.0.1"

`analyzer-test-server-cert.pem` / `analyzer-test-server-key.pem` is a
server certificate signed by `analyzer-test-cert.pem`, for the same
names. It exercises a real CA-verified TLS handshake in the Rust
daemon query; the CA certificate is passed as `tls_ca`. Regenerate
with:

    openssl req -newkey rsa:2048 -nodes \
      -keyout analyzer-test-server-key.pem \
      -out /tmp/analyzer-test-server.csr \
      -subj "/CN=localhost"
    openssl x509 -req -in /tmp/analyzer-test-server.csr \
      -CA analyzer-test-cert.pem -CAkey analyzer-test-key.pem \
      -CAcreateserial -out analyzer-test-server-cert.pem -days 3650 \
      -extfile <(printf "%s\n" \
        "subjectAltName=DNS:localhost,IP:127.0.0.1" \
        "basicConstraints=critical,CA:FALSE" \
        "keyUsage=critical,digitalSignature,keyEncipherment" \
        "extendedKeyUsage=serverAuth")
