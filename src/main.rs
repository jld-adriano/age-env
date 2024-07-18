/**
 * @name age-env
 * @description A tool for managing encrypted environments for the age encryption tool
**/
use clap::Parser;
use dotenv_parser;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitStatus;

use clap::CommandFactory;
use clap_complete::{generate, Shell};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to env storage directory
    /// Can be overridden by the AGE_ENV_CONFIG_DIR environment variables
    #[arg(short = 'd', long, env = "AGE_ENV_CONFIG_DIR", default_value_t = get_config_dir_path().to_str().unwrap().to_string())]
    config_dir: String,
    #[arg(long, env = "AGE_ENV_IDENTITIES_FILE", default_value_t = get_config_dir_path().join("identities").to_str().unwrap().to_string())]
    global_identities_file: String,
    #[arg(long, env = "AGE_ENV_RECIPIENTS_FILE", default_value_t = get_config_dir_path().join("recipients").to_str().unwrap().to_string())]
    global_recipients_file: String,
    #[command(subcommand)]
    command: Command,
}

fn get_config_dir_path() -> PathBuf {
    if let Ok(config_dir) = env::var("AGE_ENV_CONFIG_DIR") {
        return PathBuf::from(config_dir);
    }

    let mut current_dir = env::current_dir().expect("Failed to get current directory");
    loop {
        let age_env_path = current_dir.join(".age-env");
        if age_env_path.exists() {
            return age_env_path;
        }

        if !current_dir.pop() {
            break;
        }
    }

    PathBuf::from(format!("{}/.age-env", env::var("HOME").unwrap()))
}

#[derive(Parser, Debug)]
enum Command {
    /// Add a new identity to the global configuration
    #[command(alias = "ai")]
    AddIdentity,
    /// Add a new recipient to the global configuration
    #[command(alias = "ar")]
    AddRecipient,
    /// List all environments
    #[command(alias = "l")]
    List {
        /// If active, only show names of environments
        #[arg(short = 's', long)]
        short: bool,
    },
    ListKeys {
        /// Name of the environment to list keys for
        name: String,
    },
    /// Create a new environment
    #[command(alias = "c")]
    Create {
        /// Name of the environment to create
        name: String,
        #[arg(short = 'f', long)]
        from_env_file: Option<String>,
        #[arg(short = 'r', long)]
        recipient: Option<String>,
        #[arg(short = 'R', long)]
        recipients_file: Option<String>,
        #[arg(short = 'y', long)]
        skip_upsert_confirmation: bool,
        #[arg(short = 'o', long)]
        only: Option<Vec<String>>,
        #[arg(short = 'e', long)]
        exclude: Option<Vec<String>>,
    },
    /// Show the contents of an environment
    #[command(alias = "s")]
    Show {
        /// Name of the environment to show
        name: String,
        #[arg(short = 'o', long)]
        only: Option<Vec<String>>,
        #[arg(short = 'e', long)]
        exclude: Option<Vec<String>>,
        #[arg(short = 'v', long)]
        value: Option<String>,
    },
    /// Show the contents of an environment prepared for eval
    #[command(alias = "se")]
    ShowForEval {
        /// Name of the environment to show
        name: String,
        #[arg(short = 'o', long)]
        only: Option<Vec<String>>,
        #[arg(short = 'e', long)]
        exclude: Option<Vec<String>>,
    },
    /// Delete an environment
    #[command(alias = "d")]
    Delete {
        /// Name of the environment to delete
        name: String,
    },
    /// Delete all environments
    #[command(alias = "da")]
    DeleteAll,
    /// Reset the installation
    #[command(alias = "r")]
    Reset,
    /// Reencrypt an environment with a new set of recipients
    #[command(alias = "re")]
    Reencrypt {
        /// Name of the environment to reencrypt
        name: String,
        #[arg(short = 'r', long)]
        recipient: Option<String>,
        #[arg(short = 'R', long)]
        recipients_file: Option<String>,
    },
    /// Reencrypt all environments with a new set of recipients
    #[command(alias = "rea")]
    ReencryptAll {
        #[arg(short = 'r', long)]
        recipient: Option<String>,
        #[arg(short = 'R', long)]
        recipients_file: Option<String>,
    },
    /// Run a command with the environment
    #[command(alias = "rwe")]
    RunWithEnv {
        /// Name of the environment to run with
        name: String,
        #[arg(last = true)]
        command: Vec<String>,
        #[arg(short = 'o', long)]
        only: Option<Vec<String>>,
        #[arg(short = 'e', long)]
        exclude: Option<Vec<String>>,
    },
    /// Generate shell completions
    #[command(alias = "g")]
    Generate {
        /// The shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

fn main() {
    let args = Args::parse();

    if !which::which("age").is_ok() {
        panic!("The 'age' command is required but it's not installed or not found in the PATH.");
    }

    let dir = Path::new(&args.config_dir);
    if !dir.exists() {
        fs::create_dir(&dir).expect("Failed to create config directory");
    }

    let envs_dir = dir.join("envs");
    if !envs_dir.exists() {
        fs::create_dir(&envs_dir).expect("Failed to create envs directory");
    }

    let valid_pre_init_commands = vec![Command::AddIdentity, Command::AddRecipient];

    #[allow(unused_variables)]
    let is_pre_init_command = valid_pre_init_commands
        .into_iter()
        .any(|command| matches!(&args.command, command));

    let identities_file = PathBuf::from(args.global_identities_file);
    if !identities_file.exists() && !is_pre_init_command {
        panic!(
            "Identities file {:?} does not exist. Run `age-env add-identity` to create it.",
            identities_file
        );
    }

    let global_recipients_file_path = dir.join("recipients");
    let global_recipients_file_exists = global_recipients_file_path.exists();
    let global_recipients_file = match global_recipients_file_exists {
        true => Some(global_recipients_file_path.clone()),
        false => None,
    };

    match args.command {
        Command::AddIdentity => {
            let mut identities_file = match identities_file.exists() {
                true => File::options()
                    .append(true)
                    .open(&identities_file)
                    .expect("Failed to open identities file for appending"),
                false => File::create(&identities_file).expect("Failed to create identities file"),
            };

            let mut identities = String::new();
            std::io::stdin()
                .read_to_string(&mut identities)
                .expect("Failed to read identities from stdin");
            identities_file
                .write_all(identities.as_bytes())
                .expect("Failed to write identities to file");
        }
        Command::AddRecipient => {
            let mut recipients_file = match global_recipients_file_path.exists() {
                true => File::options()
                    .append(true)
                    .open(&global_recipients_file_path)
                    .expect("Failed to open recipients file for appending"),
                false => File::create(&global_recipients_file_path)
                    .expect("Failed to create recipients file"),
            };

            let mut recipients = String::new();
            std::io::stdin()
                .read_to_string(&mut recipients)
                .expect("Failed to read recipients from stdin");
            recipients_file
                .write_all(recipients.as_bytes())
                .expect("Failed to write recipients to file");
        }
        Command::List { short } => {
            let files = fs::read_dir(&envs_dir).expect("Failed to read envs directory");
            for file in files {
                let mut file = file
                    .expect("Failed to read file in envs directory")
                    .path()
                    .to_str()
                    .expect("Failed to convert path to string")
                    .to_string();
                if short {
                    file = file.split("/").last().unwrap().to_string();
                }
                println!("{}", file);
            }
        }
        Command::ListKeys { name } => {
            let file = envs_dir.join(name.clone());
            let contents = decrypt_file_contents(&file, &identities_file);
            let contents_str =
                String::from_utf8(contents).expect("Failed to convert contents to string");
            let parsed_env = dotenv_parser::parse_dotenv(&contents_str)
                .expect("Failed to parse dotenv contents");
            for key in parsed_env.keys() {
                println!("{}", key);
            }
        }
        Command::Create {
            name,
            from_env_file,
            recipient,
            recipients_file,
            skip_upsert_confirmation,
            only,
            exclude,
        } => {
            let file_path = envs_dir.join(name.clone());
            let env_file = from_env_file.map(|file| Path::new(&dir).join(file));

            if file_path.exists() && !skip_upsert_confirmation {
                println!(
                    "Environment {:?} already exists. Do you want to overwrite it? (y/n)",
                    file_path
                );
                let mut input = String::new();
                std::io::stdin()
                    .read_line(&mut input)
                    .expect("Failed to read input from stdin");
                if input.trim().eq_ignore_ascii_case("y") {
                    panic!("Aborted");
                }
            }

            if !global_recipients_file_exists && !recipient.is_some() && !recipients_file.is_some()
            {
                panic!(
                    "Either --recipient or --recipients-file must be provided, or the global recipients file must be present"
                );
            }

            // Read the environment contents from either mode
            let env_contents = match env_file {
                Some(file) => {
                    let mut file = File::open(&file).expect("Failed to open env file");
                    let mut contents = String::new();
                    file.read_to_string(&mut contents)
                        .expect("Failed to read env file");
                    contents
                }
                None => {
                    let mut stdin = String::new();
                    std::io::stdin()
                        .read_to_string(&mut stdin)
                        .expect("Failed to read env from stdin");
                    stdin
                }
            };
            let parsed_env = dotenv_parser::parse_dotenv(&env_contents)
                .expect("Failed to parse dotenv contents");

            let filtered_env_contents = apply_only_exclude(parsed_env, only, exclude);
            let filtered_env_contents_string = filtered_env_contents
                .iter()
                .map(|(key, value)| format!("{}={}", key, value))
                .collect::<Vec<String>>()
                .join("\n");

            let status = encrypt_contents_into_file(
                &recipient,
                &recipients_file,
                &global_recipients_file,
                &file_path,
                filtered_env_contents_string,
            );
            if status.success() {
                println!("Created environment {} in {:?}", name, file_path);
            } else {
                panic!("Failed to create environment {} in {:?}", name, file_path);
            }
        }
        Command::Show {
            name,
            only,
            exclude,
            value,
        } => {
            let file = envs_dir.join(name.clone());
            if !file.exists() {
                panic!("Environment {:?} does not exist", file);
            }
            let contents = decrypt_file_contents(&file, &identities_file);
            let parsed_env = dotenv_parser::parse_dotenv(
                &String::from_utf8(contents).expect("Failed to convert bytes to string"),
            )
            .expect("Failed to parse dotenv contents");
            let filtered_env_contents = apply_only_exclude(parsed_env, only, exclude);
            if let Some(key) = value {
                if let Some(val) = filtered_env_contents.get(&key) {
                    println!("{}", val);
                } else {
                    panic!("Key {} not found", key);
                }
            } else {
                for (key, value) in filtered_env_contents.iter() {
                    println!("{}={}", key, value);
                }
            }
        }
        Command::ShowForEval {
            name,
            only,
            exclude,
        } => {
            let file = envs_dir.join(name.clone());
            if !file.exists() {
                panic!("Environment {:?} does not exist", file);
            }
            let contents = decrypt_file_contents(&file, &identities_file);
            let parsed_env = dotenv_parser::parse_dotenv(
                &String::from_utf8(contents).expect("Failed to convert bytes to string"),
            )
            .expect("Failed to parse dotenv contents");
            let filtered_env_contents = apply_only_exclude(parsed_env, only, exclude);
            for (key, value) in filtered_env_contents.iter() {
                println!("export {}={}", key, value);
            }
        }
        Command::Delete { name } => {
            let file = envs_dir.join(name.clone());
            if file.exists() {
                fs::remove_file(&file).expect("Failed to delete environment file");
                println!("Deleted environment {:?}", file);
            } else {
                println!("Environment {:?} does not exist", file);
            }
        }
        Command::DeleteAll => {
            println!("Deleting all environments in {:?}\n", envs_dir);
            let files = fs::read_dir(&envs_dir)
                .expect("Failed to read envs directory")
                .collect::<Vec<_>>();
            if files.len() == 0 {
                println!("No environments to delete");
                return;
            }
            println!("List:");
            for file in files.iter() {
                println!(
                    "{:?}",
                    file.as_ref()
                        .expect("Failed to read file in envs directory")
                        .path()
                );
            }
            println!(
                "\nAre you sure you want to delete all files in {:?}? (y/n)",
                dir
            );
            let mut input = String::new();
            std::io::stdin()
                .read_line(&mut input)
                .expect("Failed to read input from stdin");
            if input.trim().eq_ignore_ascii_case("y") {
                for file in files.iter() {
                    let file = file
                        .as_ref()
                        .expect("Failed to read file in envs directory")
                        .path();
                    if file.is_file() {
                        fs::remove_file(&file).expect("Failed to delete file");
                        println!("Deleted file {:?}", file);
                    }
                }
            } else {
                panic!("Aborted");
            }
            println!("Deleted all environments in {:?}", dir);
        }
        Command::RunWithEnv {
            name,
            command,
            only,
            exclude,
        } => {
            let file = envs_dir.join(name.clone());
            if !file.exists() {
                panic!("Environment {:?} does not exist", file);
            }
            let contents = decrypt_file_contents(&file, &identities_file);

            let source = &String::from_utf8(contents).expect("Failed to convert stdout to string");
            let parsed_env = dotenv_parser::parse_dotenv(source).expect("Failed to parse dotenv");

            let filtered_env = apply_only_exclude(parsed_env, only, exclude);

            let mut command_process = std::process::Command::new(command[0].clone());

            for (key, value) in filtered_env.iter() {
                command_process.env(key, value);
            }
            command_process.args(&command[1..]);

            let mut child = command_process.spawn().expect(&format!(
                "Failed to spawn command process: `{}`",
                command[0]
            ));
            let status = child.wait().expect("Failed to wait for command process");
            std::process::exit(status.code().unwrap_or(1));
        }
        Command::Reset => {
            fs::remove_dir_all(&dir).expect("Failed to remove config directory");
        }
        Command::Reencrypt {
            name,
            recipient,
            recipients_file,
        } => {
            let path = envs_dir.join(name.clone());
            reencrypt(
                path,
                &recipient,
                &recipients_file,
                &identities_file,
                &global_recipients_file,
            );
        }
        Command::ReencryptAll {
            recipient,
            recipients_file,
        } => {
            for file in fs::read_dir(&envs_dir).expect("Failed to read envs directory") {
                let file = file.expect("Failed to read file in envs directory");
                let path = file.path();
                let name = path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .expect("Failed to convert path to string");
                let path = envs_dir.join(name);
                reencrypt(
                    path,
                    &recipient,
                    &recipients_file,
                    &identities_file,
                    &global_recipients_file,
                );
            }
        }
        Command::Generate { shell } => {
            let mut cmd = Args::command();
            let bin_name = cmd.get_name().to_string();
            generate(shell, &mut cmd, bin_name, &mut io::stdout());
        }
    }
}

fn reencrypt(
    path: PathBuf,
    recipient: &Option<String>,
    recipients_file: &Option<String>,
    identities_file: &PathBuf,
    global_recipients_file: &Option<PathBuf>,
) -> ExitStatus {
    let previous_contents = decrypt_file_contents(&path, identities_file);
    let status = encrypt_contents_into_file(
        recipient,
        recipients_file,
        global_recipients_file,
        &path,
        String::from_utf8(previous_contents).unwrap(),
    );
    status
}

fn apply_only_exclude(
    parsed_env: BTreeMap<String, String>,
    only: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
) -> BTreeMap<String, String> {
    let filtered_env_contents = if let Some(only_keys) = only {
        filter_env_contents(parsed_env, only_keys)
    } else {
        parsed_env
    };
    let filtered_env_contents = if let Some(exclude_keys) = exclude {
        exclude_env_contents(filtered_env_contents, exclude_keys)
    } else {
        filtered_env_contents
    };
    filtered_env_contents
}

fn exclude_env_contents(
    env_contents: BTreeMap<String, String>,
    exclude_keys: Vec<String>,
) -> BTreeMap<String, String> {
    env_contents
        .into_iter()
        .filter(|(key, _)| !exclude_keys.contains(key))
        .collect::<BTreeMap<String, String>>()
}
fn filter_env_contents(
    env_contents: BTreeMap<String, String>,
    only_keys: Vec<String>,
) -> BTreeMap<String, String> {
    env_contents
        .into_iter()
        .filter(|(key, _)| only_keys.contains(key))
        .collect::<BTreeMap<String, String>>()
}

fn decrypt_file_contents(
    file: &std::path::PathBuf,
    identities_file: &std::path::PathBuf,
) -> Vec<u8> {
    let file_contents = fs::read(&file).expect("Failed to read environment file");
    let mut child = std::process::Command::new("age")
        .arg("-d")
        .arg("--identity")
        .arg(&identities_file)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn age command");
    {
        let stdin = child
            .stdin
            .as_mut()
            .expect("Failed to open stdin for age command");
        stdin
            .write_all(&file_contents)
            .expect("Failed to write environment contents to age command");
    }
    let status = child.wait().expect("Failed to wait for age command");
    if !status.success() {
        panic!(
            "Failed to run command with status {}: stderr: {:?}",
            status,
            child
                .stderr
                .expect("Failed to read stderr from age command")
        );
    }
    let mut contents = Vec::new();
    child
        .stdout
        .expect("Failed to open stdout for age command")
        .read_to_end(&mut contents)
        .expect("Failed to read stdout from age command");
    contents
}

fn encrypt_contents_into_file(
    recipient: &Option<String>,
    recipients_file: &Option<String>,
    global_recipients_file: &Option<PathBuf>,
    file_path: &PathBuf,
    filtered_env_contents_string: String,
) -> std::process::ExitStatus {
    let mut age_command = std::process::Command::new("age");

    if let Some(recipient) = recipient {
        age_command.arg("-r").arg(&recipient);
    }
    if let Some(recipients_file) = recipients_file {
        age_command.arg("-R").arg(&recipients_file);
    }
    if let Some(global_recipients_file) = global_recipients_file {
        age_command.arg("-R").arg(&global_recipients_file);
    }

    age_command.arg("-o").arg(file_path);

    let mut child = age_command
        .stdin(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn age command");
    {
        let stdin = child
            .stdin
            .as_mut()
            .expect("Failed to open stdin for age command");
        stdin
            .write_all(filtered_env_contents_string.as_bytes())
            .expect("Failed to write environment contents to age command");
    }
    let status = child.wait().expect("Failed to wait for age command");
    status
}
