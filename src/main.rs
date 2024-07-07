/**
 * @name age-env
 * @description A tool for managing encrypted environments for the age encryption tool
**/
use clap::Parser;
use dotenv_parser;
use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::io::Read;
use std::io::Write;
use std::path::Path;

use clap::CommandFactory;
use clap_complete::{generate, Shell};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to env storage directory
    #[arg(short = 'd', long, default_value_t = format!("{}/.age-env", env::var("HOME").unwrap()))]
    config_dir: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser, Debug)]
enum Command {
    /// Add a new identity to the global configuration
    AddIdentity,
    /// Add a new recipient to the global configuration
    AddRecipient,
    /// List all environments
    List,
    /// Create a new environment
    Create {
        /// Name of the environment to create
        name: String,
        #[arg(short = 'e', long)]
        from_env_file: Option<String>,
        #[arg(short = 'r', long)]
        recipient: Option<String>,
        #[arg(short = 'R', long)]
        recipients_file: Option<String>,
    },
    /// Delete an environment
    Delete {
        /// Name of the environment to delete
        name: String,
    },
    /// Delete all environments
    DeleteAll,
    /// Reset the installation
    Reset,
    /// Run a command with the environment
    RunWithEnv {
        /// Name of the environment to run with
        name: String,
        #[arg(last = true)]
        command: Vec<String>,
    },
    /// Generate shell completions
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
        fs::create_dir(&dir).unwrap();
    }

    let envs_dir = dir.join("envs");
    if !envs_dir.exists() {
        fs::create_dir(&envs_dir).unwrap();
    }

    let valid_pre_init_commands = vec![Command::AddIdentity, Command::AddRecipient];

    #[allow(unused_variables)]
    let is_pre_init_command = valid_pre_init_commands
        .into_iter()
        .any(|command| matches!(&args.command, command));

    let identities_file = dir.join("identities");
    if !identities_file.exists() && !is_pre_init_command {
        panic!(
            "Identities file {:?} does not exist. Run `age-env add-identity` to create it.",
            identities_file
        );
    }

    let global_recipients_file = dir.join("recipients");
    let global_recipients_file_exists = global_recipients_file.exists();

    match args.command {
        Command::AddIdentity => {
            let mut identities_file = match identities_file.exists() {
                true => File::options().append(true).open(&identities_file).unwrap(),
                false => File::create(&identities_file).unwrap(),
            };

            let mut identities = String::new();
            std::io::stdin().read_to_string(&mut identities).unwrap();
            identities_file.write_all(identities.as_bytes()).unwrap();
        }
        Command::AddRecipient => {
            let mut recipients_file = match global_recipients_file.exists() {
                true => File::options()
                    .append(true)
                    .open(&global_recipients_file)
                    .unwrap(),
                false => File::create(&global_recipients_file).unwrap(),
            };

            let mut recipients = String::new();
            std::io::stdin().read_to_string(&mut recipients).unwrap();
            recipients_file.write_all(recipients.as_bytes()).unwrap();
        }
        Command::List => {
            let files = fs::read_dir(&envs_dir).unwrap();
            for file in files {
                println!("{:?}", file.unwrap().path());
            }
        }
        Command::Create {
            name,
            from_env_file,
            recipient,
            recipients_file,
        } => {
            let file_path = envs_dir.join(name.clone());
            let env_file = from_env_file.map(|file| Path::new(&dir).join(file));

            if file_path.exists() {
                panic!("Environment {:?} already exists", file_path);
            }

            let mut age_command = std::process::Command::new("age");

            if !global_recipients_file_exists && !recipient.is_some() && !recipients_file.is_some()
            {
                panic!(
                    "Either --recipient or --recipients-file must be provided, or the global recipients file must be present"
                );
            }

            if let Some(recipient) = recipient {
                age_command.arg("-r").arg(&recipient);
            }
            if let Some(recipients_file) = recipients_file {
                age_command.arg("-R").arg(&recipients_file);
            }
            if global_recipients_file_exists {
                age_command.arg("-R").arg(&global_recipients_file);
            }

            age_command.arg("-o").arg(&file_path);

            // Read the environment contents from either mode
            let env_contents = match env_file {
                Some(file) => {
                    let mut file = File::open(&file).unwrap();
                    let mut contents = String::new();
                    file.read_to_string(&mut contents).unwrap();
                    contents
                }
                None => {
                    let mut stdin = String::new();
                    std::io::stdin().read_to_string(&mut stdin).unwrap();
                    stdin
                }
            };

            let mut file = File::create(&file_path).unwrap();
            file.write_all(env_contents.as_bytes()).unwrap();

            let mut child = age_command
                .stdin(std::process::Stdio::piped())
                .spawn()
                .unwrap();
            {
                let stdin = child.stdin.as_mut().unwrap();
                stdin.write_all(env_contents.as_bytes()).unwrap();
            }
            let status = child.wait().unwrap();
            if status.success() {
                println!("Created environment {} in {:?}", name, file_path);
            } else {
                panic!("Failed to create environment {} in {:?}", name, file_path);
            }
        }
        Command::Delete { name } => {
            let file = envs_dir.join(name.clone());
            if file.exists() {
                fs::remove_file(&file).unwrap();
                println!("Deleted environment {:?}", file);
            } else {
                println!("Environment {:?} does not exist", file);
            }
        }
        Command::DeleteAll => {
            println!("Deleting all environments in {:?}\n", envs_dir);
            let files = fs::read_dir(&envs_dir).unwrap().collect::<Vec<_>>();
            if files.len() == 0 {
                println!("No environments to delete");
                return;
            }
            println!("List:");
            for file in files.iter() {
                println!("{:?}", file.as_ref().unwrap().path());
            }
            println!(
                "\nAre you sure you want to delete all files in {:?}? (y/n)",
                dir
            );
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            if input.trim().eq_ignore_ascii_case("y") {
                for file in files.iter() {
                    let file = file.as_ref().unwrap().path();
                    if file.is_file() {
                        fs::remove_file(&file).unwrap();
                        println!("Deleted file {:?}", file);
                    }
                }
            } else {
                panic!("Aborted");
            }
            println!("Deleted all environments in {:?}", dir);
        }
        Command::RunWithEnv { name, command } => {
            let file = envs_dir.join(name.clone());
            if !file.exists() {
                panic!("Environment {:?} does not exist", file);
            }
            let file_contents = fs::read(&file).unwrap();
            let mut child = std::process::Command::new("age")
                .arg("-d")
                .arg("--identity")
                .arg(&identities_file)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .spawn()
                .unwrap();
            {
                let stdin = child.stdin.as_mut().unwrap();
                stdin.write_all(&file_contents).unwrap();
            }
            let status = child.wait().unwrap();
            if !status.success() {
                panic!(
                    "Failed to run command with environment {}: {}: stderr: {:?}",
                    name,
                    status,
                    child.stderr.unwrap()
                );
            }
            let mut contents = Vec::new();
            child.stdout.unwrap().read_to_end(&mut contents).unwrap();

            let source = &String::from_utf8(contents).unwrap();
            let parsed_env = dotenv_parser::parse_dotenv(source).unwrap();

            let mut command_process = std::process::Command::new(command[0].clone());

            for (key, value) in parsed_env.iter() {
                command_process.env(key, value);
            }
            command_process.args(&command[1..]);

            let mut child = command_process.spawn().unwrap();
            child.wait().unwrap();
        }
        Command::Reset => {
            fs::remove_dir_all(&dir).unwrap();
        }
        Command::Generate { shell } => {
            let mut cmd = Args::command();
            let bin_name = cmd.get_name().to_string();
            generate(shell, &mut cmd, bin_name, &mut io::stdout());
        }
    }
}
