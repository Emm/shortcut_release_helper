//! This module groups git-related operation
//!
//! The `Repository` structures wraps a [`git2::Repository`].
use anyhow::Result;
use git2::{
    Commit as GitCommit, ErrorClass as GitErrorClass, ErrorCode as GitErrorCode, Oid as GitOid,
    Repository as GitRepository,
};
use tracing::debug;

use crate::types::{RepositoryConfiguration, RepositoryReference, UnreleasedCommit};

pub struct Repository<'a> {
    repository: GitRepository,
    release_branch: &'a RepositoryReference,
    next_branch: &'a RepositoryReference,
}

impl<'a> Repository<'a> {
    pub fn new(configuration: &'a RepositoryConfiguration) -> Result<Self> {
        let repository = GitRepository::open(&*configuration.location.as_ref())?;
        Ok(Self {
            repository,
            release_branch: &configuration.release_branch,
            next_branch: &configuration.next_branch,
        })
    }

    pub fn find_unreleased_commits(&self) -> Result<Vec<UnreleasedCommit>> {
        let release_commit = self.find_commit_id(&self.release_branch)?;
        let next_commit = self.find_commit_id(&self.next_branch)?;

        debug!("Next commit {:?}", next_commit.id());
        debug!("Finding merge base");
        let merge_base = self
            .repository
            .merge_base(release_commit.id(), next_commit.id())?;
        debug!("Merge base {:?}", commit = merge_base);
        let mut rev_walk = self.repository.revwalk()?;
        let range = format!(
            "{}..{}",
            merge_base.to_string(),
            next_commit.id().to_string()
        );
        rev_walk.push_range(&range)?;
        let commits = rev_walk
            .inspect(|commit_id| debug!(ancestor_id = ?commit_id))
            .map(|commit_id| match commit_id {
                Ok(commit_id) => Ok(UnreleasedCommit {
                    id: commit_id,
                    message: self
                        .repository
                        .find_commit(commit_id)?
                        .message()
                        .map(|msg| msg.to_owned()),
                }),
                Err(e) => Err(e),
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(commits)
    }

    fn find_commit_id(&'a self, branch: &RepositoryReference) -> Result<GitCommit<'a>> {
        let maybe_reference = self
            .repository
            .resolve_reference_from_short_name(&*branch.as_ref())
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
            let oid = GitOid::from_str(&*branch.as_ref())?;
            Ok(self.repository.find_commit(oid)?)
        }
    }
}
