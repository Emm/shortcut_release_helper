# Shortcut release helper

This is a command-line tool to retrieve the list of stories from the
[Shortcut](https://app.shortcut.com/) issue tracker which will be deployed in
a release.

# Development

## Dependencies

### Utilities

- [Open API generator](https://github.com/OpenAPITools/openapi-generator).
  This will be used to generate the Shortcut client, based on the OpenAPI
  definition available from the [Shortcut API documentation
  site](https://shortcut.com/api/rest/v3). The project's generator script
  expects an `openapi-generator-cli` in the `PATH`.
- [curl](https://curl.se/), to download the Shortcut OpenAPI definition.
- [jq](https://stedolan.github.io/jq/), to patch it.
- [git](https://git-scm.com/), obviously
- [rustc](https://www.rust-lang.org/), at least 1.58.0

### Libraries

- [OpenSSL](https://www.openssl.org/)

## Building

Clone the repository.

Generate the OpenAPI client via `./bin/generate_openapi_client.sh`.

Build the application via `cargo build`
