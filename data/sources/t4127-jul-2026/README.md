# T4127 July 2026 CSV archive (123rd edition)

Byte-for-byte copies of the Chapter 8 CSV files hosted under

https://www.canada.ca/en/revenue-agency/services/forms-publications/payroll/t4127-payroll-deductions-formulas/t4127-jul.html

Retrieved `2026-09-12T04:49:52Z`. Hashes in `archive.json` are recorded
**before** ingest. The ingest tool parses only this directory, never a URL.

## This edition is a delta

T4127 is explicit:

> Please refer to the 122nd edition for any sections that have not been reproduced.

Only British Columbia, Newfoundland and Labrador, and Prince Edward Island
changed. That sentence is the `delta_marker` in `archive.json`. Ingest must
key off the marker, not Table 8.1 row count: CRA reprints the unchanged
jurisdictions in the July rate CSV.

`supplement.json` holds Option 2 tables and the BC tax-reduction upper
threshold. Those figures are in T4127 Chapters 4 and 5, not in the CSV
bundle. The parser does not invent them.
