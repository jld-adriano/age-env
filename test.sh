#!/usr/bin/env sh
set -eo pipefail

rm -rf test-output.generated
mkdir test-output.generated
cd test-output.generated

alias run="cargo run -q -- --config-dir=."
RUN="cargo run -q -- --config-dir=."

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
echo "reencrypt"
run reencrypt test-env-1

echo "----------------"
echo "reencrypt-all"
run reencrypt-all


echo "----------------"
echo "list"
run list | grep test-env-1
run list | grep test-env-2

echo "----------------"
echo "list-keys"
run list-keys test-env-2 | grep -v realval | grep TEST

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
echo "preload"
source <(run show-for-eval test-env-1 -l)
echo $AGE_ENV_PRELOAD_B64 | grep test-env-1
run show test-env-1 | grep newval
unset AGE_ENV_PRELOAD_B64
echo $AGE_ENV_PRELOAD_B64 | grep -v test-env-1


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
run-local-flag list --short | grep test-env-3

echo "----------------"
echo "delete"
run-local-flag delete test-env-3

echo "----------------"
echo "global files"
rm -rf ./local-config-dir
mkdir -p ./local-config-dir
echo $PUBLIC_KEY_2 | run-local-env add-recipient
echo "TEST=globalval" | run-local-env create test-env-4
AGE_ENV_IDENTITIES_FILE=./identities run-local-env show test-env-4 | grep globalval


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
echo "create with --exclude"
echo 'TEST=realval
OTHER=otherval' | run create --exclude OTHER test-env-6
run show test-env-6 | grep realval
run show test-env-6 | grep -v otherval

echo "----------------"
echo "show with --exclude"
echo 'TEST=realval
OTHER=otherval' | run create test-env-7
run show --exclude OTHER test-env-7 | grep realval
run show --exclude OTHER test-env-7 | grep -v otherval

echo "----------------"
echo "show-for-eval with --exclude"
run show-for-eval --exclude OTHER test-env-7 | grep "export TEST=realval"
run show-for-eval --exclude OTHER test-env-7 | grep -v "export OTHER=otherval"

echo "----------------"
echo "run-with-env with --exclude"
run run-with-env --exclude OTHER test-env-7 -- zsh -c 'echo "$TEST"' | grep realval
run run-with-env --exclude OTHER test-env-7 -- zsh -c 'echo "$OTHER"' | grep -v otherval

echo "----------------"
echo "malformed create input should error"
if echo "TEST" | run create malformed-env 2>/dev/null; then
    echo "Error: Malformed input did not cause an error" >&2
    exit 1
else
    echo "Malformed input caused an error. This is as expected"
fi

echo "----------------"
echo "show with --value"
echo 'TEST=realval
OTHER=otherval' | run create test-env-8
run show --value TEST test-env-8 | grep realval | grep -v otherval
run show --value OTHER test-env-8 | grep otherval | grep -v realval

if run show --value NONEXISTENT test-env-8 >/dev/null 2>&1; then
    echo "Error: Key NONEXISTENT did not cause an error"
    exit 1
else
    echo "Key NONEXISTENT not found. This is as expected"
fi

echo "----------------"
echo "passthrough"
echo "TEST=realval
NEW=newval" | run create  test-env-9
TEST=otherval run show --passthrough -o TEST test-env-9 | grep 'TEST=otherval'
TEST=otherval run show --passthrough -v TEST test-env-9 | grep 'otherval' | grep -v newval
TEST=otherval run show-for-eval --passthrough -o TEST test-env-9 | grep 'export TEST=otherval'
# How to test this one
# TEST=otherval run show-for-eval --passthrough -o TEST -o NEW test-env-9 | grep 'export TEST=otherval' | grep 'export NEW=newval'
TEST=otherval run run-with-env test-env-9 -- cargo run -q -- --config-dir=. show --passthrough test-env-9
cargo run -q -- --config-dir=. show test-env-9 | grep '__passthrough_age_env_test_env_9=1'