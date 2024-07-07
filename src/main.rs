/**
 * @name age-env
 * @description A tool for managing encrypted environments for the age encryption tool
**/
use clap::Parser;
use dotenv_parser;
use std::env;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to env storage directory
    #[arg(short = 'd', long, default_value_t = String::new())]
    config_dir: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser, Debug)]
enum Command {
    Init,
    List,
    Create {
        /// Name of the environment to create
        name: String,
        #[arg(short = 'e', long)]
        from_env_file: Option<String>,
        #[arg(short = 'r', long)]
        recipient: Option<String>,
        #[arg(short = 'R', long)]
        recipients_file: Option<String>,
        #[arg(short = 's', long)]
        from_stdin: bool,
    },
    Delete {
        /// Name of the environment to delete
        name: String,
    },
    DeleteAll,
    RunWithEnv {
        /// Name of the environment to run with
        name: String,
        #[arg(last = true)]
        command: Vec<String>,
    },
}

fn main() {
    let args = Args::parse();

    if !which::which("age").is_ok() {
        panic!("The 'age' command is required but it's not installed or not found in the PATH.");
    }

    let mut dir = args.config_dir;
    if dir == "" {
        dir = env::var("HOME").unwrap() + "/.age-env";
        if !Path::new(&dir).exists() {
            fs::create_dir(&dir).unwrap();
        }
    }
    let dir = Path::new(&dir);

    let envs_dir = dir.join("envs");
    if !envs_dir.exists() {
        fs::create_dir(&envs_dir).unwrap();
    }
    let identities_file = dir.join("identities");
    if !identities_file.exists() && !matches!(args.command, Command::Init) {
        panic!(
            "Identities file {:?} does not exist. Run `age-env init` to create it.",
            identities_file
        );
    }

    match args.command {
        Command::Init => {
            if identities_file.exists() {
                panic!(
                    "Entities file {:?} already exists no need to init",
                    identities_file
                );
            }
            let mut identities_file = File::create(&identities_file).unwrap();

            let mut identities = String::new();
            std::io::stdin().read_to_string(&mut identities).unwrap();
            identities_file.write_all(identities.as_bytes()).unwrap();
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
            from_stdin,
            recipient,
            recipients_file,
        } => {
            if recipient.is_some() && recipients_file.is_some() {
                panic!("Cannot use both --recipient and --recipients-file");
            }

            let file_path = envs_dir.join(name.clone());
            let env_file = from_env_file.map(|file| Path::new(&dir).join(file));

            if from_stdin && env_file.is_some() {
                panic!("Cannot use both --from-stdin and --from-env-file");
            }

            if file_path.exists() {
                panic!("Environment {:?} already exists", file_path);
            }

            let mut age_command = std::process::Command::new("age");

            match (recipient, recipients_file) {
                (Some(recipient), None) => {
                    age_command.arg("-r").arg(&recipient);
                }
                (None, Some(recipients_file)) => {
                    age_command.arg("-R").arg(&recipients_file);
                }
                _ => {
                    panic!(
                        "Either --recipient or --recipients-file must be provided, but not both"
                    );
                }
            }

            age_command.arg("-o").arg(&file_path);

            // Read the environment contents from either mode
            let env_contents = match from_stdin {
                true => {
                    let mut stdin = String::new();
                    std::io::stdin().read_line(&mut stdin).unwrap();
                    stdin
                }
                false => match env_file {
                    Some(file) => {
                        let mut file = File::open(&file).unwrap();
                        let mut contents = String::new();
                        file.read_to_string(&mut contents).unwrap();
                        contents
                    }
                    None => panic!("No environment file or stdin provided. one of --from-env-file or --from-stdin is required"),
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
            child.wait().unwrap();

            println!("Created environment {} in {:?}", name, file_path);
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
            println!("Deleting all environments in {:?}\n", dir);
            let files = fs::read_dir(&dir).unwrap().collect::<Vec<_>>();
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
            child.wait().unwrap();
            let mut contents = Vec::new();
            child.stdout.unwrap().read_to_end(&mut contents).unwrap();
            println!("contents: {:?}", contents);
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
    }
}
