mod collector;

use std::{env, path::{Path, PathBuf}};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("doctor") => doctor(),
        Some("scan") => scan(args.collect()),
        Some("--version") | Some("-V") => println!("usage-subscription {VERSION}"),
        Some("--help") | Some("-h") | None => print_help(),
        Some(command) => {
            eprintln!("Unknown command: {command}\n");
            print_help();
            std::process::exit(2);
        }
    }
}

fn print_help() {
    println!(
        "usage-subscription {VERSION}\n\n\
        A local-first monitor for AI usage and subscriptions.\n\n\
        USAGE:\n    usage-subscription <COMMAND>\n\n\
        COMMANDS:\n    doctor       Check local data sources used by collectors\n    scan         Read Claude Code transcripts and summarize usage\n    help         Show this help\n\n\
        OPTIONS:\n    -h, --help       Show this help\n    -V, --version    Show the version"
    );
}

fn doctor() {
    let home = env::var_os("HOME").map(PathBuf::from);
    println!("usage-subscription doctor");
    println!();

    match home {
        Some(home) => {
            let claude_dir = home.join(".claude");
            let projects_dir = claude_dir.join("projects");
            let sessions_dir = claude_dir.join("sessions");

            report_path("HOME", &home, true);
            report_path("Claude config", &claude_dir, claude_dir.is_dir());
            report_path("Claude projects", &projects_dir, projects_dir.is_dir());
            report_path("Claude sessions", &sessions_dir, sessions_dir.is_dir());
        }
        None => {
            println!("[missing] HOME is not set");
            std::process::exit(1);
        }
    }
}

fn scan(args: Vec<String>) {
    let path = args.first().map(PathBuf::from).or_else(default_projects_path);
    let Some(path) = path else {
        eprintln!("Cannot scan: HOME is not set and no path was provided");
        std::process::exit(1);
    };

    match collector::scan_projects(&path) {
        Ok(summary) => {
            println!("Claude Code usage scan");
            println!("  path: {}", path.display());
            println!("  files: {}", summary.files);
            println!("  malformed lines: {}", summary.malformed_lines);
            println!("  usage events: {}", summary.events.len());
            println!("  input tokens: {}", sum(&summary.events, |event| event.input_tokens));
            println!("  output tokens: {}", sum(&summary.events, |event| event.output_tokens));
            println!("  cache read tokens: {}", sum(&summary.events, |event| event.cache_read_tokens));
            println!("  cache creation tokens: {}", sum(&summary.events, |event| event.cache_creation_tokens));
        }
        Err(error) => {
            eprintln!("Cannot scan {}: {error}", path.display());
            std::process::exit(1);
        }
    }
}

fn default_projects_path() -> Option<PathBuf> {
    env::var_os("HOME").map(|home| PathBuf::from(home).join(".claude/projects"))
}

fn sum(events: &[collector::UsageEvent], value: impl Fn(&collector::UsageEvent) -> u64) -> u64 {
    events.iter().map(value).sum()
}

fn report_path(label: &str, path: &Path, exists: bool) {
    let state = if exists { "ok" } else { "missing" };
    println!("[{state}] {label}: {}", path.display());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_available() {
        assert!(!VERSION.is_empty());
    }
}
