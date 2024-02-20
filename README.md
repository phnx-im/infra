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
  repository.
- `backend`: Implements the protocol logic of the server component. It is
  written to work with arbitrary storate providers. The crate itself provides an
  in-memory and a Postgres-based storage provider.
- `server`: The frontend component that makes the logic implemented in the
  `backend` available to clients a REST API. It can be compiled into a binary
  that can be run to expose the HTTP endpoints.
- `coreclient`: Implements the protocol logic of the client component. Its
  storage is based on Sqlite.
- `apiclient`: A shallow layer that the `coreclient` calls to interact with the
  `backend` via the `server` using HTTP(s).
- `applogic`: A thin layer for initial testing of higher-level messaging
  concepts.
- `test_harness`: Exclusively used for testing. Contains a test framework to
  conduct integration tests and can be compiled into a binary for docker-based
  testing of the protocol's federation capabilities.

## Docker-based federation testing

The Phoenix Protocol allows for communication between users across different
servers. To properly test federation-related functionalities, the `test_harness`
provides utilities to spin up multiple servers using Docker. Docker allows us to
approximate a real-world networking environments, where servers can discover
one-another using DNS and facilitate communication between their respective
clients across a network.

Since the tests build fresh Docker images from the code and spin up multiple
containers, running them is somewhat slow. To make regular testing more
ergonomic, the `--include-ignored` flag has to be used when running `cargo test`
on the `server` crate. Note that Docker-based tests are run as part of the CI
whenever a pull request is made.

## Threat model

For privacy, the Phoenix Protocol aims to protect against two different types of
adversaries: the "warrant" adversary which has access to snapshots of persisted
state, and the active observer adversary which can observe the server's working
memory. Note that observation of traffic patterns and network metadata is not
part of the protocol's threat model. Metadata of that nature can be hidden using
tools such as onion routing or mixnets, both of which can be run in conjunction
with our protocol. For more documentation on the threat model, see
[here](https://docs.phnx.im/threat_model.html).

## Security measures

For the "warrant" threat model, the protocol heavily relies on
encryption-at-rest, which allows it to hide group communication metadata almost
entirely. For the active observer threat model, the protocol uses pseudonyms to
create a disconnect between a user's communication behaviour and its real
identity. For a more complete overview over the security measures, see the
[protocol specification](https://docs.phnx.im/spec.html);

## Licensing

All crates in this repository are licensed under the [AGPL 3.0](https://www.gnu.org/licenses/agpl-3.0.html). This README file is licensed under [CC-BY 4.0](https://creativecommons.org/licenses/by/4.0/).
