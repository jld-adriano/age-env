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
echo "TEST=realval" | run create test-env-1

echo "----------------"
echo "create"
echo "TEST=realval" | run create test-env-2

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
echo "show"
run show test-env-1 | grep realval

echo "----------------"
echo "update"
echo "TEST=newval" | run create -y test-env-1
run show test-env-1 | grep newval

echo "----------------"
echo "show-for-eval"
run show-for-eval test-env-1 | grep "export TEST=newval"

echo "----------------"
echo "delete"
run delete test-env-1
run delete test-env-2

echo "local-config-dir"
alias run-local-env="AGE_ENV_CONFIG_DIR=./local-config-dir cargo run -q --"
alias run-local-flag="cargo run -q -- --config-dir=./local-config-dir "

age-keygen > test-key-3.age
cat test-key-3.age | run-local-env add-identity
export PUBLIC_KEY_3=$(cat test-key-3.age | grep "public key" | cut -d ":" -f 2 | tr -d " ")
echo $PUBLIC_KEY_3 | run-local-env add-recipient
echo "----------------"
echo "create"
echo "TEST=localval" | run-local-env create test-env-3

echo "----------------"
echo "list"
run-local-flag list | grep test-env-3

echo "----------------"
echo "create with --only"
echo 'TEST=realval
OTHER=otherval' | run create --only TEST test-env-4
run show test-env-4 | grep realval
run show test-env-4 | grep -v otherval

echo "----------------"
echo "show with --only"
echo 'TEST=realval
OTHER=otherval' | run create test-env-5
run show --only TEST test-env-5 | grep realval
run show --only TEST test-env-5 | grep -v otherval

echo "----------------"
echo "show-for-eval with --only"
run show-for-eval --only TEST test-env-5 | grep "export TEST=realval"
run show-for-eval --only TEST test-env-5 | grep -v "export OTHER=otherval"

echo "----------------"
echo "run-with-env with --only"
run run-with-env --only TEST test-env-5 -- zsh -c 'echo "$TEST"' | grep realval
run run-with-env --only TEST test-env-5 -- zsh -c 'echo "$OTHER"' | grep -v otherval


echo "----------------"
echo "malformed create input should error"
if echo "TEST" | run create malformed-env 2>/dev/null; then
    echo "Error: Malformed input did not cause an error" >&2
    exit 1
else
    echo "Malformed input caused an error as expected"
fi


