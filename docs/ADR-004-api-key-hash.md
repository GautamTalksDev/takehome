# ADR-004: API keys are SHA-256, not bcrypt

**Who this is for:** API maintainers reviewing D1 schema and authentication.

**When you finish:** You know why keys are hashed with SHA-256 and must not move to bcrypt.

## Status

Accepted (A04:2025)

## Context

API keys (`np_test_…` / `np_live_…`) are high-entropy 128-bit CSPRNG tokens from
`crypto.getRandomValues`, rendered as lowercase hex. They are not
user-chosen passwords. They are looked up on every authenticated request
by hashing the presented Bearer secret and fetching the row.

A later reader who sees a SHA-256 hex in `api_keys.hash` may assume this
is a password store and "fix" it to bcrypt / scrypt / argon2. That is
the wrong model, and it is a breaking change:

- Password KDFs are deliberately slow and salted. You cannot index
  `SHA256(presented)` and find the row. Lookup becomes a scan of every
  key, or you store a separate lookup hash and have two schemes.
- Treating the column as a password hash invites lowering key entropy
  ("users will type this") or adding a pepper/salt that the request path
  does not have.

## Decision

Store `SHA-256(token)` as 64 lowercase hex characters. Verify with
`equalDigest` (`crypto.timingSafeEqual`) against the stored digest.

Do not bcrypt these tokens. Do not reduce `KEY_RANDOM_BYTES` below 16.
If a human-memorable secret is ever added, that is a different column
with a password KDF; it is not this one.

## Consequences

- Lost keys cannot be recovered from D1. Rotation is re-issue.
- A leaked hash is not the key, but it is a lookup token: protect D1 as
  you would a session table, not as a password file.
- bcrypt-on-keys is a defect, not a hardening.

**Last reviewed:** 2026-09-20  
**Engine:** 0.1.0
