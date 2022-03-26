# Shortcut release helper

This is a command-line tool to retrieve the list of stories from the [Shortcut](https://app.shortcut.com/) issue tracker which will be deployed in a release

# Development

First, install the [Open API generator](https://github.com/OpenAPITools/openapi-generator). This will be used to generate the Shortcut client, based on the OpenAPI definition available from the [Shortcut API documentation site](https://shortcut.com/api/rest/v3). The project's generator script expects an `openapi-generator-cli` in the `PATH`.

Clone the repository.

Generate the client via `./bin/generate_openapi_client.sh`.
