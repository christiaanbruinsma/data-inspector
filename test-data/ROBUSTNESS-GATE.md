# Data Inspector v0.9 robustness gate

Use these files for the runtime gate before release polish.

## Error handling

- `invalid-json-trailing-comma.json` -> reject with an Invalid JSON toast containing line/column.
- `invalid-json-unclosed.json` -> reject with an Invalid JSON toast containing line/column.
- `csv-empty.csv` -> reject with `This CSV file is empty.`
- `csv-invalid-utf8.csv` -> reject with `This file is not valid UTF-8 CSV.`
- A failed open must not replace an already-open valid document.

## BOM source fidelity

- `json-with-bom.json` -> Structured parses normally; Raw retains the original decoded source including its leading UTF-8 BOM.
- `csv-with-bom.csv` -> first header must be `name`, not a BOM-prefixed variant; Raw retains the original decoded source.

## CSV structure

- `csv-edge-cases.csv` -> quoted delimiters, multiline cells, empty cells.
- `csv-headerless.csv` -> generated column names; First row is header override remains reversible.
- `csv-ragged.csv` -> flexible rows load and missing cells normalize as empty cells.

## Performance/runtime

Open, search, sort, inspect cells, and middle-mouse pan each of:

- `csv-stress-test-10000.csv`
- `csv-stress-test-50000.csv`
- `csv-stress-test-100000.csv`

Record any visible freeze, long input lag, panic, GTK critical, or excessive memory behavior before changing the implementation.
