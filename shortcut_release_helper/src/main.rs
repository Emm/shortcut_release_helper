//! An utility to find all Shortcut tickets for a future release.
//!
//! This tool, given a list of repository and, for each repository, a **release** branch and a
//! **next** branch, finds all commits only present in the **next** branch.
//!
//! # Usage
//!
//! ```bash
//! $ ./shortcut_release_helper
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

use std::{collections::HashMap, fs, path::PathBuf, time::Instant};

use anyhow::Result;
use clap::Parser;
use git::Repository;
use tracing::{debug, info};
use tracing_subscriber;

use crate::types::{RepositoryConfiguration, RepositoryName, UnreleasedCommit};
use crate::{config::AppConfig, shortcut::parse_commits, shortcut::ShortcutClient};

mod config;
mod git;
mod shortcut;
mod template;
mod template_utils;
mod types;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    output_file: PathBuf,
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
        let repo = Repository::new(&repo_config)?;
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
                Ok((name.clone(), commits))
            })
        }),
    )
    .await?;
    let repo_names_and_commits = repo_names_and_commits
        .into_iter()
        .collect::<Result<HashMap<_, _>>>()?;
    let parsed_commits = parse_commits(repo_names_and_commits)?;
    debug!("Got result {:?}", parsed_commits);
    let shortcut_client = ShortcutClient::new(&config.api_key);
    let release = shortcut_client.get_release(parsed_commits).await?;
    template.render_to_file(&release, &args.output_file)?;
    Ok(())
}
