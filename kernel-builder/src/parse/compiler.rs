use std::fmt;
use thiserror::Error;

#[derive(Debug)]
pub enum CompilerType {
    GCC,
    CLANG,
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

// impl Display trait for Enum::CompilerType
impl fmt::Display for CompilerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompilerType::GCC => write!(f, "gcc"),
            CompilerType::CLANG => write!(f, "clang"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::compiler::CompilerType::GCC;

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
}
