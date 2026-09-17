"""JSON wire tests for the Python binding (spec §5.4 / §9.3)."""

from __future__ import annotations

import json
from pathlib import Path

import takehome_ca

REPO = Path(__file__).resolve().parents[3]
M1 = REPO / "crates/takehome-core/tests/vectors/pdoc_ontario_2026_01.json"


def test_malformed_json_returns_error_and_does_not_raise() -> None:
    body = json.loads(takehome_ca.calculate("{"))
    assert body["error"]["code"] == "malformed_json"
    assert "employee" not in body


def test_m1_vectors_match_expected_cents() -> None:
    vectors = json.loads(M1.read_text())["vectors"]
    assert len(vectors) == 20
    for vector in vectors:
        parsed = json.loads(takehome_ca.calculate(json.dumps(vector["request"])))
        assert parsed["employee"]["net_pay"] == vector["expected"]["net_pay"], vector["id"]
        assert parsed["employee"]["federal_tax"] == vector["expected"]["federal_tax"], vector["id"]
        assert parsed["engine_build_sha256"] == takehome_ca.engine_build_sha()


def test_engine_build_sha_is_64_hex() -> None:
    sha = takehome_ca.engine_build_sha()
    assert len(sha) == 64
    assert all(c in "0123456789abcdef" for c in sha)


def test_listings() -> None:
    jurisdictions = json.loads(takehome_ca.list_jurisdictions())
    qc = next(row for row in jurisdictions["jurisdictions"] if row["code"] == "QC")
    assert qc["supported"] is False
    versions = json.loads(takehome_ca.list_rule_set_versions())
    assert [row["version"] for row in versions["rule_set_versions"]] == [
        "2026-01-01",
        "2026-07-01",
    ]
