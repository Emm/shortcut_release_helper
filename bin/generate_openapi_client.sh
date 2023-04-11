#!/bin/bash

set -eu
set -o pipefail

SHORTCUT_OPENAPI_SPEC_URL='https://developer.shortcut.com/api/rest/v3/shortcut.swagger.json'
OUTPUT_DIR="$(dirname "$0")/../shortcut_client"
ROOT_DIR="$(realpath "$(dirname "$0")/..")"

CRATE_NAME="shortcut_client"
CRATE_VERSION="3.0.0"

TEMPFILE=$(mktemp --suffix .json)

# Fix the PullRequestLabel id type, which is incorrect
# Remove the /api/v3/files endpoint, open API generator doesn't seem to handle form data
# Remove epic's projects ids, because id can be null and lead parsing errors (projects seems to be a legacy feature anyway)
curl "$SHORTCUT_OPENAPI_SPEC_URL" | \
    jq '.definitions.PullRequestLabel.properties.id.type = "string" | del(.definitions.PullRequestLabel.properties.id.format) | del(.paths["/api/v3/files"]) | del(.definitions.Epic.properties.project_ids) | del(.definitions.Epic.properties.associated_groups)' \
    > "$TEMPFILE"

if which openapi-generator-cli
then
    echo "Found openapi-generator-cli in PATH: $(which openapi-generator-cli)"
    OPENAPI_GENERATOR_CLI=openapi-generator-cli
elif [ -x "$ROOT_DIR/node_modules/.bin/openapi-generator-cli" ]
then
    echo "Found openapi-generator-cli in $ROOT_DIR/node_modules"
    OPENAPI_GENERATOR_CLI="$ROOT_DIR/node_modules/.bin/openapi-generator-cli"
else
    echo "Warning: did not find openapi-generator-cli in PATH nor in node_modules"
fi

"$OPENAPI_GENERATOR_CLI" generate \
    -i "$TEMPFILE" \
    -g rust \
    -p packageName="$CRATE_NAME" \
    -p packageVersion="$CRATE_VERSION" \
    -o "$OUTPUT_DIR"

rm "$TEMPFILE"

"$(dirname "$0")/cleanup.sh"
