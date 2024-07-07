#!/usr/bin/env sh

set -eo pipefail

# Bump Cargo.toml version
if git diff --quiet Cargo.toml; then
    current_version=$(grep '^version' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
    new_version=$(echo $current_version | awk -F. -v OFS=. '{$NF += 1 ; print}')
    sed -i -e "s/version = \"$current_version\"/version = \"$new_version\"/" Cargo.toml
fi


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

sha256=$(cat dist.generated/$tar_file.sha256 | awk '{print $1}')
sed -i -e "s|sha256 \".*\"|sha256 \"$sha256\"|g" ./homebrew-age-env/Formula/age-env.rb
sed -i -e "s|version \".*\"|version \"$version\"|g" ./homebrew-age-env/Formula/age-env.rb

cd homebrew-age-env
git add Formula
git commit -m "Bump version to $version"
cd ..
git add homebrew-age-env
git add Cargo.toml Cargo.lock
git commit -m "Bump version to $version"

echo "Should I push both repos? (y/n)"
read -r answer
if [ "$answer" = "y" ]; then
    cd homebrew-age-env
    git push
    cd ..
    git push
fi
