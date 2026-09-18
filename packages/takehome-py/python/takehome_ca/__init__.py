"""Canadian T4127 payroll deductions. Same engine as native Rust and WASM."""

from takehome_ca._native import (
    calculate,
    diff_rule_sets,
    engine_build_sha,
    list_jurisdictions,
    list_rule_set_versions,
)

__all__ = [
    "calculate",
    "diff_rule_sets",
    "engine_build_sha",
    "list_jurisdictions",
    "list_rule_set_versions",
]
