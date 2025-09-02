use clap::{Arg, Command};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::process;

/// JSONL input format for CLI
#[derive(Debug, Deserialize)]
struct JsonlInput {
    pos: [i32; 3],
    text: String,
}

/// Enhanced error output for CLI
#[derive(Debug, Serialize)]
struct CliError {
    error: String,
    tuple_index: Option<usize>,
    statement_index: Option<usize>,
}

fn main() {
    let matches = Command::new("insign-cli")
        .version("0.1.0")
        .about("Compiles Insign DSL from JSONL input to regions+metadata JSON")
        .arg(
            Arg::new("input")
                .help("Input JSONL file (stdin if not provided)")
                .value_name("FILE")
                .index(1)
        )
        .arg(
            Arg::new("pretty")
                .long("pretty")
                .short('p')
                .help("Pretty-print JSON output")
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();

    // Read input from file or stdin
    let input_reader: Box<dyn BufRead> = match matches.get_one::<String>("input") {
        Some(filename) => {
            match File::open(filename) {
                Ok(file) => Box::new(BufReader::new(file)),
                Err(e) => {
                    eprintln!("Error reading file '{}': {}", filename, e);
                    process::exit(1);
                }
            }
        }
        None => Box::new(io::stdin().lock()),
    };

    // Parse JSONL input
    let units = match parse_jsonl_input(input_reader) {
        Ok(units) => units,
        Err(e) => {
            eprintln!("Error parsing JSONL input: {}", e);
            process::exit(1);
        }
    };

    // Compile using the insign library
    match insign::compile(&units) {
        Ok(dsl_map) => {
            // Output compiled result to stdout
            let json_output = if matches.get_flag("pretty") {
                serde_json::to_string_pretty(&dsl_map)
            } else {
                serde_json::to_string(&dsl_map)
            };

            match json_output {
                Ok(json) => {
                    println!("{}", json);
                    process::exit(0);
                }
                Err(e) => {
                    eprintln!("Error serializing output: {}", e);
                    process::exit(1);
                }
            }
        }
        Err(compile_error) => {
            // Format error with diagnostic information
            let cli_error = format_compile_error(&compile_error);
            
            // Output error as JSON to stderr for structured processing
            if let Ok(error_json) = serde_json::to_string_pretty(&cli_error) {
                eprintln!("{}", error_json);
            } else {
                // Fallback to simple error message
                eprintln!("Compilation error: {}", compile_error);
            }
            
            process::exit(2);
        }
    }
}

/// Parse JSONL input into units format expected by compiler
fn parse_jsonl_input(reader: Box<dyn BufRead>) -> Result<Vec<([i32; 3], String)>, Box<dyn std::error::Error>> {
    let mut units = Vec::new();
    
    for (line_num, line) in reader.lines().enumerate() {
        let line = line?;
        
        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }
        
        // Parse JSON line
        let input: JsonlInput = serde_json::from_str(&line)
            .map_err(|e| format!("Line {}: Invalid JSON: {}", line_num + 1, e))?;
        
        // Validate position array
        if input.pos.len() != 3 {
            return Err(format!("Line {}: 'pos' must be an array of exactly 3 integers", line_num + 1).into());
        }
        
        units.push((input.pos, input.text));
    }
    
    Ok(units)
}

/// Format compilation error for CLI output with enhanced diagnostics
fn format_compile_error(error: &insign::Error) -> CliError {
    use insign::Error::*;
    
    match error {
        Parser(parse_err) => {
            CliError {
                error: format!("Parse error: {}", parse_err),
                tuple_index: None,
                statement_index: None,
            }
        }
        NotImplemented => {
            CliError {
                error: "Feature not implemented yet".to_string(),
                tuple_index: None,
                statement_index: None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_parse_jsonl_input_valid() {
        let input = "{\"pos\": [10, 64, 10], \"text\": \"@rc([0,0,0],[3,2,1])\"}
{\"pos\": [0, 64, 0], \"text\": \"#doc.label=\\\"test\\\"\"}";
        
        let reader = Box::new(Cursor::new(input));
        let result = parse_jsonl_input(reader).unwrap();
        
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, [10, 64, 10]);
        assert_eq!(result[0].1, "@rc([0,0,0],[3,2,1])");
        assert_eq!(result[1].0, [0, 64, 0]);
        assert_eq!(result[1].1, r#"#doc.label="test""#);
    }

    #[test]
    fn test_parse_jsonl_input_invalid_json() {
        let input = "{\"pos\": [10, 64, 10], \"text\": \"@rc([0,0,0],[3,2,1])\"}
invalid json line";
        
        let reader = Box::new(Cursor::new(input));
        let result = parse_jsonl_input(reader);
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Line 2: Invalid JSON"));
    }

    #[test]
    fn test_parse_jsonl_input_empty_lines() {
        let input = "{\"pos\": [10, 64, 10], \"text\": \"@rc([0,0,0],[3,2,1])\"}

{\"pos\": [0, 64, 0], \"text\": \"#doc.label=\\\"test\\\"\"}";        
        let reader = Box::new(Cursor::new(input));
        let result = parse_jsonl_input(reader).unwrap();
        
        assert_eq!(result.len(), 2);
    }
}
