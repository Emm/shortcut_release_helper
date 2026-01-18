//! This module groups git-related operation
//!
//! The `Repository` structures wraps a [`git2::Repository`].
use anyhow::Result;
use git2::{
    Commit as GitCommit, ErrorClass as GitErrorClass, ErrorCode as GitErrorCode, Oid as GitOid,
    Repository as GitRepository,
};
use itertools::Itertools;
use tracing::debug;

use crate::types::{HeadCommit, RepositoryConfiguration, RepositoryReference, UnreleasedCommit};

pub struct Repository<'a> {
    repository: GitRepository,
    release_branch: &'a RepositoryReference,
    next_branch: &'a RepositoryReference,
}

pub struct UnreleasedCommits {
    pub next_head: HeadCommit,
    pub unreleased_commits: Vec<UnreleasedCommit>,
}

impl<'a> Repository<'a> {
    pub fn new(configuration: &'a RepositoryConfiguration) -> Result<Self> {
        let repository = GitRepository::open(configuration.location.as_ref())?;
        Ok(Self {
            repository,
            release_branch: &configuration.release_branch,
            next_branch: &configuration.next_branch,
        })
    }

    /// Return the list of commits present in the next branch but not the release branch, as well
    /// as the head commit of the next branch
    pub fn find_unreleased_commits_and_head(&'a self) -> Result<UnreleasedCommits> {
        let release_head = self.find_commit(self.release_branch)?;
        let next_head = self.find_commit(self.next_branch)?;

        debug!("Next commit {:?}", next_head.id());
        debug!("Finding merge base");
        let merge_base = self
            .repository
            .merge_base(release_head.id(), next_head.id())?;
        debug!("Merge base {commit:?}", commit = merge_base);
        let mut rev_walk = self.repository.revwalk()?;
        let range = format!("{}..{}", merge_base, next_head.id());
        rev_walk.push_range(&range)?;
        let unreleased_commits = rev_walk
            .inspect(|commit_id| debug!(ancestor_id = ?commit_id))
            .map(|commit_id| match commit_id {
                Ok(commit_id) => self.repository.find_commit(commit_id),
                Err(e) => Err(e),
            })
            .filter_map_ok(|commit| {
                if commit.parent_count() < 2 {
                    Some(UnreleasedCommit {
                        id: commit.id(),
                        message: commit.message().map(|msg| msg.to_owned()),
                    })
                } else {
                    None
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(UnreleasedCommits {
            next_head: HeadCommit {
                id: next_head.id(),
                message: next_head.message().map(|msg| msg.to_owned()),
            },
            unreleased_commits,
        })
    }

    fn find_commit(&'a self, branch: &RepositoryReference) -> Result<GitCommit<'a>> {
        let maybe_reference = self
            .repository
            .resolve_reference_from_short_name(branch.as_ref())
            .map_or_else(
                |err| {
                    if err.class() == GitErrorClass::Reference
                        && err.code() == GitErrorCode::NotFound
                    {
                        Ok(None)
                    } else {
                        Err(err)
                    }
                },
                |reference| Ok(Some(reference)),
            )?;
        if let Some(reference) = maybe_reference {
            let commit = reference.peel_to_commit()?;
            Ok(commit)
        } else {
            let oid = GitOid::from_str(branch.as_ref())?;
            Ok(self.repository.find_commit(oid)?)
        }
    }
}
