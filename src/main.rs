mod app;
mod git;
mod ui;

use app::App;
use clap::Parser;
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "cac", version, about = "Amend your latest commit with co-authors")]
struct Cli {
    /// Git repository directory (default: current directory)
    #[arg(short, long)]
    directory: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let repo_path = cli.directory.unwrap_or_else(|| PathBuf::from("."));

    let repo = git::open_repo(&repo_path)?;

    // Load pinned authors from config file and git history contributors
    let pinned = git::load_pinned_authors(&repo);
    let git_contributors = git::get_contributors(&repo)?;

    // Deduplicate: remove from git list any email already in pinned list
    let pinned_emails: HashSet<String> = pinned
        .iter()
        .map(|c| c.email.to_lowercase())
        .collect();
    let unique_git: Vec<_> = git_contributors
        .into_iter()
        .filter(|c| !pinned_emails.contains(&c.email.to_lowercase()))
        .collect();

    // Merge: pinned first (in file order), then git history
    let contributors: Vec<_> = pinned.into_iter().chain(unique_git).collect();

    if contributors.is_empty() {
        eprintln!("No contributors found in git history or config file.");
        std::process::exit(1);
    }

    let (commit_msg, commit_id) = git::get_latest_commit_info(&repo)?;

    let mut app = App::new(contributors, commit_msg.clone(), commit_id);

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
