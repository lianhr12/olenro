//! Output formatting utilities

use crate::errors::Result;
use serde::Serialize;

/// Print a formatted table
pub fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    if rows.is_empty() {
        println!("(empty)");
        return;
    }

    // Calculate column widths
    let col_widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    let col_widths: Vec<usize> = rows.iter().fold(col_widths, |acc, row| {
        acc.iter().zip(row.iter()).map(|(a, b)| (*a).max(b.len())).collect()
    });

    // Print header
    for (i, header) in headers.iter().enumerate() {
        print!("{:<width$} ", header, width = col_widths[i]);
    }
    println!();
    println!("{}", "-".repeat(col_widths.iter().sum()));

    // Print rows
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            print!("{:<width$} ", cell, width = col_widths[i]);
        }
        println!();
    }
}

/// Print as JSON
pub fn print_json<T: Serialize>(value: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| crate::errors::CliError::Failed(e.to_string()))?;
    println!("{}", json);
    Ok(())
}
