FROM rust:1.83.0 AS builder

# Set the working directory inside the container
WORKDIR /usr/src/medpack

# copy over your manifests
COPY ./rust-toolchain.toml ./

# for installing toolchain
RUN rustup show

# Cache dependencies. First, copy the Cargo.toml and Cargo.lock
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to ensure `cargo build` can succeed for dependencies
RUN mkdir -p src && echo "fn main() {}" > src/main.rs

# Fetch dependencies without building the actual project (this will be cached)

RUN cargo fetch
RUN cargo build --release

# Copy the rest of the source code and build
COPY . .

ARG GIT_COMMIT_TIMESTAMP
ENV GIT_COMMIT_TIMESTAMP=${GIT_COMMIT_TIMESTAMP}

ARG GIT_SHA
ENV GIT_SHA=${GIT_SHA}

RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim

ARG TARGETARCH

# wkhtmltopdf is installed from github as it needs to be `with patched qt`
RUN \
  --mount=type=cache,target=/var/cache/apt \
  apt-get update && apt-get install --no-install-recommends -y ca-certificates wget curl unzip \
  && apt-get install --no-install-recommends -y poppler-utils img2pdf ocrmypdf tesseract-ocr-eng tesseract-ocr-rus \
  && apt-get clean \
  && wget https://github.com/wkhtmltopdf/packaging/releases/download/0.12.6.1-3/wkhtmltox_0.12.6.1-3.bookworm_${TARGETARCH}.deb \
  && apt-get update \
  && apt-get install -y ./wkhtmltox_0.12.6.1-3.bookworm_${TARGETARCH}.deb \
  && rm wkhtmltox_0.12.6.1-3.bookworm_${TARGETARCH}.deb \
  && rm -rf /var/lib/apt/lists/* \
  && CPDF_ARCH=$([ "$TARGETARCH" = "arm64" ] && echo "Linux-ARM-64bit" || echo "Linux-Intel-64bit") \
  && wget https://github.com/coherentgraphics/cpdf-binaries/raw/master/${CPDF_ARCH}/cpdf \
  && chmod +x cpdf \
  && mv cpdf /usr/local/bin/ \
  && curl -fsSL https://deno.land/install.sh | DENO_INSTALL=/usr/local sh -s -- -y

# Copy the compiled binary from the build stage
COPY --from=builder /usr/src/medpack/target/release/medpack /usr/local/bin/medpack

RUN mkdir -p /tmp/deno && chmod 0777 /tmp/deno

ENV DENO_DIR=/tmp/deno

ENTRYPOINT [ "/usr/local/bin/medpack" ]
