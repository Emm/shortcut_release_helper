//! An utility to find all Shortcut stories for a future release.
//!
//! This tool, given a list of repository and, for each repository, a **release** branch and a
//! **next** branch, finds all commits only present in the **next** branch. It then attempts to
//! locate [Shortcut](https://shortcut.com/) stories linked to each commit, as well as any epic
//! these stories may belong to. Finally, it produces a Markdown release notes file based on a
//! template.
//!
//! # Usage
//!
//! ```bash
//! $ ./shortcut_release_helper \
//!     --version 3.4.0 \
//!     --name 'Super release' \
//!     --description 'Exciting release' \
//!     notes.md
//! ```
//!
//! # Configuration
//!
//! This tool expects a `config.toml`, in the current working directory, like so:
//!
//! ```toml
//! api_key = "<your_shortcut_api_key>"
//! template_file = "template.md.jinja"
//!
//! [repositories]
//! # Name of the first repository, can be anything
//! dev = { location = "../project1", release_branch = "master", next_branch = "next" }
//! # Same for the second repository
//! legacy = { location = "../project2", release_branch = "master", next_branch = "next" }
//! ```
//!
//! # Debugging
//!
//! You can use `RUST_LOG` to control the amount logged by the utility in the console.

#[macro_use]
extern crate derive_more;

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    time::Instant,
};

use ansi_term::{
    Colour::{Blue, Green, Red},
    Style,
};
use anyhow::Result;
use clap::Parser;
use git::Repository;
use serde::Serialize;
use shortcut::{ReleaseContent, StoryId};
use shortcut_client::models::{Epic, Story};
use tracing::{debug, info};
use types::RepoToCommits;

use crate::types::{RepositoryConfiguration, RepositoryName, UnreleasedCommit};
use crate::{config::AppConfig, shortcut::parse_commits, shortcut::ShortcutClient};

mod config;
mod git;
mod shortcut;
mod template;
mod types;

/// A command-line tool to generate release notes.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Output file for the release notes
    output_file: PathBuf,
    /// Version to release
    #[clap(long)]
    version: Option<String>,
    /// Name of the release
    #[clap(long)]
    name: Option<String>,
    /// Description of the release
    #[clap(long)]
    description: Option<String>,
    /// Id of story to exclude, can be used multiple times
    #[clap(long)]
    exclude_story_id: Vec<StoryId>,
}

#[tracing::instrument(level = "info", skip_all, fields(repo = %repo_name))]
fn find_unreleased_commits(
    repo_name: &RepositoryName,
    repo_config: &RepositoryConfiguration,
) -> Result<Vec<UnreleasedCommit>> {
    info!(
        release_branch = %repo_config.release_branch,
        next_branch = %repo_config.next_branch
    );
    debug!("Initializing repository");
    let repo = {
        let now = Instant::now();
        let repo = Repository::new(repo_config)?;
        debug!(
            "Initialization done in {time}ms",
            time = now.elapsed().as_millis()
        );
        repo
    };
    let commits = {
        let now = Instant::now();
        let commits = repo.find_unreleased_commits()?;
        info!(
            "Found {commit_count} unreleased commits in {time}ms",
            commit_count = commits.len(),
            time = now.elapsed().as_millis()
        );
        commits
    };
    Ok(commits)
}

fn print_summary(release: &ReleaseContent) {
    let header_style = Style::new().bold();
    println!(
        "{}: {}",
        header_style.paint("Total stories"),
        Green.paint(&release.stories.len().to_string())
    );
    println!(
        "\n{}: {}",
        header_style.paint("Total epics"),
        Green.paint(&release.epics.len().to_string())
    );
    for (repo, commits) in &release.unparsed_commits {
        if !commits.is_empty() {
            println!(
                "\n{}{}: {}",
                header_style.paint("Total unparsed commits in "),
                Blue.paint(repo.as_ref()),
                Red.paint(&commits.len().to_string())
            );
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Release<'a> {
    pub name: Option<&'a str>,
    pub version: Option<&'a str>,
    pub description: Option<&'a str>,
    pub stories: Vec<Story>,
    pub epics: Vec<Epic>,
    pub unparsed_commits: RepoToCommits,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    let config = AppConfig::parse(&PathBuf::from("config.toml"))?;
    let template_content = fs::read_to_string(&config.template_file)?;
    let template = template::FileTemplate::new(&template_content)?;
    let repo_names_and_commits = futures::future::try_join_all(
        config.repositories.into_iter().map(|(name, repo_config)| {
            tokio::task::spawn_blocking::<_, Result<_>>(move || {
                let commits = find_unreleased_commits(&name, &repo_config)?;
                Ok((name, commits))
            })
        }),
    )
    .await?;
    let repo_names_and_commits = repo_names_and_commits
        .into_iter()
        .collect::<Result<HashMap<_, _>>>()?;
    let exclude_story_ids = HashSet::from_iter(args.exclude_story_id.iter().copied());
    let parsed_commits = parse_commits(repo_names_and_commits, &exclude_story_ids)?;
    debug!("Got result {:?}", parsed_commits);
    let shortcut_client = ShortcutClient::new(&config.api_key);
    let release_content = shortcut_client.get_release(parsed_commits).await?;
    print_summary(&release_content);
    let release = Release {
        name: args.name.as_deref(),
        version: args.version.as_deref(),
        description: args.description.as_deref(),
        stories: release_content.stories,
        epics: release_content.epics,
        unparsed_commits: release_content.unparsed_commits,
    };
    template.render_to_file(&release, &args.output_file)?;
    Ok(())
}
