/**
 * @name age-env
 * @description A tool for managing encrypted environments for the age encryption tool
**/
use clap::Parser;
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
    /// Path to a recipients file
    #[arg(short, long)]
    recipients_file: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser, Debug)]
enum Command {
    Create {
        /// Name of the environment to create
        name: String,
        #[arg(short, long)]
        from_env_file: Option<String>,
    },
    Delete {
        /// Name of the environment to delete
        name: String,
    },
    DeleteAll,
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
    match args.command {
        Command::Create {
            name,
            from_env_file,
        } => {
            let file = Path::new(&dir).join(name.clone());
            let env_file = from_env_file.map(|file| Path::new(&dir).join(file));

            if env_file.is_some() {
                let env_file_path = env_file.as_ref().unwrap();
                let env_file_exists = env_file_path.exists();
                if !env_file_exists {
                    println!(
                        "Environment file {} does not exist",
                        env_file_path.display()
                    );
                }
            }

            if !file.exists() {
                File::create(&file).unwrap();
                println!("Created environment {} in {:?}", name, file);
                let mut file = File::open(&file).unwrap();
                if let Some(env_file) = env_file {
                    let mut env_file = File::open(&env_file).unwrap();
                    let mut contents = String::new();
                    env_file.read_to_string(&mut contents).unwrap();
                    file.write_all(contents.as_bytes()).unwrap();
                    println!("Copied environment {:?} to {:?}", env_file, file);
                }
            } else {
                panic!("Environment {:?} already exists", file);
            }
        }
        Command::Delete { name } => {
            let file = Path::new(&dir).join(name.clone());
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
    }
}
