#!/usr/bin/env bash
# Build static-musl tarball and Debian package artifacts.
# Place versioned release outputs in dist/ with a SHA-256 checksum.
set -euo pipefail
cd "$(dirname "$0")/.."

TARGET=x86_64-unknown-linux-musl
VERSION=$(cargo pkgid | sed 's/.*[#@]//')
NAME="netmon-$VERSION-x86_64-linux-musl"

if ! rustup target list --installed | grep -qx "$TARGET"; then
  echo "error: missing Rust target; run: rustup target add $TARGET" >&2
  exit 2
fi

if ! cargo deb --version >/dev/null 2>&1; then
  echo "error: cargo-deb not installed; run: cargo install cargo-deb --locked" >&2
  exit 2
fi

cargo build --release --locked --target "$TARGET"
rm -rf "dist/$NAME" "dist/$NAME.tar.gz" "dist/$NAME.tar.gz.sha256"
rm -f dist/netmon_*.deb
mkdir -p "dist/$NAME"

cp "target/$TARGET/release/netmon" README.md LICENSE "dist/$NAME/"
tar -C dist -czf "dist/$NAME.tar.gz" "$NAME"
(cd dist && sha256sum "$NAME.tar.gz" > "$NAME.tar.gz.sha256")
cargo deb --target "$TARGET" --no-build -o dist/

echo "Artifacts:"
ls -1 dist/
