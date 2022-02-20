use std::collections::HashSet;
use std::num::NonZeroU32;
use std::{collections::HashMap, str::FromStr};

use anyhow::{bail, Result};
use futures::stream::FuturesUnordered;
use futures::{Future, StreamExt};
use governor::clock::QuantaClock;
use governor::state::direct::StreamRateLimitExt;
use governor::state::InMemoryState;
use governor::state::NotKeyed;
use governor::Quota;
use governor::RateLimiter;
use lazy_static::lazy_static;
use regex::Regex;
use serde::Serialize;
use shortcut_client::apis::configuration as shortcut_cfg;
use shortcut_client::apis::default_api as shortcut_api;
use shortcut_client::models::{Epic, Story};

use crate::types::RepoToCommits;
use crate::types::ShortcutApiKey;

#[derive(Debug)]
pub struct Commits {
    story_commits: HashMap<StoryId, RepoToCommits>,
    unparsed_commits: RepoToCommits,
}

/// not linked to a story.
pub fn parse_commits(commits: RepoToCommits) -> Result<Commits> {
    lazy_static! {
        static ref SHORTCUT_RE: Regex =
            Regex::new(r"^\[(?:sc-|ch)(\d+)\]").expect("Could not compile SHORTCUT_RE");
    };
    let mut story_commits: HashMap<StoryId, RepoToCommits> = HashMap::new();
    let mut unparsed_commits: RepoToCommits = HashMap::new();
    for (repo_name, commits) in commits {
        for commit in commits {
            let maybe_story_id = commit
                .message
                .as_ref()
                .map(|message| {
                    SHORTCUT_RE.captures(&message).map(|captures| {
                        captures
                            .get(1)
                            .expect("Story id should be captured")
                            .as_str()
                    })
                })
                .flatten()
                .map(|story_id| StoryId::from_str(story_id).expect("Should be parsed as number"));
            if let Some(story_id) = maybe_story_id {
                story_commits
                    .entry(story_id)
                    .or_default()
                    .entry(repo_name.clone())
                    .or_default()
                    .push(commit);
            } else {
                unparsed_commits
                    .entry(repo_name.clone())
                    .or_default()
                    .push(commit);
            }
        }
    }
    Ok(Commits {
        story_commits,
        unparsed_commits,
    })
}

pub struct ShortcutClient {
    configuration: shortcut_cfg::Configuration,
    rate_limiter: RateLimiter<NotKeyed, InMemoryState, QuantaClock>,
}

impl ShortcutClient {
    pub fn new(api_key: &ShortcutApiKey) -> Self {
        let mut configuration = shortcut_cfg::Configuration::new();
        configuration.api_key = Some(shortcut_cfg::ApiKey {
            key: api_key.to_string(),
            prefix: None,
        });
        let shortcut_api_limit: std::num::NonZeroU32 =
            NonZeroU32::new(200u32).expect("Should be non-zero");
        let rate_limiter = RateLimiter::direct(Quota::per_minute(shortcut_api_limit));
        Self {
            configuration,
            rate_limiter,
        }
    }

    async fn get_shortcut_data<T: std::fmt::Debug + Unpin, E: std::fmt::Debug + Unpin>(
        &self,
        actions: impl Iterator<Item = impl Future<Output = Result<T, E>>>,
    ) -> Result<Vec<T>> {
        let items = actions
            .collect::<FuturesUnordered<_>>()
            .ratelimit_stream(&self.rate_limiter)
            .collect::<Vec<_>>()
            .await;
        let (items, errors): (Vec<_>, Vec<_>) = items.into_iter().partition(Result::is_ok);
        let items = items.into_iter().map(Result::unwrap).collect::<Vec<_>>();
        let errors = errors
            .into_iter()
            .map(Result::unwrap_err)
            .collect::<Vec<_>>();
        if !errors.is_empty() {
            bail!("Got the following errors: {:?}", errors);
        }
        Ok(items)
    }

    pub async fn get_release(&self, commits: Commits) -> Result<Release> {
        let stories = self.get_stories(&commits).await?;
        let epics = self.get_epics(stories.iter()).await?;
        let Commits {
            unparsed_commits, ..
        } = commits;
        let release = Release {
            stories,
            epics,
            unparsed_commits,
        };
        Ok(release)
    }

    async fn get_stories(&self, commits: &Commits) -> Result<Vec<Story>> {
        let mut stories = self
            .get_shortcut_data(commits.story_commits.keys().map(|story_id| {
                let story_id: &u32 = story_id.as_ref();
                shortcut_api::get_story(&self.configuration, *story_id as i64)
            }))
            .await?;
        stories.sort_by_key(|story| story.id);
        Ok(stories)
    }

    async fn get_epics(&self, stories: impl Iterator<Item = &Story>) -> Result<Vec<Epic>> {
        let epic_ids = stories
            .filter_map(|story| story.epic_id)
            .collect::<HashSet<_>>();
        let mut epics = self
            .get_shortcut_data(
                epic_ids
                    .into_iter()
                    .map(|epic_id| shortcut_api::get_epic(&self.configuration, epic_id)),
            )
            .await?;
        epics.sort_by_key(|epic| epic.id);
        Ok(epics)
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, AsRef, FromStr, Display, Into)]
pub struct StoryId(u32);

#[derive(Debug, Serialize)]
pub struct Release {
    pub stories: Vec<Story>,
    pub epics: Vec<Epic>,
    pub unparsed_commits: RepoToCommits,
}
