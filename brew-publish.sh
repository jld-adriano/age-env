#!/usr/bin/env sh

set -eo pipefail

rm -rf dist.generated
mkdir -p dist.generated

cargo build --release
export PATH="$(pwd)/target/release:$PATH"

tar_file="age-env.tar.gz"
echo "Creating $tar_file"

(cd target/release && tar -czf ../../dist.generated/$tar_file age-env)
shasum -a 256 dist.generated/$tar_file > dist.generated/$tar_file.sha256



version=$(age-env --version | cut -d' ' -f2)
age-env run-with-env gh -- gh release create "v$version" --generate-notes dist.generated/$tar_file dist.generated/$tar_file.sha256
