#!/usr/bin/env sh
set -eo pipefail

rm -rf test-output.generated
mkdir test-output.generated
cd test-output.generated

alias run="cargo run -q -- --config-dir=."

echo "----------------"
echo "init"

age-keygen > test-key-1.age
cat test-key-1.age | run add-identity
export PUBLIC_KEY_1=$(cat test-key-1.age | grep "public key" | cut -d ":" -f 2 | tr -d " ")
echo $PUBLIC_KEY_1 | run add-recipient

age-keygen > test-key-2.age
cat test-key-2.age | run add-identity
export PUBLIC_KEY_2=$(cat test-key-2.age | grep "public key" | cut -d ":" -f 2 | tr -d " ")

echo "----------------"
echo "create"
echo "TEST=realval" | run create  test-env-1

echo "----------------"
echo "create"
echo "TEST=realval" | run create  test-env-2

echo "----------------"
echo "list"
run list | grep test-env-1
run list | grep test-env-2

echo "----------------"
echo "run-with-env"
run run-with-env test-env-1 -- zsh -c 'echo "$TEST"' | grep realval

echo "----------------"
echo "run-with-env"
run run-with-env test-env-2 -- zsh -c 'echo "$TEST"' | grep realval

echo "----------------"
echo "delete"
run delete test-env-1
run delete test-env-2


