use clap::{Arg, ArgAction, Command};
use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{self, BufRead, BufReader, Write},
    path::Path,
    process::{Command as SystemCommand, Stdio},
};

fn main() -> io::Result<()> {
    let matches = Command::new("Encode Queue")
        .version("1.0")
        .author("brianjsummerfield@gmail.com")
        .about("Manages a queue of commands")
        .subcommand_required(true)
        .subcommand(
            Command::new("add")
                .about("Adds a command to the queue")
                .arg(
                    Arg::new("COMMAND")
                        .help("The command to add")
                        .required(true)
                        .action(ArgAction::Set),
                ),
        )
        .subcommand(Command::new("start").about("Starts processing the queue"))
        .subcommand(Command::new("ls").about("Lists all commands in the queue"))
        .subcommand(Command::new("edit").about("Opens the queue in an editor"))
        .get_matches();

    let queue_dir = Path::new(".config/encode_queue");
    let queue_path = queue_dir.join("commands.txt");

    fs::create_dir_all(queue_dir)?;

    match matches.subcommand() {
        Some(("add", sub_matches)) => {
            let command = sub_matches.get_one::<String>("COMMAND").unwrap();
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&queue_path)?;
            writeln!(file, "{}", command)?;
        }
        Some(("start", _)) => {
            process_queue(&queue_path)?;
        }
        Some(("ls", _)) => {
            let contents = fs::read_to_string(&queue_path)?;
            println!("{}", contents);
        }
        Some(("edit", _)) => {
            open_editor(queue_path.to_str().unwrap())?;
        }
        _ => unreachable!(),
    }

    Ok(())
}

fn process_queue(queue_path: &Path) -> io::Result<()> {
    loop {
        let file = File::open(queue_path)?;
        let mut reader = BufReader::new(&file);
        let mut first_line = String::new();
        if reader.read_line(&mut first_line)? == 0 {
            break;
        }

        if !first_line.trim().is_empty() {
            let mut child = SystemCommand::new("fish")
                .arg("-c")
                .arg(first_line.trim())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()?;

            child.wait()?;
        }

        let file = File::open(queue_path)?;
        let lines = BufReader::new(&file).lines();
        let all_lines: Vec<String> = lines.collect::<Result<_, _>>()?;

        if all_lines.len() > 1 {
            let remaining_lines = &all_lines[1..];

            let mut file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(queue_path)?;
            for line in remaining_lines {
                writeln!(file, "{}", line)?;
            }
        } else {
            OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(queue_path)?;
        }
    }

    Ok(())
}

fn open_editor(file_path: &str) -> std::io::Result<()> {
    let editor = env::var("EDITOR").unwrap_or_else(|_| "nvim".to_string());

    SystemCommand::new(editor)
        .arg(file_path)
        .status()
        .expect("Failed to open editor");

    Ok(())
}
