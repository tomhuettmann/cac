mod app;
mod git;
mod ui;

use app::App;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "cac", version, about = "Amend your latest commit with co-authors")]
struct Cli {
    /// Git repository directory (default: current directory)
    #[arg(short, long)]
    directory: Option<PathBuf>,

    /// Number of commits to scan per page
    #[arg(short, long, default_value = "100")]
    num_commits: usize,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let repo_path = cli.directory.unwrap_or_else(|| PathBuf::from("."));

    let repo = git::open_repo(&repo_path)?;
    let (contributors, all_scanned) = git::get_contributors(&repo, cli.num_commits)?;
    let (commit_msg, commit_id) = git::get_latest_commit_info(&repo)?;

    if contributors.is_empty() && all_scanned {
        eprintln!("No contributors found in git history.");
        std::process::exit(1);
    }

    let mut app = App::new(contributors, commit_msg.clone(), commit_id, cli.num_commits, all_scanned);

    let selected = ui::run(&mut app, &repo)?;

    if app.should_quit {
        println!("Cancelled. Commit unchanged.");
        return Ok(());
    }

    git::amend_with_coauthors(&repo, &commit_msg, &selected)?;

    if selected.is_empty() {
        println!("✓ Commit amended: co-authors cleared.");
    } else {
        println!("✓ Commit amended with {} co-author(s):", selected.len());
        for author in &selected {
            println!("  Co-authored-by: {} <{}>", author.name, author.email);
        }
    }

    Ok(())
}
