# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |

## Scope

djvu-rs is a parser for an untrusted binary format. The following are in scope:

- Panics or aborts when parsing malformed DjVu input
- Out-of-bounds memory access (the crate forbids `unsafe`, but logic bugs can still
  cause index panics)
- Integer overflow leading to incorrect output or resource exhaustion
- Denial-of-service via crafted input (excessive allocation, infinite loops)

## Reporting a vulnerability

Please **do not** open a public GitHub issue for security vulnerabilities.

Report privately via GitHub's built-in mechanism:
**Security → Report a vulnerability** on the repository page.

Include:
- A minimal reproducer (a byte sequence or `#[test]` that triggers the issue)
- Rust version and OS
- What you expected vs. what happened

You will receive a response within 7 days. If the issue is confirmed, a patched
release will be published and you will be credited in the changelog unless you
prefer otherwise.

## Fuzzing

The `fuzz/` directory contains libFuzzer targets for the main codec entry points.
If you find a crash via fuzzing, please include the minimized corpus input.
