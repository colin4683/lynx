#!/usr/bin/env bash
# gen-cert.sh - Generate RSA key, CSR, and signed certificate using an existing CA and server.cnf.
# Usage: ./gen-cert.sh <name> [server.cnf] [out_dir]
# Env:  CA_CERT=ca.crt  CA_KEY=ca.key  DAYS=825

set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <name> [server.cnf] [out_dir]" >&2
  exit 1
fi

NAME="$1"
CNF="${2:-server.cnf}"
OUT_DIR="${3:-.}"

CA_CERT="${CA_CERT:-${OUT_DIR}/ca.crt}"
CA_KEY="${CA_KEY:-${OUT_DIR}/ca.key}"
CA_SERIAL="${CA_SERIAL:-${OUT_DIR}/ca.csr}"
DAYS="${DAYS:-825}"

# Resolve paths
mkdir -p "$OUT_DIR"
KEY="$OUT_DIR/$NAME.key"
CSR="$OUT_DIR/$NAME.csr"
CRT="$OUT_DIR/$NAME.crt"

# Checks
command -v openssl >/dev/null 2>&1 || { echo "Error: openssl not found in PATH" >&2; exit 2; }
[[ -f "$CNF" ]] || { echo "Error: config not found: $CNF" >&2; exit 2; }
[[ -f "$CA_CERT" ]] || { echo "Error: CA cert not found: $CA_CERT" >&2; exit 2; }
[[ -f "$CA_KEY" ]] || { echo "Error: CA key not found: $CA_KEY" >&2; exit 2; }
for f in "$KEY" "$CSR" "$CRT"; do
  [[ -e "$f" ]] && { echo "Error: output exists: $f (refusing to overwrite)" >&2; exit 3; }
done

# Pick serial handling
if [[ -f "$CA_SERIAL" ]]; then
  SERIAL_ARGS=(-CAserial "$CA_SERIAL")
else
  SERIAL_ARGS=(-CAcreateserial)
fi

echo "Generating key: $KEY"
umask 077
openssl genrsa -out "$KEY" 4096 >/dev/null

echo "Creating CSR: $CSR (using $CNF)"
openssl req -new -key "$KEY" -out "$CSR" -config "$CNF"

echo "Signing certificate: $CRT (CA=$CA_CERT) with extensions 'req_ext' from $CNF"
openssl x509 -req -in "$CSR" -CA "$CA_CERT" -CAkey "$CA_KEY" "${SERIAL_ARGS[@]}" \
  -out "$CRT" -days "$DAYS" -sha256 -extensions req_ext -extfile "$CNF"

echo "Done."
echo "Key: $KEY"
echo "CSR: $CSR"
echo "Cert: $CRT"