FROM rust:1.70-buster as builder

RUN apt-get update
RUN apt-get install -y jq curl maven

ENV OPENAPI_GENERATOR_VERSION=6.6.0
RUN curl "https://repo1.maven.org/maven2/org/openapitools/openapi-generator-cli/$OPENAPI_GENERATOR_VERSION/openapi-generator-cli-$OPENAPI_GENERATOR_VERSION.jar" -o /usr/local/lib/openapi-generator.jar

WORKDIR /usr/src/shortcut_release_helper
COPY --chmod=700 bin/generate_openapi_client.sh bin/cleanup.sh bin/
COPY --chmod=700 docker/openapi-generator-cli /usr/local/bin/
COPY Cargo.toml Cargo.lock ./
COPY shortcut_release_helper ./shortcut_release_helper/
RUN ./bin/generate_openapi_client.sh
RUN cargo build --release --bin shortcut_release_helper


FROM debian:bullseye-slim
RUN apt update \
    && apt install -y curl jq git \
    && rm -rf /var/lib/apt/lists/*

## GITHUB Actions execution group is 123 while testing, locally it is 1000
##      This is to resolve file ownership issues will running inside a container
RUN groupadd -g 123 schelper \
    && useradd -m -u 1001 -g schelper schelper
USER schelper

WORKDIR /src
RUN git config --global --add safe.directory "*"

CMD ["/script/execute.sh"]

COPY --chmod=700 docker/execute.sh /script/execute.sh
COPY --from=builder /usr/src/shortcut_release_helper/target/release /usr/local/bin/shortcut_release_helper

