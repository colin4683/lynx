# file: core-install.sh
# Purpose: Set up PKI (root + 4 intermediates), issue Core server/client certs,
#          write trust bundles, and create an enrollment token.
# Requires: bash, openssl
# Usage: sudo /usr/local/bin/lynx-core-install.sh core.example.com

set -euo pipefail

if [[ $EUID -ne 0 ]]; then
  echo "Run as root."
  exit 1
fi

CORE_HOSTNAME="${1:-}"
if [[ -z "${CORE_HOSTNAME}" ]]; then
  echo "Usage: lynx-core-install.sh <core-hostname>"
  exit 1
fi

PKI_DIR='/home/colin/Work/lynx-view/certs/pki'
CORE_DIR='/home/colin/Work/lynx-view/certs/core'
mkdir -p "${PKI_DIR}/root" "${PKI_DIR}/icas"/{core-server,core-client,agent-server,agent-client} "${CORE_DIR}"
#chmod 700 "${PKI_DIR}" "${PKI_DIR}/root" "${PKI_DIR}/icas" "${CORE_DIR}"

# 1) Root CA
if [[ ! -f "${PKI_DIR}/root/ca.key.pem" ]]; then
  openssl genrsa -out "${PKI_DIR}/root/ca.key.pem" 4096
  chmod 600 "${PKI_DIR}/root/ca.key.pem"
  openssl req -x509 -new -key "${PKI_DIR}/root/ca.key.pem" -sha256 -days 3650 \
    -subj "/CN=Lynx Root CA" \
    -out "${PKI_DIR}/root/ca.cert.pem" \
    -addext "basicConstraints=critical,CA:TRUE,pathlen:1" \
    -addext "keyUsage=critical,keyCertSign,cRLSign" \
    -addext "subjectKeyIdentifier=hash"
fi

make_intermediate() {
  local name="$1"
  local cn="$2"
  local dir="${PKI_DIR}/icas/${name}"
  mkdir -p "${dir}"
  if [[ -f "${dir}/ica.key.pem" ]]; then return; fi

  openssl genrsa -out "${dir}/ica.key.pem" 4096
  chmod 600 "${dir}/ica.key.pem"
  openssl req -new -key "${dir}/ica.key.pem" -subj "/CN=${cn}" -out "${dir}/ica.csr.pem"

  # Sign intermediate with Root (create serial file if missing)
  openssl x509 -req -in "${dir}/ica.csr.pem" \
    -CA "${PKI_DIR}/root/ca.cert.pem" -CAkey "${PKI_DIR}/root/ca.key.pem" \
    -CAserial "${PKI_DIR}/root/ca.srl" -CAcreateserial \
    -days 1825 -sha256 \
    -out "${dir}/ica.cert.pem" \
    -addext "basicConstraints=critical,CA:TRUE,pathlen:0" \
    -addext "keyUsage=critical,keyCertSign,cRLSign" \
    -addext "authorityKeyIdentifier=keyid,issuer" \
    -addext "subjectKeyIdentifier=hash"

  cat "${dir}/ica.cert.pem" "${PKI_DIR}/root/ca.cert.pem" > "${dir}/ica.chain.pem"
}

# 2) Intermediates
make_intermediate "core-server"  "Lynx Core Server CA"
make_intermediate "core-client"  "Lynx Core Client CA"
make_intermediate "agent-server" "Lynx Agent Server CA"
make_intermediate "agent-client" "Lynx Agent Client CA"

issue_server_cert() {
  local ica="$1" ; local cn="$2" ; local outdir="$3"
  mkdir -p "${outdir}"
  openssl genrsa -out "${outdir}/${ica}.key.pem" 2048
  chmod 600 "${outdir}/${ica}.key.pem"
  openssl req -new -key "${outdir}/${ica}.key.pem" -subj "/CN=${cn}" -out "${outdir}/${ica}.csr.pem"

  openssl x509 -req -in "${outdir}/${ica}.csr.pem" \
    -CA "${PKI_DIR}/icas/${ica}/ica.cert.pem" -CAkey "${PKI_DIR}/icas/${ica}/ica.key.pem" \
    -CAserial "${PKI_DIR}/icas/${ica}/ica.srl" -CAcreateserial \
    -days 120 -sha256 \
    -out "${outdir}/${ica}.cert.pem" \
    -addext "subjectAltName=DNS:${cn}" \
    -addext "extendedKeyUsage=serverAuth" \
    -addext "keyUsage=critical,digitalSignature,keyEncipherment"

  cat "${outdir}/${ica}.cert.pem" "${PKI_DIR}/icas/${ica}/ica.cert.pem" "${PKI_DIR}/root/ca.cert.pem" > "${outdir}/${ica}.fullchain.pem"
}

issue_client_cert() {
  local ica="$1" ; local cn="$2" ; local outdir="$3"
  mkdir -p "${outdir}"
  openssl genrsa -out "${outdir}/${ica}.key.pem" 2048
  chmod 600 "${outdir}/${ica}.key.pem"
  openssl req -new -key "${outdir}/${ica}.key.pem" -subj "/CN=${cn}" -out "${outdir}/${ica}.csr.pem"

  openssl x509 -req -in "${outdir}/${ica}.csr.pem" \
    -CA "${PKI_DIR}/icas/${ica}/ica.cert.pem" -CAkey "${PKI_DIR}/icas/${ica}/ica.key.pem" \
    -CAserial "${PKI_DIR}/icas/${ica}/ica.srl" -CAcreateserial \
    -days 120 -sha256 \
    -out "${outdir}/${ica}.cert.pem" \
    -addext "extendedKeyUsage=clientAuth" \
    -addext "keyUsage=critical,digitalSignature,keyEncipherment"

  cat "${outdir}/${ica}.cert.pem" "${PKI_DIR}/icas/${ica}/ica.cert.pem" "${PKI_DIR}/root/ca.cert.pem" > "${outdir}/${ica}.fullchain.pem"
}

# 3) Core gRPC server cert (issued by Core-Server-CA)
issue_server_cert "core-server" "${CORE_HOSTNAME}" "${CORE_DIR}"
# 4) Core WS mTLS client cert (issued by Core-Client-CA)
issue_client_cert "core-client" "lynx-core-client" "${CORE_DIR}"

# 5) Trust bundles
# Agent trusts these for Core:
cat "${PKI_DIR}/icas/core-server/ica.cert.pem" "${PKI_DIR}/root/ca.cert.pem" > "${CORE_DIR}/agent_trust_core_server.bundle.pem"
cat "${PKI_DIR}/icas/core-client/ica.cert.pem" "${PKI_DIR}/root/ca.cert.pem" > "${CORE_DIR}/agent_trust_core_client.bundle.pem"
# Core trusts these for Agents:
cat "${PKI_DIR}/icas/agent-client/ica.cert.pem" "${PKI_DIR}/root/ca.cert.pem" > "${CORE_DIR}/core_trust_agent_client.bundle.pem"
cat "${PKI_DIR}/icas/agent-server/ica.cert.pem" "${PKI_DIR}/root/ca.cert.pem" > "${CORE_DIR}/core_trust_agent_server.bundle.pem"
chmod 640 "${CORE_DIR}"/*.pem

# 6) Enrollment token
if [[ ! -f "${CORE_DIR}/enroll.token" ]]; then
  openssl rand -hex 32 > "${CORE_DIR}/enroll.token"
  chmod 600 "${CORE_DIR}/enroll.token"
fi

echo "Core PKI ready in '${PKI_DIR}'."
echo "Core certs in '${CORE_DIR}'. Enrollment token in '${CORE_DIR}/enroll.token'."
