//! d3-fetch demonstration
//!
//! This example demonstrates the data loading utilities inspired by d3-fetch:
//! - CSV parsing
//! - TSV parsing
//! - Custom delimiter parsing (DSV)
//! - Automatic type inference

use d3rs::fetch::{
    AutoTyped, DsvParser, DsvRow, auto_type, auto_type_row, format_csv, parse_csv, parse_dsv,
    parse_tsv,
};

fn main() {
    println!("=== d3-fetch Demonstration ===\n");

    // ========================================
    // CSV Parsing
    // ========================================
    println!("--- CSV Parsing ---\n");

    let csv_data = r#"name,age,score,active
Alice,30,95.5,true
Bob,25,87.2,false
Carol,35,92.8,true
Dave,28,,false
Eve,22,88.0,true"#;

    println!("Input CSV:");
    println!("{}\n", csv_data);

    let rows = parse_csv(csv_data);
    println!("Parsed {} rows:\n", rows.len());

    for (i, row) in rows.iter().enumerate() {
        println!("  Row {}: {:?}", i, row);
    }

    // Access specific fields
    println!("\nAccessing fields:");
    if let Some(first) = rows.first() {
        println!("  First row name: {:?}", first.get("name"));
        println!("  First row age: {:?}", first.get("age"));
        println!("  First row score: {:?}", first.get("score"));
    }

    // ========================================
    // TSV Parsing
    // ========================================
    println!("\n--- TSV Parsing ---\n");

    let tsv_data = "product\tprice\tquantity
Widget\t19.99\t100
Gadget\t29.99\t50
Gizmo\t9.99\t200";

    println!("Input TSV:");
    println!("{}\n", tsv_data);

    let tsv_rows = parse_tsv(tsv_data);
    println!("Parsed {} rows:", tsv_rows.len());
    for row in &tsv_rows {
        println!(
            "  {}: ${} x {}",
            row.get("product").unwrap_or(&String::new()),
            row.get("price").unwrap_or(&String::new()),
            row.get("quantity").unwrap_or(&String::new())
        );
    }

    // ========================================
    // Custom Delimiter (DSV)
    // ========================================
    println!("\n--- Custom Delimiter (Pipe) ---\n");

    let pipe_data = "id|category|value
1|Electronics|500
2|Books|150
3|Clothing|300";

    println!("Input (pipe-delimited):");
    println!("{}\n", pipe_data);

    let pipe_rows = parse_dsv(pipe_data, '|');
    println!("Parsed {} rows:", pipe_rows.len());
    for row in &pipe_rows {
        println!(
            "  {} - {}: {}",
            row.get("id").unwrap_or(&String::new()),
            row.get("category").unwrap_or(&String::new()),
            row.get("value").unwrap_or(&String::new())
        );
    }

    // ========================================
    // Quoted Fields
    // ========================================
    println!("\n--- Quoted Fields ---\n");

    let quoted_csv = r#"name,description,price
"Widget","A useful widget",19.99
"Super Gadget","The best gadget, ever!",49.99
"Mega Gizmo","He said, ""wow!""",99.99"#;

    println!("Input (with quoted fields):");
    println!("{}\n", quoted_csv);

    let quoted_rows = parse_csv(quoted_csv);
    println!("Parsed {} rows:", quoted_rows.len());
    for row in &quoted_rows {
        println!(
            "  {}: {} ({})",
            row.get("name").unwrap_or(&String::new()),
            row.get("description").unwrap_or(&String::new()),
            row.get("price").unwrap_or(&String::new())
        );
    }

    // ========================================
    // Formatting (Writing) CSV
    // ========================================
    println!("\n--- Formatting CSV ---\n");

    // Create some data
    let mut data: Vec<DsvRow> = Vec::new();

    let mut row1 = DsvRow::new();
    row1.insert("x".to_string(), "10".to_string());
    row1.insert("y".to_string(), "20".to_string());
    row1.insert("label".to_string(), "Point A".to_string());
    data.push(row1);

    let mut row2 = DsvRow::new();
    row2.insert("x".to_string(), "30".to_string());
    row2.insert("y".to_string(), "40".to_string());
    row2.insert("label".to_string(), "Point B, with comma".to_string());
    data.push(row2);

    let output = format_csv(&data, &["x", "y", "label"]);
    println!("Formatted CSV output:");
    println!("{}", output);

    // ========================================
    // Auto Type Inference
    // ========================================
    println!("--- Auto Type Inference ---\n");

    let test_values = vec![
        "42",
        "3.14159",
        "true",
        "false",
        "",
        "NaN",
        "hello world",
        "2023-12-25",
        "1e10",
        "-0.5",
        "infinity",
    ];

    println!("Type inference results:");
    for value in &test_values {
        let typed = auto_type(value);
        let type_name = match &typed {
            AutoTyped::Null => "Null",
            AutoTyped::Bool(_) => "Bool",
            AutoTyped::Integer(_) => "Integer",
            AutoTyped::Float(_) => "Float",
            AutoTyped::String(_) => "String",
            AutoTyped::Date(_) => "Date",
        };
        println!(
            "  {:20} -> {:10} = {:?}",
            format!("\"{}\"", value),
            type_name,
            typed
        );
    }

    // ========================================
    // Auto Type on Entire Row
    // ========================================
    println!("\n--- Auto Type on Row ---\n");

    let typed_row = auto_type_row(&rows[0]);
    println!("First row with auto-typing:");
    for (key, value) in &typed_row {
        let type_name = match value {
            AutoTyped::Null => "Null",
            AutoTyped::Bool(_) => "Bool",
            AutoTyped::Integer(_) => "Integer",
            AutoTyped::Float(_) => "Float",
            AutoTyped::String(_) => "String",
            AutoTyped::Date(_) => "Date",
        };
        println!("  {}: {:?} ({})", key, value, type_name);
    }

    // ========================================
    // Working with Typed Values
    // ========================================
    println!("\n--- Working with Typed Values ---\n");

    let score = auto_type("95.5");
    let age = auto_type("30");
    let active = auto_type("true");

    println!("Accessing typed values:");
    println!("  Score as f64: {:?}", score.as_f64());
    println!("  Age as i64: {:?}", age.as_i64());
    println!("  Active as bool: {:?}", active.as_bool());

    // Calculate with typed data
    if let (Some(s), Some(a)) = (score.as_f64(), age.as_i64()) {
        println!("\nCalculation: score / age = {:.2}", s / a as f64);
    }

    // ========================================
    // DsvParser Configuration
    // ========================================
    println!("\n--- Custom Parser Configuration ---\n");

    let parser = DsvParser::new(',').skip_empty_lines(true).trim_values(true);

    let messy_csv = "  name  ,  value
  Alice  ,  100

  Bob  ,  200  ";

    println!("Input (messy whitespace):");
    println!("{}\n", messy_csv);

    let cleaned = parser.parse(messy_csv);
    println!("Parsed (trimmed):");
    for row in &cleaned {
        println!(
            "  '{}' = '{}'",
            row.get("name").unwrap_or(&String::new()),
            row.get("value").unwrap_or(&String::new())
        );
    }

    // Parse without headers (rows mode)
    let raw_data = "1,2,3\n4,5,6\n7,8,9";
    let raw_rows = parser.parse_rows(raw_data);
    println!("\nRaw rows (no headers):");
    for (i, row) in raw_rows.iter().enumerate() {
        println!("  Row {}: {:?}", i, row);
    }

    // ========================================
    // Real-world Example
    // ========================================
    println!("\n--- Real-world Example: Sales Data ---\n");

    let sales_csv = r#"date,product,region,quantity,unit_price,total
2023-01-15,Widget,North,100,19.99,1999.00
2023-01-15,Gadget,South,50,29.99,1499.50
2023-01-16,Widget,East,75,19.99,1499.25
2023-01-16,Gizmo,West,200,9.99,1998.00
2023-01-17,Gadget,North,30,29.99,899.70"#;

    let sales = parse_csv(sales_csv);

    println!("Sales summary:");

    // Group by product
    let mut by_product: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    for row in &sales {
        let product = row.get("product").unwrap().clone();
        let total: f64 = row.get("total").and_then(|s| s.parse().ok()).unwrap_or(0.0);
        *by_product.entry(product).or_insert(0.0) += total;
    }

    println!("\n  Total by product:");
    for (product, total) in &by_product {
        println!("    {}: ${:.2}", product, total);
    }

    // Group by region
    let mut by_region: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    for row in &sales {
        let region = row.get("region").unwrap().clone();
        let total: f64 = row.get("total").and_then(|s| s.parse().ok()).unwrap_or(0.0);
        *by_region.entry(region).or_insert(0.0) += total;
    }

    println!("\n  Total by region:");
    for (region, total) in &by_region {
        println!("    {}: ${:.2}", region, total);
    }

    // Grand total
    let grand_total: f64 = by_product.values().sum();
    println!("\n  Grand total: ${:.2}", grand_total);

    println!("\n=== End of d3-fetch Demo ===");
}
