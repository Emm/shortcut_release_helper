FROM rust:1.70-buster as builder

RUN apt-get update
RUN apt-get install -y jq curl maven

# https://github.com/OpenAPITools/openapi-generator
RUN mkdir -p ~/bin/openapitools \
    && curl https://raw.githubusercontent.com/OpenAPITools/openapi-generator/master/bin/utils/openapi-generator-cli.sh > ~/bin/openapitools/openapi-generator-cli \
    && chmod u+x ~/bin/openapitools/openapi-generator-cli \
    && export OPENAPI_GENERATOR_CLI=~/bin/openapitools/ \
    && export PATH=$PATH:$OPENAPI_GENERATOR_CLI \
    && cp ~/bin/openapitools/openapi-generator-cli /usr/bin/openapi-generator-cli

WORKDIR /usr/src/shortcut_release_helper
COPY --chmod=700 bin/generate_openapi_client.sh bin/cleanup.sh bin/
COPY --chmod=700 docker/openapi-generator-cli /usr/local/bin/

RUN echo "export OPENAPI_GENERATOR_CLI=~/bin/openapitools/" >> ~/.bashrc
RUN echo "export PATH=$PATH:'~/bin/openapitools'" >> ~/.bashrc

RUN export OPENAPI_GENERATOR_CLI="~/bin/openapitools/" \
    && export PATH="$PATH:~/bin/openapitools/" \
    && ./bin/generate_openapi_client.sh

## Done to improve build times, ensures caching of dependencies independent of compilation
COPY Cargo.toml .
RUN mkdir src \
    && echo "// dummy file" > src/lib.rs \
    && cargo build --release || true

COPY shortcut_release_helper shortcut_release_helper
RUN cargo build --release


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

COPY docker/execute.sh /script/execute.sh
COPY --from=builder /usr/src/shortcut_release_helper/target/release /usr/local/bin/shortcut_release_helper

