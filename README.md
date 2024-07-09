<!--
SPDX-FileCopyrightText: 2024 Phoenix R&D GmbH <hello@phnx.im>

SPDX-License-Identifier: AGPL-3.0-or-later
-->

# Messaging Layer Security Infrastructure Prototype

This repository contains the code for a Rust implementation of the Phoenix
Protocol, an infrastructure protocol built around the Messaging Layer Security
(MLS) group messaging protocol.

The Phoenix Protocol aims to enable functionality commonly required by messaging
applications while providing strong security and privacy guarantees. It allows
federation between different servers and their connected clients.

For security, the protocol relies mainly on the strong security guarantees
provided by the underlying MLS protocol. The authentication service required by
the MLS protocol is a simple signature-based PKI.

The documentation including the full specification for the Phoenix Protocol can
be found [here](https://docs.phnx.im).

## Code structure

The implementation spans client, server and test-specific components. It is
split across multiple crates:

- `types`: Structs, Enums and utilities used by multiple other crates in this
  repository. This crate allows us to compile client and server components
  separately.
- `backend`: Implements both the local and the federation part of the protocol
  logic on the server side. Inspired by the type-based verificaiton design of
  the OpenMLS crate (on which both `coreclient` and `backend` are built),
  verification of incoming messages is enforced using Rust's type system. The
  `backend` is written to work with arbitrary storage providers. The crate
  itself provides an in-memory and a Postgres-based storage provider.
- `server`: The server component that makes the logic implemented in the
  `backend` available to clients a REST API. Beside the REST API, the `server`
  also supports web-sockets, which the backend can use to notify clients of new
  messages. The `server` crate can be compiled into a binary that can be run to
  expose the HTTP endpoints. A `Dockerfile` is available to build a Docker image
  that contains the server binary.
- `coreclient`: Implements the protocol logic of the client component. The
  `coreclient` stores and manages a user's contacts, conversations, as well as
  the underlying MLS groups. It provides a high-level API to make use of the
  protocol in the context of a messaging application. Just like the `backend`,
  the `coreclient` uses a type-based message verification approach. The crate
  can be used to instantiate and persist (via Sqlite) multiple clients
  parallely.
- `apiclient`: A shallow layer that the `coreclient` calls to interact with the
  `backend` via the `server` using HTTP(s).
- `applogic`: A thin layer for initial testing of higher-level messaging
  concepts.
- `test_harness`: Exclusively used for testing. The `test_harness` contains a
  test framework to conduct integration tests and can be compiled into a binary
  for [docker-based testing](#docker-based-federation-testing) of the protocol's
  federation capabilities.

## Docker-based federation testing

The Phoenix Protocol allows for communication between users across different
servers. To properly test federation-related functionalities, the `test_harness`
provides utilities to spin up multiple servers using Docker. Docker allows us to
approximate a real-world networking environments, where servers can discover
one-another using DNS and facilitate communication between their respective
clients across a network. The `server` crate contains several federated test
scenarios that are run in this way.

Since the tests build fresh Docker images from the code and spin up multiple
containers, running them is somewhat slow. To make regular testing more
ergonomic, the `--include-ignored` flag has to be used when running `cargo test`
on the `server` crate. Note that Docker-based tests are run as part of the CI
whenever a pull request is made.

## Licensing

All crates in this repository are licensed under the [AGPL 3.0](https://www.gnu.org/licenses/agpl-3.0.html). This README file is licensed under [CC-BY 4.0](https://creativecommons.org/licenses/by/4.0/).
