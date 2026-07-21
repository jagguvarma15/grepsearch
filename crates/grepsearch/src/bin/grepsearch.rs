//! Development command line interface for exercising the grepsearch library.
//!
//! This binary is a thin wrapper over the library primitives. It exists so a
//! human can poke at the engine by hand; it is not an agent and not part of
//! the library API. Build it with the `cli` feature enabled.

use clap::{Parser, Subcommand};
use grepsearch::{EngineConfig, FileQuery, GrepQuery, SearchEngine};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "grepsearch", about = "Exercise the grepsearch library")]
struct Cli {
    /// Root directory to search from.
    #[arg(long, default_value = ".", global = true)]
    root: PathBuf,

    /// Print results as JSON instead of plain text.
    #[arg(long, global = true)]
    json: bool,

    /// Do not respect gitignore and ignore files.
    #[arg(long, global = true)]
    no_ignore: bool,

    /// Include hidden files and directories.
    #[arg(long, global = true)]
    hidden: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Search file contents for a pattern.
    Grep {
        /// Pattern to search for.
        pattern: String,
        /// Treat the pattern as a fixed string instead of a regex.
        #[arg(short = 'F', long)]
        literal: bool,
        /// Match case-insensitively.
        #[arg(short = 'i', long)]
        ignore_case: bool,
        /// Restrict the search to paths matching these globs.
        #[arg(short = 'g', long = "glob")]
        globs: Vec<String>,
        /// Lines of context before each match.
        #[arg(short = 'B', long, default_value_t = 0)]
        before: usize,
        /// Lines of context after each match.
        #[arg(short = 'A', long, default_value_t = 0)]
        after: usize,
        /// Maximum number of matches to return.
        #[arg(long, default_value_t = grepsearch::DEFAULT_MAX_RESULTS)]
        max_results: usize,
    },
    /// Find files whose paths match a glob.
    Find {
        /// Glob to match, using gitignore style semantics.
        glob: String,
        /// Maximum number of files to return.
        #[arg(long, default_value_t = grepsearch::DEFAULT_MAX_FILES)]
        max_results: usize,
    },
    /// Read an inclusive 1-based line range from a file.
    Read {
        /// File to read, relative to the root.
        file: PathBuf,
        /// First line to read.
        start: usize,
        /// Last line to read.
        end: usize,
    },
    /// List a directory below the root.
    Ls {
        /// Directory to list, relative to the root.
        #[arg(default_value = ".")]
        dir: PathBuf,
        /// Maximum depth to descend.
        #[arg(long, default_value_t = 1)]
        depth: usize,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let engine = SearchEngine::new(EngineConfig {
        root: cli.root.clone(),
        respect_gitignore: !cli.no_ignore,
        include_hidden: cli.hidden,
        follow_symlinks: false,
    })?;

    match cli.command {
        Command::Grep {
            pattern,
            literal,
            ignore_case,
            globs,
            before,
            after,
            max_results,
        } => {
            let result = engine.grep(&GrepQuery {
                pattern,
                literal,
                case_insensitive: ignore_case,
                globs,
                context_before: before,
                context_after: after,
                max_results,
                ..GrepQuery::default()
            })?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                for file in &result.files {
                    for m in &file.matches {
                        for line in &m.before {
                            println!("{}  {}", file.path.display(), line);
                        }
                        println!("{}:{}: {}", file.path.display(), m.line_number, m.line);
                        for line in &m.after {
                            println!("{}  {}", file.path.display(), line);
                        }
                    }
                }
                println!(
                    "{} matches in {} files, truncated: {}",
                    result.total_matches,
                    result.files.len(),
                    result.truncated
                );
            }
        }
        Command::Find { glob, max_results } => {
            let hits = engine.find_files(&FileQuery { glob, max_results })?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&hits)?);
            } else {
                for hit in &hits {
                    println!("{}  {} bytes", hit.path.display(), hit.bytes);
                }
            }
        }
        Command::Read { file, start, end } => {
            let slice = engine.read_lines(&file, start, end)?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&slice)?);
            } else {
                println!("{}", slice.content);
            }
        }
        Command::Ls { dir, depth } => {
            let entries = engine.list_dir(&dir, depth)?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&entries)?);
            } else {
                for entry in &entries {
                    if entry.is_dir {
                        println!("{}/", entry.path.display());
                    } else {
                        println!("{}", entry.path.display());
                    }
                }
            }
        }
    }
    Ok(())
}
