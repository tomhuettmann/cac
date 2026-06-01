#!/bin/bash
set -e

if [ -z "$1" ]; then
    echo "Usage: ./create-release.sh <version>  (e.g. 1.0.0)"
    exit 1
fi

VERSION=$1
TAG="v${VERSION}"

sed -i '' "s/^version = \".*\"/version = \"${VERSION}\"/" Cargo.toml
cargo update --workspace

git add Cargo.toml Cargo.lock
if [ -n "$(git status --porcelain)" ]; then
    git commit -m "bump version to ${VERSION}"
fi
git tag -a "${TAG}" -m "Release ${VERSION}"

git push origin main
git push origin "${TAG}"

echo "✓ Released ${TAG}"
