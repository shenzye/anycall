#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
port=${ANYCALL_WASM_TEST_PORT:-18765}
chromedriver_bin=${CHROMEDRIVER:-$(command -v chromedriver)}
chrome_bin=${CHROME_BIN:-$(command -v chromium || true)}

if [[ -z "${chromedriver_bin}" ]]; then
  echo "chromedriver not found on PATH" >&2
  exit 1
fi

cd "$root"
cargo build -p anycall-test --bin wasm-client-server

server_pid=""
cleanup() {
  if [[ -n "$server_pid" ]]; then
    kill "$server_pid" 2>/dev/null || true
    wait "$server_pid" 2>/dev/null || true
  fi
}
trap cleanup EXIT

ANYCALL_WASM_TEST_PORT="$port" "$root/target/debug/wasm-client-server" &
server_pid=$!

ready=0
for _ in $(seq 1 600); do
  if command -v curl >/dev/null 2>&1; then
    if curl -sf "http://127.0.0.1:${port}/health" >/dev/null; then
      ready=1
      break
    fi
  elif (echo >/dev/tcp/127.0.0.1/"$port") >/dev/null 2>&1; then
    ready=1
    break
  fi
  if ! kill -0 "$server_pid" 2>/dev/null; then
    echo "wasm-client-server exited before becoming ready" >&2
    exit 1
  fi
  sleep 0.1
done

if [[ "$ready" -ne 1 ]]; then
  echo "timed out waiting for wasm-client-server on port ${port}" >&2
  exit 1
fi

if [[ -n "$chrome_bin" ]]; then
  export CHROME_BIN="$chrome_bin"
fi
export WASM_BINDGEN_TEST_TIMEOUT="${WASM_BINDGEN_TEST_TIMEOUT:-90}"

cd "$root/test"
wasm-pack test \
  --headless \
  --chrome \
  --chromedriver "$chromedriver_bin" \
  --mode no-install \
  -- --lib
