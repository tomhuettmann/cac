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

fn sanitize_contributors(
    contributors: Vec<Contributor>,
    myself: &Option<Contributor>,
) -> Vec<Contributor> {
    // First, deduplicate by email (case-insensitive), keeping first occurrence
    let mut seen = HashSet::new();
    let deduped: Vec<_> = contributors
        .into_iter()
        .filter(|c| seen.insert(c.email.to_lowercase()))
        .collect();

    // Then, filter out the current user
    match myself {
        None => deduped,
        Some(me) => deduped
            .into_iter()
            .filter(|c| c.email.to_lowercase() != me.email.to_lowercase())
            .collect(),
    }
}

pub fn get_contributors(repo: &Repository) -> Result<Vec<Contributor>, git2::Error> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(Sort::TIME)?;

    let myself = get_current_user(repo);
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

        contributors.push(Contributor { name, email });
    }

    // Sanitize: deduplicate by email and filter out current user
    Ok(sanitize_contributors(contributors, &myself))
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

    // Sanitize: deduplicate by email and filter out current user
    let myself = get_current_user(repo);
    sanitize_contributors(contributors, &myself)
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

    #[test]
    fn test_sanitize_contributors_dedup() {
        let contributors = vec![
            Contributor {
                name: "Alice Smith".to_string(),
                email: "alice@example.com".to_string(),
            },
            Contributor {
                name: "Bob Jones".to_string(),
                email: "bob@example.com".to_string(),
            },
            Contributor {
                name: "Alice Duplicate".to_string(),
                email: "alice@example.com".to_string(), // Duplicate, different name
            },
            Contributor {
                name: "Charlie Brown".to_string(),
                email: "charlie@example.com".to_string(),
            },
        ];

        let sanitized = sanitize_contributors(contributors, &None);

        // Should have 3 entries (alice deduped to first, bob, charlie)
        assert_eq!(sanitized.len(), 3);
        assert_eq!(sanitized[0].name, "Alice Smith"); // First occurrence kept
        assert_eq!(sanitized[1].name, "Bob Jones");
        assert_eq!(sanitized[2].name, "Charlie Brown");
    }

    #[test]
    fn test_sanitize_contributors_case_insensitive_dedup() {
        let contributors = vec![
            Contributor {
                name: "Alice".to_string(),
                email: "Alice@Example.COM".to_string(),
            },
            Contributor {
                name: "Alice Again".to_string(),
                email: "alice@example.com".to_string(), // Different case
            },
        ];

        let sanitized = sanitize_contributors(contributors, &None);

        // Should have 1 entry (case-insensitive match)
        assert_eq!(sanitized.len(), 1);
        assert_eq!(sanitized[0].name, "Alice"); // First occurrence kept with original casing
        assert_eq!(sanitized[0].email, "Alice@Example.COM");
    }

    #[test]
    fn test_sanitize_contributors_filters_self() {
        let myself = Some(Contributor {
            name: "Tom".to_string(),
            email: "tom@example.com".to_string(),
        });

        let contributors = vec![
            Contributor {
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
            },
            Contributor {
                name: "Tom".to_string(),
                email: "tom@example.com".to_string(),
            },
            Contributor {
                name: "Bob".to_string(),
                email: "bob@example.com".to_string(),
            },
        ];

        let sanitized = sanitize_contributors(contributors, &myself);

        // Should have 2 entries (tom filtered out)
        assert_eq!(sanitized.len(), 2);
        assert_eq!(sanitized[0].name, "Alice");
        assert_eq!(sanitized[1].name, "Bob");
    }

    #[test]
    fn test_sanitize_contributors_filters_self_case_insensitive() {
        let myself = Some(Contributor {
            name: "Tom".to_string(),
            email: "Tom@Example.COM".to_string(),
        });

        let contributors = vec![
            Contributor {
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
            },
            Contributor {
                name: "Tom".to_string(),
                email: "tom@example.com".to_string(), // Different case
            },
        ];

        let sanitized = sanitize_contributors(contributors, &myself);

        // Should have 1 entry (tom filtered out despite different case)
        assert_eq!(sanitized.len(), 1);
        assert_eq!(sanitized[0].name, "Alice");
    }

    #[test]
    fn test_sanitize_contributors_dedup_and_filter_self() {
        let myself = Some(Contributor {
            name: "Tom".to_string(),
            email: "tom@example.com".to_string(),
        });

        let contributors = vec![
            Contributor {
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
            },
            Contributor {
                name: "Alice Duplicate".to_string(),
                email: "alice@example.com".to_string(), // Duplicate
            },
            Contributor {
                name: "Tom".to_string(),
                email: "tom@example.com".to_string(),
            },
            Contributor {
                name: "Bob".to_string(),
                email: "bob@example.com".to_string(),
            },
            Contributor {
                name: "Tom Again".to_string(),
                email: "tom@example.com".to_string(), // Duplicate self
            },
        ];

        let sanitized = sanitize_contributors(contributors, &myself);

        // Should have 2 entries: alice (first occurrence), bob (toms filtered out)
        assert_eq!(sanitized.len(), 2);
        assert_eq!(sanitized[0].name, "Alice");
        assert_eq!(sanitized[1].name, "Bob");
    }
}


