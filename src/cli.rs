// Copyright 2025, University of Maryland, All Rights Reserved

use anyhow::{Result};
use rusqlite::Connection;
use crate::commands;
use crate::resolver;

/// Outcome of a command execution
pub enum CommandOutcome {
    Continue,
    Exit,
}

/// Parses the input line and dispatches the command.
pub async fn parse_and_dispatch_command(line: &str, conn: &Connection) -> Result<CommandOutcome> {
    let parts = split_command_line(line)?;
    if parts.is_empty() {
        return Ok(CommandOutcome::Continue);
    }

    let command_name = parts[0].to_lowercase();
    let args = parts[1..].to_vec();

    // Basic command parsing logic (to be expanded significantly)
    match command_name.as_str() {
        "exit" | "quit" | "q" => Ok(CommandOutcome::Exit),
        "help" | "h" | "?" => {
            commands::handle_help(&args, conn)?;
            Ok(CommandOutcome::Continue)
        }
        "about" => {
            commands::handle_about()?;
            Ok(CommandOutcome::Continue)
        }
        "self-update" => {
            commands::handle_self_update().await?;
            Ok(CommandOutcome::Continue)
        }
        "list-tables" => {
            commands::list_tables(&args, conn)?;
            Ok(CommandOutcome::Continue)
        }
        "list-columns" => {
            if args.is_empty() {
                commands::handle_help(&["list-columns".to_string()], conn)?;
            } else {
                // Pass the first argument as the table_name
                let mut all = false;
                if args.len() > 1 && args[1] == "all" {
                    all = true;
                }

                commands::list_columns(&args[0], &all, conn)?;
            }
            Ok(CommandOutcome::Continue)
        }
        "aws-download" => {
            if args.is_empty() {
                commands::handle_help(&["aws-download".to_string()], conn)?;
            } else {
                commands::aws_download(&args[0], conn).await?;
            }
            Ok(CommandOutcome::Continue)
        }
        "query-table" => {
            if args.is_empty() || args.len() < 2 {
                commands::handle_help(&["query-table".to_string()], conn)?;
            } else {
                // Pass the first argument as the table_name
                let table_name = &args[0];
                let position_str = if crate::SUPPORTED_TABLES.contains(&table_name.as_str()) {
                    resolver::resolve_position(&args[1]).await?
                } else {
                    args[1].clone()
                };
                let mut radius  = 0.0;
                let mut columns_arg: Option<String> = None;
                let mut products_only_flag = false;

                let mut current_arg_idx = 2; // Index for optional arguments

                // Try to parse radius
                if current_arg_idx < args.len() {
                    if let Ok(r_val) = args[current_arg_idx].parse::<f64>() {
                        radius = r_val;
                        current_arg_idx += 1;
                    }
                    // If not a float, assume it's not radius, and current_arg_idx remains for columns/products
                }

                // Try to parse columns specifier
                if current_arg_idx < args.len() && args[current_arg_idx].to_lowercase() != "products" {
                    columns_arg = Some(args[current_arg_idx].clone());
                    current_arg_idx += 1;
                }

                // Try to parse "products" flag
                if current_arg_idx < args.len() && args[current_arg_idx].to_lowercase() == "products" {
                    products_only_flag = true;
                }

                commands::query_table(table_name, &position_str, &radius, &columns_arg, &products_only_flag, conn)?;
            }
            Ok(CommandOutcome::Continue)
        }
        _ => {
            println!("Unknown command: {}", command_name);
            Ok(CommandOutcome::Continue)
        }
    }
}

fn split_command_line(line: &str) -> Result<Vec<String>> {
    let mut parts = Vec::new();
    let mut chars = line.chars().peekable();

    while let Some(&next) = chars.peek() {
        if next.is_whitespace() {
            chars.next();
            continue;
        }

        let mut part = String::new();
        if next == '"' || next == '\'' {
            let quote = chars.next().expect("peeked quote");
            let mut closed = false;
            while let Some(ch) = chars.next() {
                if ch == quote {
                    closed = true;
                    break;
                }
                if ch == '\\' && chars.peek() == Some(&quote) {
                    part.push(chars.next().expect("peeked escaped quote"));
                } else {
                    part.push(ch);
                }
            }
            if !closed {
                anyhow::bail!("Unterminated quoted argument");
            }
        } else {
            while let Some(&ch) = chars.peek() {
                if ch.is_whitespace() {
                    break;
                }
                part.push(ch);
                chars.next();
            }
        }
        parts.push(part);
    }

    Ok(parts)
}

#[cfg(test)]
mod tests {
    use super::split_command_line;

    #[test]
    fn parses_quoted_target_names_and_regular_arguments() {
        let parts =
            split_command_line(r#"query-table xmmmaster "NGC 1300" 40 ra,dec products"#).unwrap();

        assert_eq!(
            parts,
            [
                "query-table",
                "xmmmaster",
                "NGC 1300",
                "40",
                "ra,dec",
                "products"
            ]
        );
    }

    #[test]
    fn rejects_unterminated_quotes() {
        assert!(split_command_line(r#"query-table xmmmaster "NGC 1300"#).is_err());
    }
}
