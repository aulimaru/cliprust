mod config;
mod history;
use clap::Parser;
use clap::Subcommand;
use config::ThumbMode;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "cliprust",
    version = "0.1.0",
    about = "A command line tool to manage clipboard history written in rust"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,

    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    #[arg(short, long, value_name = "FILE")]
    db_path: Option<PathBuf>,

    #[arg(short = 'm', long)]
    max_dedupe_depth: Option<usize>,

    #[arg(short = 'i', long)]
    max_items: Option<usize>,

    #[arg(short = 'p', long)]
    max_preview_width: Option<usize>,

    #[arg(short = 'g', long)]
    generate_thumb: Option<ThumbMode>,

    #[arg(short = 't', long)]
    header: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Store,
    List,
    Decode,
    Delete,
    Clear,
    Last,
    SecondLast,
    Debug,
}

fn main() {
    let args = Cli::parse();
    let mut config_path = config::default_config_path();
    if let Some(ref path) = args.config {
        config_path = path.clone();
    }
    if !config_path.exists() {
        let default_config = config::default_config();
        default_config.to_file(&config_path);
    }
    let mut config = config::Config::from_file(&config_path);
    config.cli_override(&args);
    let mut clipboard_hist = history::ClipboardHistory::from_file(&config.db_dir_path);
    match args.cmd {
        Commands::Store => {
            let mut input = Vec::new();
            std::io::stdin().read_to_end(&mut input).unwrap();
            if input.is_empty() || input == [10] {
                std::process::exit(1);
            }
            clipboard_hist.add_entry(input, &config);
            clipboard_hist.to_file(&config.db_dir_path)
        }
        Commands::List => {
            if let Some(header) = &args.header {
                println!("{}", header);
            }
            clipboard_hist.list_entries(&config);
        }
        Commands::Decode => {
            let mut input = Vec::new();
            std::io::stdin().read_to_end(&mut input).unwrap();
            let input = String::from_utf8(input).unwrap();
            let result = clipboard_hist.get_entry(get_index(input), &config);
            if let Ok(bytes) = result {
                std::io::stdout().write_all(&bytes).unwrap();
            } else {
                std::process::exit(1);
            }
        }
        Commands::Delete => {
            let mut input = Vec::new();
            std::io::stdin().read_to_end(&mut input).unwrap();
            let input = String::from_utf8(input).unwrap();
            let index = get_index(input);
            clipboard_hist.delete_entry(index, &config);
            clipboard_hist.to_file(&config.db_dir_path)
        }
        Commands::Clear => {
            clipboard_hist.clear(&config);
            clipboard_hist.to_file(&config.db_dir_path)
        }
        Commands::Last => {
            if let Some(header) = &args.header {
                println!("{}", header);
            }
            let result = clipboard_hist.last(&config);
            println!("{}", result);
        }
        Commands::SecondLast => {
            if let Some(header) = &args.header {
                println!("{}", header);
            }
            let result = clipboard_hist.second_last(&config);
            println!("{}", result);
        }
        Commands::Debug => {
            println!("{:?}", config);
            println!("{:?}", clipboard_hist);
        }
    }
}

fn get_index(input: String) -> usize {
    let mut input = input.split_whitespace();
    let input = input.next();
    if let Some(input) = input {
        input.parse().unwrap()
    } else {
        std::process::exit(1);
    }
}
