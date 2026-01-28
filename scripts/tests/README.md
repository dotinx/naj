# Gosh Integration & Scenario Tests

This directory contains the integration test suite and scenario simulations for `gosh`. These scripts are designed to validate the core functionality, security isolation, and developer workflows in a controlled, sandbox environment.

## Overview

The tests in this directory focus on end-to-end (E2E) validation, ensuring that `gosh` correctly interacts with Git configurations, SSH keys, and system environments without contaminating the user's global settings.

## Test Matrix

| Script | Focus Area | Description |
| :--- | :--- | :--- |
| `alice.sh` | **Basic Workflow** | Validates profile creation, cloning, and identity switching with SSH signing. |
| `alice2.sh` | **Advanced Signing** | Extends basic tests with complex signing scenarios and multi-profile setups. |
| `alice3.sh` | **Full Lifecycle** | Simulates a complex developer lifecycle involving multiple organizations. |
| `edge_cases.sh`| **Robustness** | Tests boundary conditions, invalid inputs, and error handling. |
| `security_edge.sh`| **Security** | Validates configuration isolation and prevents "leakage" between profiles. |

## Prerequisites

To run these tests locally, ensure you have the following installed:
- **Bash**: Most scripts use standard Bash features.
- **Git**: Version 2.34+ is required for SSH signing tests.
- **Gosh**: The `gosh` binary must be available in your PATH or accessible via the `GOSH_CMD` environment variable.

## Running Tests

### Preparation

Before running the tests, compile the project to ensure the latest binary is used:

```bash
cargo build --release
export PATH="$PWD/target/release:$PATH"
```

### Execution

You can run individual test scripts directly:

```bash
bash scripts/tests/alice.sh
```

Each script initializes a sandbox in `/tmp/gosh_test_*` or similar, ensuring that your `~/.gitconfig` and `~/.ssh` remain untouched.

## Design Principles

1. **Isolation**: All tests use a dedicated `GOSH_CONFIG_PATH` and temporary directories to ensure side-effect-free execution.
2. **Assertions**: Scripts use exit codes and explicit checks to verify expected outcomes (e.g., checking `git cat-file` for signatures).
3. **Readability**: Log levels (STEP, INFO, ERROR) are used to provide clear feedback during execution.

## Adding New Tests

When adding a new test script:
- Use `set -e` to ensure the script fails on the first error.
- Use a dedicated sandbox directory for all temporary files.
- Export `GOSH_CONFIG_PATH` to point into your sandbox.
- Document the scenario and expected results in the script header.