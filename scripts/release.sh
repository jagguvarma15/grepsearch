#!/usr/bin/env bash
# Cuts a release. The version is 0.1.x where x is the total number of
# commits on main at the release commit, so the bump commit created here is
# itself included in the count.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

branch=$(git rev-parse --abbrev-ref HEAD)
if [ "$branch" != "main" ]; then
    echo "error: releases are cut from main, currently on $branch" >&2
    exit 1
fi

if ! git diff-index --quiet HEAD --; then
    echo "error: working tree is not clean" >&2
    exit 1
fi

git pull --ff-only origin main

count=$(git rev-list --count HEAD)
version="0.1.$((count + 1))"
tag="v$version"

if git rev-parse "$tag" >/dev/null 2>&1; then
    echo "error: tag $tag already exists" >&2
    exit 1
fi

# The workspace manifest holds the single shared version line.
sed -i.bak "s/^version = \".*\"/version = \"$version\"/" Cargo.toml
rm Cargo.toml.bak

cargo check --workspace --quiet

git commit -am "Release $version"
git tag "$tag"
git push origin main "$tag"

echo "released $version"
