use std::collections::HashMap;
use std::env;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ParsedCommand {
    pub env_vars: HashMap<String, String>,
    pub executable: String,
    pub working_dir: String,
    pub arguments: Vec<String>,
}

/// Parses a complex Linux command into its components
pub fn parse_linux_command(command: &str) -> Result<ParsedCommand, String> {
    let tokens = tokenize_command(command)?;

    if tokens.is_empty() {
        return Err("Empty command".to_string());
    }

    let mut env_vars = HashMap::new();
    let mut i = 0;

    // Parse environment variables (VAR=value format)
    while i < tokens.len() {
        if let Some(eq_pos) = tokens[i].find('=') {
            let var_name = tokens[i][..eq_pos].to_string();
            let var_value = tokens[i][eq_pos + 1..].to_string();

            // Skip CWD as requested
            if var_name != "CWD" {
                env_vars.insert(var_name.clone(), var_value.clone());
            }

            // Store CWD separately for working directory determination
            if var_name == "CWD" {
                env_vars.insert("_INTERNAL_CWD".to_string(), var_value);
            }

            i += 1;
        } else {
            break;
        }
    }

    // No executable found after env vars
    if i >= tokens.len() {
        return Err("No executable found in command".to_string());
    }

    // Extract executable
    let executable = tokens[i].clone();
    i += 1;

    // Extract arguments
    let arguments: Vec<String> = tokens[i..].to_vec();

    // Determine working directory
    let working_dir = determine_working_directory(&env_vars, &executable)?;

    // Remove internal CWD marker if present
    env_vars.remove("_INTERNAL_CWD");

    Ok(ParsedCommand {
        env_vars,
        executable,
        working_dir,
        arguments,
    })
}

/// Tokenizes a command string, respecting quotes and escapes
fn tokenize_command(command: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut chars = command.chars().peekable();
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while let Some(ch) = chars.next() {
        match ch {
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            '\\' if !in_single_quote => {
                // Handle escape sequences
                if let Some(next_ch) = chars.next() {
                    match next_ch {
                        'n' => current_token.push('\n'),
                        't' => current_token.push('\t'),
                        'r' => current_token.push('\r'),
                        '\\' => current_token.push('\\'),
                        '"' => current_token.push('"'),
                        '\'' => current_token.push('\''),
                        ' ' => current_token.push(' '),
                        _ => {
                            current_token.push(next_ch);
                        }
                    }
                }
            }
            ' ' | '\t' if !in_single_quote && !in_double_quote => {
                // Whitespace outside quotes ends token
                if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
            }
            _ => {
                current_token.push(ch);
            }
        }
    }

    // Check for unclosed quotes
    if in_single_quote {
        return Err("Unclosed single quote".to_string());
    }
    if in_double_quote {
        return Err("Unclosed double quote".to_string());
    }

    // Add final token if any
    if !current_token.is_empty() {
        tokens.push(current_token);
    }

    Ok(tokens)
}

/// Determines the working directory based on the rules provided
fn determine_working_directory(env_vars: &HashMap<String, String>, executable: &str) -> Result<String, String> {
    // First priority: CWD environment variable
    if let Some(cwd) = env_vars.get("_INTERNAL_CWD") {
        return Ok(cwd.clone());
    }

    // Second priority: Parent directory if executable is a file path
    if executable.contains('/') {
        let path = Path::new(executable);
        if let Some(parent) = path.parent() {
            return Ok(parent.to_string_lossy().to_string());
        }
    }

    // Default: Current process working directory
    env::current_dir()
        .map(|path| path.to_string_lossy().to_string())
        .map_err(|e| format!("Failed to get current directory: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_command() {
        let cmd = "ls -la /home";
        let parsed = parse_linux_command(cmd).unwrap();

        assert_eq!(parsed.executable, "ls");
        assert_eq!(parsed.arguments, vec!["-la", "/home"]);
        assert!(parsed.env_vars.is_empty());
    }

    #[test]
    fn test_command_with_env_vars() {
        let cmd = "PATH=/usr/bin USER=test ls -la";
        let parsed = parse_linux_command(cmd).unwrap();

        assert_eq!(parsed.executable, "ls");
        assert_eq!(parsed.arguments, vec!["-la"]);
        assert_eq!(parsed.env_vars.get("PATH"), Some(&"/usr/bin".to_string()));
        assert_eq!(parsed.env_vars.get("USER"), Some(&"test".to_string()));
    }

    #[test]
    fn test_command_with_cwd() {
        let cmd = "CWD=/tmp ls";
        let parsed = parse_linux_command(cmd).unwrap();

        assert_eq!(parsed.executable, "ls");
        assert_eq!(parsed.working_dir, "/tmp");
        assert!(!parsed.env_vars.contains_key("CWD"));
    }

    #[test]
    fn test_quoted_arguments() {
        let cmd = r#"echo "hello world" 'single quotes' normal"#;
        let parsed = parse_linux_command(cmd).unwrap();

        assert_eq!(parsed.executable, "echo");
        assert_eq!(parsed.arguments, vec!["hello world", "single quotes", "normal"]);
    }

    #[test]
    fn test_executable_with_path() {
        let cmd = "/usr/bin/python script.py";
        let parsed = parse_linux_command(cmd).unwrap();

        assert_eq!(parsed.executable, "/usr/bin/python");
        assert_eq!(parsed.working_dir, "/usr/bin");
        assert_eq!(parsed.arguments, vec!["script.py"]);
    }

    #[test]
    fn test_complex_command() {
        let cmd = r#"VAR1="value with spaces" VAR2=value2 CWD=/workspace /usr/local/bin/app --config="my config.json" --verbose"#;
        let parsed = parse_linux_command(cmd).unwrap();

        assert_eq!(parsed.executable, "/usr/local/bin/app");
        assert_eq!(parsed.working_dir, "/workspace");
        assert_eq!(parsed.env_vars.get("VAR1"), Some(&"value with spaces".to_string()));
        assert_eq!(parsed.env_vars.get("VAR2"), Some(&"value2".to_string()));
        assert!(!parsed.env_vars.contains_key("CWD"));
        assert_eq!(parsed.arguments, vec!["--config=my config.json", "--verbose"]);
    }

    #[test]
    fn test_escaped_characters() {
        let cmd = r#"echo "line1\nline2" "quote\"inside""#;
        let parsed = parse_linux_command(cmd).unwrap();

        assert_eq!(parsed.executable, "echo");
        assert_eq!(parsed.arguments[0], "line1\nline2");
        assert_eq!(parsed.arguments[1], "quote\"inside");
    }
}