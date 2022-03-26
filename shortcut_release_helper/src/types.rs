use std::{collections::HashMap, path::PathBuf};

use git2::Oid as GitOid;
use serde::{Deserialize, Serialize, Serializer};
use std::string::ToString;

/// Name of the Shortcut instance
#[derive(Debug, PartialEq, Eq, Hash, Clone, AsRef, Deserialize, Display)]
#[serde(transparent)]
pub struct ShortcutApiKey(String);

/// Name of the repository, must be unique
#[derive(Debug, PartialEq, Eq, Hash, Clone, AsRef, Deserialize, Display, Serialize)]
#[serde(transparent)]
pub struct RepositoryName(String);

/// Configuration of the repository
#[derive(Debug, PartialEq, Eq, Clone, Deserialize)]
pub struct RepositoryConfiguration {
    /// Path to the location of the repository on disk
    pub location: RepositoryLocation,
    /// Branch or commit name which has been released
    pub release_branch: RepositoryReference,
    /// Branch or commit name which has not been released
    pub next_branch: RepositoryReference,
}

/// Newtype for the physical location of the repository
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, AsRef)]
#[serde(transparent)]
pub struct RepositoryLocation(PathBuf);

/// Newtype for a branch or commit name
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, AsRef, Display)]
#[serde(transparent)]
pub struct RepositoryReference(String);

fn serialize_oid<S: Serializer>(oid: &GitOid, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&oid.to_string())
}

/// Commit only present in `next_branch`.
#[derive(Debug, PartialEq, Eq, Clone, Serialize)]
pub struct UnreleasedCommit {
    #[serde(serialize_with = "serialize_oid")]
    pub id: GitOid,
    pub message: Option<String>,
}

/// A repository name -> unreleased commits mapping
pub type RepoToCommits = HashMap<RepositoryName, Vec<UnreleasedCommit>>;

/// A repository name -> unreleased commit mapping
pub type RepoToCommit = HashMap<RepositoryName, UnreleasedCommit>;
