//! Command-line parsing.

use std::path::PathBuf;

/// What the process should do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CliAction {
    /// Start the application, optionally opening one file.
    Run {
        /// The file to open.
        path: Option<PathBuf>,
    },
    /// Print usage.
    Help,
    /// Print the version.
    Version,
}

/// Parse process arguments, excluding the executable name.
pub fn parse(args: impl IntoIterator<Item = String>) -> Result<CliAction, String> {
    let mut path = None;
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

        if path.is_some() {
            return Err("only one file can be opened at a time in this milestone".to_owned());
        }
        path = Some(PathBuf::from(arg));
    }

    Ok(CliAction::Run { path })
}

/// The help text printed by `--help`.
pub const HELP: &str = "\
MD View — a native Markdown and text editor

Usage:
  mdview [OPTIONS] [FILE]

Arguments:
  FILE        Open one Markdown or text file

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
        assert_eq!(parse_str(&[]), Ok(CliAction::Run { path: None }));
    }

    #[test]
    fn parses_one_path_and_options() {
        assert_eq!(
            parse_str(&["notes.md"]),
            Ok(CliAction::Run {
                path: Some(PathBuf::from("notes.md"))
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
                path: Some(PathBuf::from("-notes.md"))
            })
        );
    }

    #[test]
    fn rejects_multiple_paths_and_unknown_options() {
        assert!(parse_str(&["one.md", "two.md"]).is_err());
        assert!(parse_str(&["--unknown"]).is_err());
    }
}
