#!/usr/bin/env bash
# Regression test for install.sh checksum verification: serves fake releases
# from a local HTTP server and checks that a verified binary installs, while a
# checksum mismatch or a missing SHA256SUMS aborts without installing.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
work="$(mktemp -d)"
trap 'kill "${server_pid:-}" 2>/dev/null || true; rm -rf "$work"' EXIT

targets="x86_64-unknown-linux-musl aarch64-unknown-linux-musl x86_64-apple-darwin aarch64-apple-darwin"
sha() { if command -v sha256sum >/dev/null; then sha256sum "$@"; else shasum -a 256 "$@"; fi; }

mkdir -p "$work/srv/good" "$work/srv/bad" "$work/srv/nosums"
for t in $targets; do
  for dir in good bad nosums; do
    printf '#!/bin/sh\necho "fake oag %s"\n' "$dir" > "$work/srv/$dir/oag-$t"
  done
done
(cd "$work/srv/good" && sha oag-* > SHA256SUMS)
(cd "$work/srv/bad" && sha oag-* | sed 's/^[0-9a-f]\{8\}/00000000/' > SHA256SUMS)

port=8765
(cd "$work/srv" && exec python3 -m http.server "$port" --bind 127.0.0.1 >/dev/null 2>&1) &
server_pid=$!
for _ in $(seq 50); do curl -fs "http://127.0.0.1:$port/" >/dev/null && break; sleep 0.1; done

run() {
  local version="$1" dir="$work/bin-$1"
  shift
  env HOME="$work/home" SHELL=/bin/sh OAG_DOWNLOAD_URL="http://127.0.0.1:$port" \
    OAG_VERSION="$version" OAG_INSTALL_DIR="$dir" "$@" sh "$root/install.sh"
}

mkdir -p "$work/home"
run good >/dev/null
[ "$("$work/bin-good/oag")" = "fake oag good" ] || { echo "FAIL: verified install"; exit 1; }

if run bad >/dev/null 2>&1; then echo "FAIL: checksum mismatch was installed"; exit 1; fi
[ ! -e "$work/bin-bad/oag" ] || { echo "FAIL: mismatching binary left in place"; exit 1; }

if run nosums >/dev/null 2>&1; then echo "FAIL: installed without SHA256SUMS"; exit 1; fi
[ ! -e "$work/bin-nosums/oag" ] || { echo "FAIL: unverified binary left in place"; exit 1; }

run nosums OAG_SKIP_VERIFY=1 >/dev/null 2>&1
[ "$("$work/bin-nosums/oag")" = "fake oag nosums" ] || { echo "FAIL: OAG_SKIP_VERIFY"; exit 1; }

# Plain http:// mirrors are refused (loopback is allowed for this test).
if env HOME="$work/home" OAG_DOWNLOAD_URL="http://example.com" OAG_VERSION=good \
  OAG_INSTALL_DIR="$work/bin-http" sh "$root/install.sh" >/dev/null 2>&1; then
  echo "FAIL: http:// download URL accepted"; exit 1
fi

# The missing-checksum error must not tell users to skip verification.
msg="$(run nosums 2>&1 || true)"
case "$msg" in *OAG_SKIP_VERIFY*) echo "FAIL: error suggests OAG_SKIP_VERIFY"; exit 1 ;; esac

echo "install.sh checksum tests passed"
