#!/bin/sh

set -eu

# Cleanup the generated code to ensure it compiles

SHORTCUT_CLIENT_SRC_DIR="$(dirname $0)/../shortcut_client/src"
DEFAULT_API_FILE="$SHORTCUT_CLIENT_SRC_DIR/apis/default_api.rs"

sed -i 's!story-public-id!story_public_id!g' "$DEFAULT_API_FILE"
sed -i 's!task-public-id!task_public_id!g' "$DEFAULT_API_FILE"
sed -i 's!story-link-public-id!story_link_public_id!g' "$DEFAULT_API_FILE"
sed -i 's!comment-public-id!comment_public_id!g' "$DEFAULT_API_FILE"
sed -i 's!project-public-id!project_public_id!g' "$DEFAULT_API_FILE"
sed -i 's!milestone-public-id!milestone_public_id!g' "$DEFAULT_API_FILE"
sed -i 's!linked-file-public-id!linked_file_public_id!g' "$DEFAULT_API_FILE"
sed -i 's!label-public-id!label_public_id!g' "$DEFAULT_API_FILE"
sed -i 's!milestone-public-id!milestone_public_id!g' "$DEFAULT_API_FILE"
sed -i 's!iteration-public-id!iteration_public_id!g' "$DEFAULT_API_FILE"
sed -i 's!group-public-id!group_public_id!g' "$DEFAULT_API_FILE"
sed -i 's!epic-public-id!epic_public_id!g' "$DEFAULT_API_FILE"
sed -i 's!file-public-id!file_public_id!g' "$DEFAULT_API_FILE"
sed -i 's!entity-public-id!entity_public_id!g' "$DEFAULT_API_FILE"
sed -i 's!category-public-id!category_public_id!g' "$DEFAULT_API_FILE"
sed -i 's!workflow-public-id!workflow_public_id!g' "$DEFAULT_API_FILE"
sed -i 's!entity-template-public-id!entity_template_public_id!g' "$DEFAULT_API_FILE"
sed -i 's!repo-public-id!repo_public_id!g' "$DEFAULT_API_FILE"
sed -i 's!member-public-id!member_public_id!g' "$DEFAULT_API_FILE"
sed -i 's!custom-field-public-id!custom_field_public_id!g' "$DEFAULT_API_FILE"
