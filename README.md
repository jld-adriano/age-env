# age-env

## Description
`age-env` is a tool for managing encrypted environments for the age encryption tool.

## Usage

### Commands
```
Usage: age-env [OPTIONS] <COMMAND>

Commands:
  add-identity   
  add-recipient  
  list           
  create         
  delete         
  delete-all     
  run-with-env   
  help           Print this message or the help of the given subcommand(s)

Options:
  -d, --config-dir <CONFIG_DIR>  Path to env storage directory [default: ~/.age-env]
```
# Examples

## Managing your personal github token without unencrypted files

```sh
# Generate a new age key and add it as an identity
# If this was for real you would add a password

age-keygen | age-env add-identity

PUBLIC_KEY=$(cat ~/.age-env/identities | grep "public key" | cut -d ":" -f 2 | tr -d " ")

age-env add-recipient $PUBLIC_KEY

# Create a new environment with your github token
echo "GITHUB_TOKEN=<your-github-token>" | age-env create github-token

# Run a command with the github token environment
age-env run-with-env github-token -- gh repo list

```

# Roadmap

- Managing per env identities
- Managing per env recipients