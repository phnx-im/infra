# SPDX-FileCopyrightText: 2023 Phoenix R&D GmbH <hello@phnx.im>
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Using 1.69 for now. 1.70 gives me an E0519 error when building the
# dependencies.
FROM lukemathwalker/cargo-chef:latest-rust-1.69.0 as chef
WORKDIR /app

FROM chef as planner
COPY . .

RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder
COPY --from=planner /app/recipe.json recipe.json

RUN cargo chef cook --release --recipe-path recipe.json

COPY . .

RUN cargo build --release --bin test_runner

FROM debian:bullseye-slim AS runtime

RUN apt-get update -y \
    #    && apt-get install -y --no-install-recommends openssl ca-certificates \
    && apt-get install -y --no-install-recommends sqlite3 \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/test_runner test_runner

RUN mkdir -p .local/share

ENV PHNX_TEST_SCENARIO $test_scenario
ENV TEST_LOG true

ENTRYPOINT ["./test_runner"]
