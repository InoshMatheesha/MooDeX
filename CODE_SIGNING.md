# Code Signing Policy for MooDeX

This document outlines the code signing policy and practices for the MooDeX project, ensuring binary integrity, provenance, and trust for all distributed executables and installers.

---

## 1. Code Signing Attribution

> **Free code signing provided by [SignPath.io](https://signpath.io), certificate by [SignPath Foundation](https://signpath.org)**  
> *(Note: Code signing via SignPath Foundation is currently planned and subject to approval).*

Until approval and integration are complete, release binaries are distributed with verifiable SHA-256 checksums and self-contained signatures published alongside each release on GitHub.

---

## 2. Project Roles & Responsibilities

In accordance with SignPath Foundation guidelines, the following roles govern code changes, releases, and signing requests:

- **Authors & Committers:**
  - **Inosh Matheesha** ([@InoshMatheesha](https://github.com/InoshMatheesha)) – Primary author, architect, and core committer.
  - Contributions from community members are accepted strictly via GitHub Pull Requests to the `main` branch.
- **Reviewers:**
  - **Inosh Matheesha** ([@InoshMatheesha](https://github.com/InoshMatheesha)) – Code reviews, lint enforcement (`cargo clippy`, `cargo fmt`), static analysis, and automated test suite verification (`cargo test`).
- **Approvers for Signing Requests:**
  - **Inosh Matheesha** ([@InoshMatheesha](https://github.com/InoshMatheesha)) – The sole designated approver authorized to request and approve release code signing operations.

---

## 3. Signing Criteria & Build Verification

Code signing requests are restricted to official releases under the following conditions:
1. **Source Provenance:** Binaries must be built exclusively from tagged commits on the official `main` branch of [https://github.com/InoshMatheesha/MooDeX](https://github.com/InoshMatheesha/MooDeX).
2. **Automated CI/CD Quality Gates:** Every build must pass all automated CI checks:
   - Zero Clippy warnings (`cargo clippy --workspace --all-targets -- -D warnings`).
   - Clean formatting check (`cargo fmt --all -- --check`).
   - 100% passing test suite across all workspace crates (`cargo test --workspace`).
   - Dependency vulnerability and advisory audits via `cargo deny`.
3. **Reproducibility & Auditability:** The exact commit hash, build script, and dependencies used for every released binary are public, reproducible, and verifiable.

---

## 4. Privacy Declaration

MooDeX maintains a strict local-only privacy standard:
- The local game library and playtime data are saved on the user's PC and **are never uploaded** to any remote server.
- Outbound network requests are limited to cover art downloads, game catalogue lookups (RAWG API), and optional update checks.
- For full details, please refer to our complete [Privacy Policy](PRIVACY.md).

---

## 5. Security Inquiries & Revocation

If you suspect a signed binary has been compromised, or if you discover a security vulnerability in MooDeX, please report it immediately:
- **Security Contact:** Inosh Matheesha (<inoshmatheesha@gmail.com>)
- **Repository Issues / Advisories:** [https://github.com/InoshMatheesha/MooDeX/security](https://github.com/InoshMatheesha/MooDeX/security)
