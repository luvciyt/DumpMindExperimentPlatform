use crate::parse::report::CrashReport;
use anyhow::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use std::fmt;
use thiserror::Error;

#[derive(Debug)]
pub enum CompilerType {
    GCC,
    CLANG,
}

// impl Display trait for Enum::CompilerType
impl fmt::Display for CompilerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompilerType::GCC => write!(f, "gcc"),
            CompilerType::CLANG => write!(f, "clang"),
        }
    }
}

#[derive(Debug)]
pub struct Compiler {
    pub compiler_type: CompilerType,
    pub major: usize,
    pub minor: usize,
    pub patch: usize,
}

// self defined error for compiler
#[derive(Debug, Error)]
pub enum ParseCompilerError {
    #[error("No crash data found in the report")]
    NoCrashData,
    #[error("Compiler description string does not match the expected format")]
    FormatNotMatched,
    #[error("Version string '{0}' is not in major.minor.patch format")]
    VersionFormat(String),
    #[error("Unknown compiler type found: {0}")]
    UnknownCompiler(String),
}

pub fn select_compiler(report: &CrashReport) -> Result<Compiler> {
    let compiler_str = report.crashes.first().unwrap().compiler_description.clone();
    static RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^(?P<name>gcc|clang) \(.*?\) (?P<version>[\d.-]+)").unwrap());

    let captures = RE
        .captures(&compiler_str)
        .ok_or(ParseCompilerError::FormatNotMatched)?;

    let compiler_type = match captures.name("name").unwrap().as_str() {
        "gcc" => CompilerType::GCC,
        "clang" => CompilerType::CLANG,
        other => anyhow::bail!(ParseCompilerError::UnknownCompiler(other.to_string())),
    };

    // Parse the version string into major, minor, and patch
    let version_str = captures.name("version").unwrap().as_str();
    let mut parts = version_str.split('.');

    let major_str = parts
        .next()
        .ok_or_else(|| ParseCompilerError::VersionFormat(version_str.to_string()))?;
    let minor_str = parts
        .next()
        .ok_or_else(|| ParseCompilerError::VersionFormat(version_str.to_string()))?;
    let patch_str = parts
        .next()
        .ok_or_else(|| ParseCompilerError::VersionFormat(version_str.to_string()))?;

    // Note the change to parse into `usize`
    let major = major_str
        .parse::<usize>()
        .map_err(|_| ParseCompilerError::VersionFormat(version_str.to_string()))?;
    let minor = minor_str
        .parse::<usize>()
        .map_err(|_| ParseCompilerError::VersionFormat(version_str.to_string()))?;
    let patch = patch_str
        .parse::<usize>()
        .map_err(|_| ParseCompilerError::VersionFormat(version_str.to_string()))?;

    let compiler = Compiler {
        compiler_type,
        major,
        minor,
        patch,
    };

    Ok(compiler)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::compiler::CompilerType::GCC;
    use crate::parse::parse::parse_file;

    #[test]
    fn test_parse_compiler() {
        let compiler = Compiler {
            compiler_type: GCC,
            major: 10,
            minor: 5,
            patch: 1,
        };

        assert_eq!(compiler.compiler_type.to_string(), "gcc".to_string())
    }

    #[test]
    fn test_select_compiler() {
        let crash_report =
            parse_file("datasets/0b6b2d6d6cefa8b462930e55be699efba635788f.json").unwrap();
        let compiler = select_compiler(&crash_report).unwrap();
        assert_eq!(compiler.compiler_type.to_string(), "gcc".to_string());
        assert_eq!(compiler.major, 10);
        assert_eq!(compiler.minor, 2);
        assert_eq!(compiler.patch, 1);
    }
}
