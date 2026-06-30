use git2::{Oid, Repository, Sort};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Contributor {
    pub name: String,
    pub email: String,
}

impl Contributor {
    pub fn display(&self) -> String {
        format!("{} <{}>", self.name, self.email)
    }
}

pub fn open_repo(path: &Path) -> Result<Repository, git2::Error> {
    Repository::discover(path)
}

pub fn get_contributors(repo: &Repository) -> Result<Vec<Contributor>, git2::Error> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(Sort::TIME)?;

    let myself = get_current_user(repo);
    let mut seen = HashSet::new();
    let mut contributors = Vec::new();

    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let author = commit.author();

        let name = author.name().unwrap_or("").to_string();
        let email = author.email().unwrap_or("").to_string();

        if name.is_empty() || email.is_empty() {
            continue;
        }

        if let Some(ref me) = myself {
            if me.email.to_lowercase() == email.to_lowercase() {
                continue;
            }
        }

        let contributor = Contributor { name, email };
        if seen.insert(contributor.email.to_lowercase()) {
            contributors.push(contributor);
        }
    }

    Ok(contributors)
}

pub fn load_pinned_authors(repo: &Repository) -> Vec<Contributor> {
    let authors_file = match dirs::home_dir() {
        Some(home) => home.join(".config").join("cac").join("authors"),
        None => return Vec::new(), // No home dir found, skip
    };

    // If file doesn't exist, create it with a template
    if !authors_file.exists() {
        if let Err(_) = fs::create_dir_all(authors_file.parent().unwrap_or_else(|| Path::new("."))) {
            return Vec::new(); // Failed to create dir, skip
        }

        // Try to read current user to pre-fill the file
        let template = if let Some(user) = get_current_user(repo) {
            format!(
                "# cac authors — add one author per line in \"Name <email>\" format\n\
                 # Lines starting with # are comments and are ignored\n\
                 # Your own entry below is just a format example — it is filtered out automatically\n\
                 {} <{}>\n",
                user.name, user.email
            )
        } else {
            "# cac authors — add one author per line in \"Name <email>\" format\n\
             # Lines starting with # are comments and are ignored\n"
                .to_string()
        };

        let _ = fs::write(&authors_file, template);
        return Vec::new(); // Return empty on first creation
    }

    // File exists, parse it
    let contributors = match fs::read_to_string(&authors_file) {
        Ok(content) => parse_authors(&content),
        Err(_) => Vec::new(), // Failed to read, skip
    };

    // Filter out the current user (same logic as get_contributors)
    let myself = get_current_user(repo);
    contributors
        .into_iter()
        .filter(|c| {
            myself
                .as_ref()
                .map(|me| me.email.to_lowercase() != c.email.to_lowercase())
                .unwrap_or(true)
        })
        .collect()
}

fn parse_authors(content: &str) -> Vec<Contributor> {
    let mut contributors = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Parse "Name <email>" format
        if let Some(email_start) = trimmed.rfind('<') {
            if let Some(email_end) = trimmed.rfind('>') {
                if email_start < email_end {
                    let name = trimmed[..email_start].trim().to_string();
                    let email = trimmed[email_start + 1..email_end].trim().to_string();

                    if !name.is_empty() && !email.is_empty() {
                        contributors.push(Contributor { name, email });
                    }
                }
            }
        }
        // Malformed lines are silently skipped
    }

    contributors
}

pub fn get_latest_commit_info(repo: &Repository) -> Result<(String, Oid), git2::Error> {
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    let message = commit.message().unwrap_or("").to_string();
    Ok((message, commit.id()))
}

pub fn amend_with_coauthors(
    repo: &Repository,
    original_msg: &str,
    coauthors: &[Contributor],
) -> Result<(), git2::Error> {
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;

    // Strip existing co-author trailers to avoid duplicates
    let base_msg = strip_coauthor_trailers(original_msg);

    // Build new message with co-author trailers
    let mut new_msg = base_msg.trim_end().to_string();
    new_msg.push_str("\n");
    for author in coauthors {
        new_msg.push_str(&format!("\nCo-authored-by: {} <{}>", author.name, author.email));
    }
    new_msg.push('\n');

    let tree = commit.tree()?;
    let committer = repo.signature()?;

    commit.amend(
        Some("HEAD"),
        None,
        Some(&committer),
        None,
        Some(&new_msg),
        Some(&tree),
    )?;

    Ok(())
}

fn strip_coauthor_trailers(msg: &str) -> String {
    msg.lines()
        .filter(|line| {
            !line.trim_start().to_lowercase().starts_with("co-authored-by:")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn get_current_user(repo: &Repository) -> Option<Contributor> {
    let config = repo.config().ok()?;
    let name = config.get_string("user.name").ok()?;
    let email = config.get_string("user.email").ok()?;
    Some(Contributor { name, email })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_coauthor_trailers() {
        let msg = "feat: add feature\n\nCo-authored-by: Alice <alice@example.com>\nCo-authored-by: Bob <bob@example.com>\n";
        let result = strip_coauthor_trailers(msg);
        assert_eq!(result, "feat: add feature\n");
    }

    #[test]
    fn test_strip_coauthor_preserves_body() {
        let msg = "feat: add feature\n\nThis is a body.\n";
        let result = strip_coauthor_trailers(msg);
        assert_eq!(result, "feat: add feature\n\nThis is a body.");
    }

    #[test]
    fn test_contributor_display() {
        let c = Contributor {
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
        };
        assert_eq!(c.display(), "Alice <alice@example.com>");
    }

    #[test]
    fn test_case_insensitive_email_dedup() {
        use std::collections::HashSet;

        // Simulating walking through commits newest-first:
        // First we see Alice@Example.COM, then alice@example.com (older)
        // We should keep the first one (most recent) with its original casing
        let mut seen = HashSet::new();
        let mut contributors = Vec::new();

        let email1 = "Alice@Example.COM".to_string();
        let contrib1 = Contributor {
            name: "Alice Smith".to_string(),
            email: email1.clone(),
        };
        if seen.insert(contrib1.email.to_lowercase()) {
            contributors.push(contrib1);
        }

        let email2 = "alice@example.com".to_string();
        let contrib2 = Contributor {
            name: "Alice Smith".to_string(),
            email: email2,
        };
        if seen.insert(contrib2.email.to_lowercase()) {
            contributors.push(contrib2);
        }

        // Should have exactly one contributor with the first variant's casing
        assert_eq!(contributors.len(), 1);
        assert_eq!(contributors[0].email, "Alice@Example.COM");
    }
}
