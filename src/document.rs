use std::{
    fs,
    path::{Path, PathBuf},
};

use csv::ReaderBuilder;
use serde_json::Value;

use crate::i18n::{gettext, replace_named};

#[derive(Debug)]
pub struct JsonDocument {
    pub path: PathBuf,
    pub value: Value,
    pub raw_text: String,
    pub size_bytes: u64,
    pub node_count: usize,
    pub max_depth: usize,
}

#[derive(Debug)]
pub struct CsvDocument {
    pub path: PathBuf,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub raw_text: String,
    pub size_bytes: u64,
    pub delimiter: u8,
    pub has_headers: bool,
}

impl CsvDocument {
    pub fn set_has_headers(&mut self, has_headers: bool) {
        if self.has_headers == has_headers || self.headers.is_empty() {
            return;
        }

        if has_headers {
            if self.rows.is_empty() {
                return;
            }
            self.headers = self.rows.remove(0);
        } else {
            let column_count = self.headers.len();
            let original_header = std::mem::replace(
                &mut self.headers,
                (1..=column_count).map(generated_column_name).collect(),
            );
            self.rows.insert(0, original_header);
        }

        self.has_headers = has_headers;
    }
}

#[derive(Debug)]
pub enum DataDocument {
    Json(JsonDocument),
    Csv(CsvDocument),
}

impl DataDocument {
    pub fn path(&self) -> &Path {
        match self {
            Self::Json(document) => &document.path,
            Self::Csv(document) => &document.path,
        }
    }

    pub fn raw_text(&self) -> &str {
        match self {
            Self::Json(document) => &document.raw_text,
            Self::Csv(document) => &document.raw_text,
        }
    }

    pub fn size_bytes(&self) -> u64 {
        match self {
            Self::Json(document) => document.size_bytes,
            Self::Csv(document) => document.size_bytes,
        }
    }

    pub fn format_name(&self) -> &'static str {
        match self {
            Self::Json(_) => "JSON",
            Self::Csv(_) => "CSV",
        }
    }
}

pub fn load_document(path: &Path) -> Result<DataDocument, String> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    match extension.as_str() {
        "json" => load_json(path).map(DataDocument::Json),
        "csv" | "tsv" => load_csv(path).map(DataDocument::Csv),
        _ => Err(gettext(
            "Unsupported file type. Open a JSON, CSV, or TSV file.",
        )),
    }
}

fn read_utf8(path: &Path, format_name: &str) -> Result<(String, u64), String> {
    let bytes = fs::read(path).map_err(|error| {
        replace_named(
            gettext("Could not read file: {error}"),
            &[("error", error.to_string())],
        )
    })?;
    let size_bytes = bytes.len() as u64;
    let text = String::from_utf8(bytes).map_err(|_| {
        replace_named(
            gettext("This file is not valid UTF-8 {format}."),
            &[("format", format_name.to_string())],
        )
    })?;
    Ok((text, size_bytes))
}

fn generated_column_name(index: usize) -> String {
    replace_named(gettext("Column {index}"), &[("index", index.to_string())])
}

fn parse_source_text(raw_text: &str) -> &str {
    raw_text.strip_prefix('\u{feff}').unwrap_or(raw_text)
}

fn parse_json_text(raw_text: &str) -> Result<Value, String> {
    serde_json::from_str(parse_source_text(raw_text)).map_err(|error| {
        replace_named(
            gettext("Invalid JSON at line {line}, column {column}: {error}"),
            &[
                ("line", error.line().to_string()),
                ("column", error.column().to_string()),
                ("error", error.to_string()),
            ],
        )
    })
}

fn load_json(path: &Path) -> Result<JsonDocument, String> {
    let (raw_text, size_bytes) = read_utf8(path, "JSON")?;
    let value = parse_json_text(&raw_text)?;
    let (node_count, max_depth) = document_metrics(&value);

    Ok(JsonDocument {
        path: path.to_path_buf(),
        value,
        raw_text,
        size_bytes,
        node_count,
        max_depth,
    })
}

fn load_csv(path: &Path) -> Result<CsvDocument, String> {
    let (raw_text, size_bytes) = read_utf8(path, "CSV")?;
    let parse_text = parse_source_text(&raw_text);
    if parse_text.trim().is_empty() {
        return Err(gettext("This CSV file is empty."));
    }

    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    let delimiter = if extension == "tsv" {
        b'\t'
    } else {
        detect_delimiter(parse_text)
    };

    let mut records = parse_csv_records(parse_text, delimiter)?;

    if records.is_empty() {
        return Err(gettext("This CSV file contains no records."));
    }

    let column_count = records.iter().map(Vec::len).max().unwrap_or(0);
    if column_count == 0 {
        return Err(gettext("This CSV file contains no columns."));
    }

    for record in &mut records {
        record.resize(column_count, String::new());
    }

    let has_headers = detect_headers(&records);
    let headers = if has_headers {
        records.remove(0)
    } else {
        (1..=column_count).map(generated_column_name).collect()
    };

    Ok(CsvDocument {
        path: path.to_path_buf(),
        headers,
        rows: records,
        raw_text,
        size_bytes,
        delimiter,
        has_headers,
    })
}

fn parse_csv_records(text: &str, delimiter: u8) -> Result<Vec<Vec<String>>, String> {
    let mut reader = ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .flexible(true)
        .from_reader(text.as_bytes());

    reader
        .records()
        .map(|result| {
            result
                .map(|record| record.iter().map(ToString::to_string).collect::<Vec<_>>())
                .map_err(|error| {
                    replace_named(
                        gettext("Invalid CSV: {error}"),
                        &[("error", error.to_string())],
                    )
                })
        })
        .collect()
}

fn detect_delimiter(text: &str) -> u8 {
    b",;\t"
        .iter()
        .copied()
        .max_by_key(|delimiter| delimiter_score(text, *delimiter))
        .unwrap_or(b',')
}

fn delimiter_score(text: &str, delimiter: u8) -> i64 {
    let mut reader = ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .flexible(true)
        .from_reader(text.as_bytes());

    let widths = reader
        .records()
        .take(30)
        .filter_map(Result::ok)
        .map(|record| record.len())
        .collect::<Vec<_>>();

    if widths.is_empty() {
        return 0;
    }

    let max_width = *widths.iter().max().unwrap_or(&1);
    if max_width <= 1 {
        return widths.len() as i64;
    }

    let mut frequencies = std::collections::HashMap::<usize, usize>::new();
    for width in &widths {
        *frequencies.entry(*width).or_default() += 1;
    }
    let consistency = frequencies.values().copied().max().unwrap_or(0);

    (consistency as i64 * 10_000) + (max_width as i64 * 100) + widths.len() as i64
}

fn detect_headers(records: &[Vec<String>]) -> bool {
    let Some(first) = records.first() else {
        return false;
    };
    if first.is_empty() || first.iter().any(|value| value.trim().is_empty()) {
        return false;
    }

    let mut unique = std::collections::HashSet::new();
    if !first
        .iter()
        .all(|value| unique.insert(value.trim().to_lowercase()))
    {
        return false;
    }

    let first_data_like = first.iter().filter(|value| is_data_like(value)).count();
    let second_data_like = records
        .get(1)
        .map(|record| record.iter().filter(|value| is_data_like(value)).count())
        .unwrap_or(0);

    first_data_like == 0 || second_data_like > first_data_like
}

fn is_data_like(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() {
        return true;
    }
    if value.eq_ignore_ascii_case("true")
        || value.eq_ignore_ascii_case("false")
        || value.eq_ignore_ascii_case("null")
    {
        return true;
    }
    value.parse::<f64>().is_ok()
}

fn document_metrics(value: &Value) -> (usize, usize) {
    fn walk(value: &Value, depth: usize, nodes: &mut usize, max_depth: &mut usize) {
        *nodes += 1;
        *max_depth = (*max_depth).max(depth);
        match value {
            Value::Array(items) => {
                for child in items {
                    walk(child, depth + 1, nodes, max_depth);
                }
            }
            Value::Object(map) => {
                for child in map.values() {
                    walk(child, depth + 1, nodes, max_depth);
                }
            }
            _ => {}
        }
    }

    let mut nodes = 0;
    let mut max_depth = 0;
    walk(value, 0, &mut nodes, &mut max_depth);
    (nodes, max_depth)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_common_delimiters() {
        assert_eq!(detect_delimiter("name,email\nAda,ada@example.com\n"), b',');
        assert_eq!(detect_delimiter("name;email\nAda;ada@example.com\n"), b';');
        assert_eq!(
            detect_delimiter("name\temail\nAda\tada@example.com\n"),
            b'\t'
        );
    }

    #[test]
    fn detects_simple_header_row() {
        let records = vec![
            vec!["name".into(), "age".into(), "active".into()],
            vec!["Ada".into(), "37".into(), "true".into()],
        ];
        assert!(detect_headers(&records));
    }

    #[test]
    fn detects_headerless_numeric_records() {
        let records = vec![
            vec!["1".into(), "19.73".into(), "true".into()],
            vec!["2".into(), "39.46".into(), "false".into()],
        ];
        assert!(!detect_headers(&records));
    }

    #[test]
    fn parses_quoted_delimiters_and_multiline_cells() {
        let records = parse_csv_records(
            "id,note\n1,\"comma, stays inside\"\n2,\"line one\nline two\"\n",
            b',',
        )
        .expect("quoted CSV should parse");

        assert_eq!(records.len(), 3);
        assert_eq!(records[1][1], "comma, stays inside");
        assert_eq!(records[2][1], "line one\nline two");
    }

    #[test]
    fn toggles_csv_header_layout_without_losing_first_row() {
        let mut document = CsvDocument {
            path: PathBuf::from("test.csv"),
            headers: vec!["name".into(), "email".into()],
            rows: vec![vec!["Ada".into(), "ada@example.com".into()]],
            raw_text: "name,email\nAda,ada@example.com\n".into(),
            size_bytes: 0,
            delimiter: b',',
            has_headers: true,
        };

        document.set_has_headers(false);
        assert_eq!(document.headers, vec!["Column 1", "Column 2"]);
        assert_eq!(document.rows[0], vec!["name", "email"]);

        document.set_has_headers(true);
        assert_eq!(document.headers, vec!["name", "email"]);
        assert_eq!(document.rows, vec![vec!["Ada", "ada@example.com"]]);
    }

    #[test]
    fn keeps_bom_in_raw_source_but_excludes_it_from_parsing() {
        let raw = "\u{feff}{\"name\":\"Ada\"}";
        assert!(raw.starts_with('\u{feff}'));
        assert_eq!(parse_source_text(raw), "{\"name\":\"Ada\"}");

        let value = parse_json_text(raw).expect("BOM-prefixed JSON should parse");
        assert_eq!(value["name"], "Ada");
    }

    #[test]
    fn loaded_json_preserves_bom_in_raw_view_source() {
        let path =
            std::env::temp_dir().join(format!("data-inspector-bom-{}.json", std::process::id()));
        std::fs::write(&path, b"\xEF\xBB\xBF{\"name\":\"Ada\"}")
            .expect("test JSON should be written");

        let document = load_json(&path).expect("BOM-prefixed JSON should load");
        let _ = std::fs::remove_file(&path);

        assert!(document.raw_text.starts_with('\u{feff}'));
        assert_eq!(document.value["name"], "Ada");
    }

    #[test]
    fn loaded_csv_preserves_bom_without_polluting_first_header() {
        let path =
            std::env::temp_dir().join(format!("data-inspector-bom-{}.csv", std::process::id()));
        std::fs::write(&path, b"\xEF\xBB\xBFname,age\nAda,37\n")
            .expect("test CSV should be written");

        let document = load_csv(&path).expect("BOM-prefixed CSV should load");
        let _ = std::fs::remove_file(&path);

        assert!(document.raw_text.starts_with('\u{feff}'));
        assert_eq!(document.headers, vec!["name", "age"]);
        assert_eq!(document.rows, vec![vec!["Ada", "37"]]);
    }

    #[test]
    fn rejects_non_utf8_source_with_clear_error() {
        let path = std::env::temp_dir().join(format!(
            "data-inspector-invalid-utf8-{}.csv",
            std::process::id()
        ));
        std::fs::write(&path, [0xFF, 0xFE, b'a', b',', b'b'])
            .expect("invalid UTF-8 fixture should be written");

        let error = load_csv(&path).expect_err("invalid UTF-8 CSV must be rejected");
        let _ = std::fs::remove_file(&path);

        assert_eq!(error, "This file is not valid UTF-8 CSV.");
    }

    #[test]
    fn reports_json_line_and_column_for_invalid_input() {
        let error = parse_json_text("{\n  \"name\": \"Ada\",\n}\n")
            .expect_err("trailing comma must be rejected");
        assert!(error.starts_with("Invalid JSON at line 3, column"));
    }

    #[test]
    fn delimiter_detection_ignores_delimiters_inside_quotes() {
        let semicolon = "id;note\n1;\"comma, stays inside\"\n2;\"another, comma\"\n";
        assert_eq!(detect_delimiter(semicolon), b';');
    }

    #[test]
    fn flexible_csv_parser_keeps_ragged_records_for_normalization() {
        let records = parse_csv_records("a,b,c\n1,2\n3,4,5\n", b',')
            .expect("ragged CSV should parse in flexible mode");
        assert_eq!(records[0].len(), 3);
        assert_eq!(records[1].len(), 2);
        assert_eq!(records[2].len(), 3);
    }
}
