use std::collections::HashSet;
use std::{collections::HashMap, str::FromStr};

use anyhow::{anyhow, Result};
use futures::stream::FuturesUnordered;
use futures::{Future, FutureExt, StreamExt, TryFutureExt};
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

#[derive(Debug)]
pub struct StoryLabelFilter<'a> {
    excluded_labels: HashSet<&'a String>,
    included_labels: HashSet<&'a String>,
}

impl<'a> StoryLabelFilter<'a> {
    pub fn new(excluded_labels: &'a [String], included_labels: &'a [String]) -> Self {
        Self {
            excluded_labels: HashSet::from_iter(excluded_labels.iter()),
            included_labels: HashSet::from_iter(included_labels.iter()),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.excluded_labels.is_empty() && self.included_labels.is_empty()
    }

    pub fn filter(&self, story: &Story) -> bool {
        let mut included_labels_count = 0;
        for label in &story.labels {
            if self.excluded_labels.contains(&label.name) {
                return false;
            }
            if self.included_labels.contains(&label.name) {
                included_labels_count += 1;
            }
        }
        // This assumes that story labels, as returned by the API, are unique
        included_labels_count == self.included_labels.len()
    }
}

/// not linked to a story.
pub fn parse_commits(
    commits: RepoToCommits,
    exclude_story_ids: &HashSet<StoryId>,
) -> Result<Commits> {
    lazy_static! {
        static ref SHORTCUT_RE: Regex = Regex::new(r"(?:(\[|/)sc-|(\[|/)ch|story/)(\d+)")
            .expect("Could not compile SHORTCUT_RE");
    };
    let mut story_commits: HashMap<StoryId, RepoToCommits> = HashMap::new();
    let mut unparsed_commits: RepoToCommits = HashMap::new();
    for (repo_name, commits) in commits {
        for commit in commits {
            let maybe_story_id = commit
                .message
                .as_ref()
                .and_then(|message| {
                    SHORTCUT_RE.captures(message).map(|captures| {
                        captures
                            .get(3)
                            .expect("Story id should be captured")
                            .as_str()
                    })
                })
                .map(|story_id| StoryId::from_str(story_id).expect("Should be parsed as number"));
            if let Some(story_id) = maybe_story_id {
                if !exclude_story_ids.contains(&story_id) {
                    story_commits
                        .entry(story_id)
                        .or_default()
                        .entry(repo_name.clone())
                        .or_default()
                        .push(commit);
                }
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
    pub fn new(api_key: &ShortcutApiKey, shortcut_api_quota: Quota) -> Self {
        let mut configuration = shortcut_cfg::Configuration::new();
        configuration.api_key = Some(shortcut_cfg::ApiKey {
            key: api_key.to_string(),
            prefix: None,
        });
        let rate_limiter = RateLimiter::direct(shortcut_api_quota);
        Self {
            configuration,
            rate_limiter,
        }
    }

    async fn get_shortcut_data<T: std::fmt::Debug + Unpin, E: std::fmt::Debug + Unpin>(
        &self,
        actions: impl Iterator<Item = impl Future<Output = Result<T, E>>>,
    ) -> Result<Vec<T>> {
        let mut items = Vec::new();

        // Rate-limit the input, not the results
        let mut actions_stream =
            futures::stream::iter(actions.map(Box::pin)).ratelimit_stream(&self.rate_limiter);

        let mut futures = FuturesUnordered::new();

        let mut closed = false;

        // Don't poll the input when it's closed, otherwise it may return None infinitely
        // without select! seeing more results.
        while !closed {
            futures::select! {
                fut = actions_stream.next().fuse() => {
                    if let Some(fut) = fut {
                        futures.push(fut);
                    } else {
                        closed = true;
                    }
                }
                out = futures.next() => {
                    if let Some(res) = out {
                        items.push(res.map_err(|err| anyhow!(format!("{:?}", err)))?);
                    }
                }
            }
        }
        // Drain the rest
        while let Some(res) = futures.next().await {
            items.push(res.map_err(|err| anyhow!(format!("{:?}", err)))?);
        }

        Ok(items)
    }

    pub async fn get_release<'a>(
        &self,
        commits: Commits,
        story_label_filter: StoryLabelFilter<'a>,
    ) -> Result<ReleaseContent> {
        let mut stories = self.get_stories(&commits).await?;
        if !story_label_filter.is_empty() {
            stories.retain(|story| story_label_filter.filter(story));
        }
        let epics = self.get_epics(stories.iter()).await?;
        let Commits {
            unparsed_commits, ..
        } = commits;
        let release = ReleaseContent {
            stories,
            epics,
            unparsed_commits,
        };
        Ok(release)
    }

    async fn get_stories(&self, commits: &Commits) -> Result<Vec<Story>> {
        let mut stories: Vec<Story> = self
            .get_shortcut_data(commits.story_commits.keys().map(|story_id| {
                let story_id: &u32 = story_id.as_ref();
                shortcut_api::get_story(&self.configuration, *story_id as i64).map_err(move |err| {
                    anyhow!("Error while retrieving story {}: {:?}", story_id, err)
                })
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
            .get_shortcut_data(epic_ids.into_iter().map(|epic_id| {
                shortcut_api::get_epic(&self.configuration, epic_id).map_err(move |err| {
                    anyhow!("Error while retrieving epic {}: {:?}", epic_id, err)
                })
            }))
            .await?;
        epics.sort_by_key(|epic| epic.id);
        Ok(epics)
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, AsRef, FromStr, Display, Into)]
pub struct StoryId(u32);

#[derive(Debug, Serialize)]
pub struct ReleaseContent {
    pub stories: Vec<Story>,
    pub epics: Vec<Epic>,
    pub unparsed_commits: RepoToCommits,
}
