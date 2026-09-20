#!/usr/bin/env bash
# Local token publish is forbidden (A03:2025). A laptop with NPM_TOKEN or
# PYPI_API_TOKEN is a long-lived credential in the same class as a leaked
# GitHub PAT: anyone who copies it can ship a poisoned takehome-ca.
#
# Publish by creating a GitHub Release. .github/workflows/publish.yml uses
# OIDC: `npm publish --provenance` and PyPI Trusted Publishing. No token.
set -euo pipefail

echo "Do not publish from a laptop." >&2
echo "Create a GitHub Release; publish.yml uses OIDC (npm provenance, PyPI Trusted Publishing)." >&2
exit 1
