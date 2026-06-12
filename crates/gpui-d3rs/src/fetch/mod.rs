//! Data fetching and parsing utilities
//!
//! This module provides utilities for parsing common data formats like CSV, TSV,
//! and JSON. Inspired by d3-fetch but adapted for Rust's synchronous model.
//!
//! Note: For actual HTTP fetching, use reqwest or similar crates. This module
//! focuses on parsing the data once you have it as a string.
//!
//! # Example
//!
//! ```rust
//! use d3rs::fetch::{parse_csv, parse_tsv, CsvOptions};
//!
//! let csv_data = "name,value\nalice,10\nbob,20";
//! let rows = parse_csv(csv_data).unwrap();
//! assert_eq!(rows.len(), 2);
//! assert_eq!(rows[0].get("name"), Some(&"alice".to_string()));
//! ```

mod auto_type;
mod csv;
mod dsv;

pub use auto_type::{AutoTyped, auto_type, auto_type_row, auto_type_rows};
pub use csv::{
    CsvOptions, format_csv, format_tsv, parse_csv, parse_csv_lossy, parse_csv_lossy_with_options,
    parse_csv_with_options, parse_tsv, parse_tsv_lossy, parse_tsv_lossy_with_options,
    parse_tsv_with_options, try_parse_csv, try_parse_csv_with_options, try_parse_tsv,
    try_parse_tsv_with_options,
};
pub use dsv::{
    ColumnPolicy, DsvParseError, DsvParseErrorKind, DsvParser, DsvResult, DsvRow, parse_dsv,
    parse_dsv_lossy, try_parse_dsv,
};
