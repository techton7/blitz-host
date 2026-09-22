# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1](https://github.com/techton7/blitz-host/compare/blitz-host-transport-v0.1.0...blitz-host-transport-v0.1.1) - 2026-09-22

### Added

- add blitz-host facade crate, canonical CLI, and HostControl helper
- *(cli)* rename canonical binary from blitz-inspect to blitz-host with subcommands and flags
- *(cli)* add comprehensive --help and --json options to blitz-inspect
- *(cli)* add --click and settle support to blitz-inspect tool

### Other

- *(cli)* simplify blitz-host CLI with explicit subcommands and dedicated --help
- release v0.1.0 ([#1](https://github.com/techton7/blitz-host/pull/1))

## [0.1.0](https://github.com/techton7/blitz-host/releases/tag/blitz-host-transport-v0.1.0) - 2026-09-21

### Added

- initial blitz-host commit with inspect, act, settle, and release-plz

### Fixed

- *(cargo)* add version requirements for inter-crate path dependencies
