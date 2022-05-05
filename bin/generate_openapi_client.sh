#!/bin/bash

set -eu
set -o pipefail

SHORTCUT_OPENAPI_SPEC_URL='https://shortcut.com/api/rest/v3/shortcut.swagger.json'
OUTPUT_DIR="$(dirname $0)/../shortcut_client"

CRATE_NAME="shortcut_client"
CRATE_VERSION="3.0.0"

TEMPFILE=$(mktemp --suffix .json)

curl "$SHORTCUT_OPENAPI_SPEC_URL" | \
    jq '.definitions.PullRequestLabel.properties.id.type = "string" | del(.definitions.PullRequestLabel.properties.id.format)' \
    > "$TEMPFILE"

openapi-generator-cli generate \
    -i "$TEMPFILE" \
    -g rust \
    -p packageName="$CRATE_NAME" \
    -p packageVersion="$CRATE_VERSION" \
    -o "$OUTPUT_DIR"

rm "$TEMPFILE"

"$(dirname $0)/cleanup.sh"
