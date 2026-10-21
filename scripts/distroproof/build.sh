#!/bin/sh
# Stage the atfq builds distroproof runs side by side in each VM:
#   dist/distroproof/static/atfq   this checkout, static (musl)
#   dist/distroproof/release/atfq  the published release named by $ATFQ_RELEASE
#                                  (default v0.1.0), as users download it
set -eu
cd "$(dirname "$0")/../.."
release=${ATFQ_RELEASE:-v0.1.0}
rm -rf dist/distroproof
mkdir -p dist/distroproof/static dist/distroproof/release
target=x86_64-unknown-linux-musl
cargo build --release --locked --target "$target"
cp "target/$target/release/atfq" dist/distroproof/static/atfq
file dist/distroproof/static/atfq | grep -Eq 'statically linked|static-pie linked' || {
    echo 'the musl build is not static' >&2; exit 1; }
tmp=$(mktemp -d)
curl -fsSL "https://github.com/devutils-org/atfq/releases/download/$release/atfq-x86_64-linux.tar.gz" \
    | tar xz -C "$tmp"
cp "$tmp/atfq-x86_64-linux/atfq" dist/distroproof/release/atfq
rm -rf "$tmp"
echo "$release" > dist/distroproof/release/VERSION
