#!/bin/sh

set -eu
set pipefail

SHORTCUT_OPENAPI_SPEC_URL='https://shortcut.com/api/rest/v3/shortcut.swagger.json'
OUTPUT_DIR="$(dirname $0)/../shortcut_client"

CRATE_NAME="shortcut_client"
CRATE_VERSION="3.0.0"

openapi-generator-cli generate \
    -i "$SHORTCUT_OPENAPI_SPEC_URL" \
    -g rust \
    -p packageName="$CRATE_NAME" \
    -p packageVersion="$CRATE_VERSION" \
    -o "$OUTPUT_DIR"

"$(dirname $0)/cleanup.sh"
