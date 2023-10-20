#!/bin/bash

set -eu
set -o pipefail

# Cleanup the generated code to ensure it compiles

SHORTCUT_CLIENT_SRC_DIR="$(dirname $0)/../shortcut_client/src"
DEFAULT_API_FILE="$SHORTCUT_CLIENT_SRC_DIR/apis/default_api.rs"
PARAMS_WITH_HYPHENS=(
    'category-public-id'
    'comment-public-id'
    'custom-field-public-id'
    'entity-template-public-id'
    'epic-public-id'
    'group-public-id'
    'iteration-public-id'
    'label-public-id'
    'linked-file-public-id'
    'file-public-id'
    'member-public-id'
    'milestone-public-id'
    'objective-public-id'
    'org-public-id'
    'project-public-id'
    'repo-public-id'
    'story-link-public-id'
    'story-public-id'
    'task-public-id'
    'workflow-public-id'
);
for param_with_hyphen in ${PARAMS_WITH_HYPHENS[@]}; do
    replacement=$(echo "$param_with_hyphen" | sed "s/-/_/g")
    sed -i "s/$param_with_hyphen/$replacement/g" "$DEFAULT_API_FILE"
done
