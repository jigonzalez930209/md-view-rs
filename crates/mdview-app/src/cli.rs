//! Command-line parsing.

use std::path::PathBuf;

/// What the process should do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CliAction {
    /// Start the application, optionally opening files and a folder.
    Run {
        /// Files and folders to open.
        paths: Vec<PathBuf>,
    },
    /// Print usage.
    Help,
    /// Print the version.
    Version,
}

/// Parse process arguments, excluding the executable name.
pub fn parse(args: impl IntoIterator<Item = String>) -> Result<CliAction, String> {
    let mut paths = Vec::new();
    let mut after_separator = false;

    for arg in args {
        if !after_separator {
            match arg.as_str() {
                "--help" | "-h" => return Ok(CliAction::Help),
                "--version" | "-V" => return Ok(CliAction::Version),
                "--" => {
                    after_separator = true;
                    continue;
                }
                value if value.starts_with('-') => {
                    return Err(format!("unknown option: {value}"));
                }
                _ => {}
            }
        }

        paths.push(PathBuf::from(arg));
    }

    Ok(CliAction::Run { paths })
}

/// The help text printed by `--help`.
pub const HELP: &str = "\
MD View — a native Markdown and text editor

Usage:
  mdview [OPTIONS] [PATH…]

Arguments:
  PATH…       Open one or more files or folders

Options:
  -h, --help      Print this help
  -V, --version   Print the version
";

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(args: &[&str]) -> Result<CliAction, String> {
        parse(args.iter().map(|value| (*value).to_owned()))
    }

    #[test]
    fn parses_no_arguments() {
        assert_eq!(parse_str(&[]), Ok(CliAction::Run { paths: Vec::new() }));
    }

    #[test]
    fn parses_one_path_and_options() {
        assert_eq!(
            parse_str(&["notes.md"]),
            Ok(CliAction::Run {
                paths: vec![PathBuf::from("notes.md")]
            })
        );
        assert_eq!(parse_str(&["--help"]), Ok(CliAction::Help));
        assert_eq!(parse_str(&["--version"]), Ok(CliAction::Version));
    }

    #[test]
    fn supports_dash_paths_after_separator() {
        assert_eq!(
            parse_str(&["--", "-notes.md"]),
            Ok(CliAction::Run {
                paths: vec![PathBuf::from("-notes.md")]
            })
        );
    }

    #[test]
    fn accepts_multiple_paths_and_rejects_unknown_options() {
        assert_eq!(
            parse_str(&["one.md", "two.md"]),
            Ok(CliAction::Run {
                paths: vec![PathBuf::from("one.md"), PathBuf::from("two.md")]
            })
        );
        assert!(parse_str(&["--unknown"]).is_err());
    }
}
