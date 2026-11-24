use std::{env, path::PathBuf};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("doctor") => doctor(),
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
        COMMANDS:\n    doctor       Check local data sources used by collectors\n    help         Show this help\n\n\
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

fn report_path(label: &str, path: &std::path::Path, exists: bool) {
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
