#!/bin/bash
# Construit le paquet Arch rust-man : tarball des sources + makepkg.
set -euo pipefail

cd "$(dirname "$0")/.."
PKGDIR=package
TARBALL="$PKGDIR/rust-man-1.0.0.tar.gz"

echo "== tarball des sources =="
tar czf "$TARBALL" \
  --transform "s,^,rust-man-1.0.0/," \
  Cargo.toml Cargo.lock build.rs src content README.md LICENSE

echo "== makepkg =="
cd "$PKGDIR"
makepkg -f "$@"

echo
echo "Paquet construit : $(ls -t rust-man-*.pkg.tar.* | head -1)"
echo "Installation système : sudo pacman -U $(ls -t rust-man-*.pkg.tar.* | head -1)"
