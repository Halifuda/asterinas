// SPDX-License-Identifier: MPL-2.0

//! An imperative testing framework for NixOS-based tests.
//!
//! # Core Concepts
//!
//! ## Test Registration
//!
//! Use the `#[nixos_test]` attribute to register test cases. The framework
//! automatically discovers and runs all registered tests.
//!
//! ## Session Interaction
//!
//! Tests are implemented by interacting with a [`Session`] object. The [`Session`] type
//! provides methods for executing commands and verifying output. It supports nested execution
//! contexts (containers, SSH, etc.) with automatic cleanup.
//!
//! See the [template crate](https://github.com/asterinas/asterinas/tree/main/test/nixos/common/template)
//! for a usage example.
//!
//! See the [project README](https://github.com/asterinas/asterinas/tree/main/test/nixos)
//! for complete documentation on creating and running test suites.

#![deny(unsafe_code)]

use std::{
    collections::HashMap,
    env, fmt, fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[doc(hidden)]
pub use inventory;
pub use nixos_test_macro::nixos_test;
pub use rexpect::reader::Regex;
use rexpect::{ReadUntil, session::PtySession};
pub use session::{BackgroundProcess, CommandCheck, Session, SessionDesc, ShutdownStrategy};

mod session;

const DEFAULT_BOOT_READY_MARKERS: [&str; 2] = [
    "[kernel] rootfs is ready",
    "<<< Asterinas NixOS Stage 2 >>>",
];
const DEFAULT_ROOT_SHELL_PROMPT: &str = "root@asterinas:";
const DEFAULT_SHUTDOWN_COMMAND: &str = "poweroff";
const DEFAULT_SHUTDOWN_STRATEGY: ShutdownStrategy = ShutdownStrategy::GuestCommand;
const MATCH_READ_SIZE: usize = 1;
const FAILURE_OUTPUT_TAIL_CHARS: usize = 4096;
const BOOT_MANIFEST_ENV: &str = "NIXOS_TEST_BOOT_MANIFEST";

/// An error returned by the NixOS test framework.
#[derive(Debug)]
pub enum Error {
    /// The PTY/process produced an underlying rexpect failure.
    Pty(rexpect::error::Error),
    /// The command output reached the prompt but did not contain the expected substring.
    UnexpectedOutput { expected: String, got: String },
    /// The framing marker was missing or unparsable.
    Protocol { reason: String, got: String },
    /// A command exited with a non-zero status.
    NonZeroExit { exit_status: i32, output: String },
    /// Polling for an expected state timed out.
    Timeout {
        expected: String,
        got: String,
        timeout: Duration,
    },
    /// An error collection that aggregates multiple errors.
    ///
    /// For each collected error, it keeps a name (in `String`)
    /// and a detailed error info (in `Error`).
    Aggregated {
        summary: String,
        collections: Vec<(String, Error)>,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pty(error) => write!(formatter, "{}", error),
            Self::UnexpectedOutput { expected, got } => write!(
                formatter,
                "Expected output containing '{}' but got '{}'",
                expected, got
            ),
            Self::Protocol { reason, got } => {
                write!(formatter, "Protocol error: {}. Output: '{}'", reason, got)
            }
            Self::NonZeroExit {
                exit_status,
                output,
            } => write!(
                formatter,
                "Command exited with status {}. Output: '{}'",
                exit_status, output
            ),
            Self::Timeout {
                expected,
                got,
                timeout,
            } => write!(
                formatter,
                "Timeout waiting for '{}'; got '{}' after {:?}",
                expected, got, timeout
            ),
            Self::Aggregated {
                summary,
                collections,
            } => {
                write!(formatter, "{} ({} error(s))", summary, collections.len())
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Pty(error) => Some(error),
            Self::UnexpectedOutput { .. }
            | Self::Protocol { .. }
            | Self::NonZeroExit { .. }
            | Self::Timeout { .. }
            | Self::Aggregated { .. } => None,
        }
    }
}

impl From<rexpect::error::Error> for Error {
    fn from(error: rexpect::error::Error) -> Self {
        match error {
            rexpect::error::Error::Timeout {
                expected,
                got,
                timeout,
            } => Self::Timeout {
                expected,
                got: truncate_output_for_error(&clean_output(&got)),
                timeout,
            },
            rexpect::error::Error::EOF {
                expected,
                got,
                exit_code,
            } => Self::Pty(rexpect::error::Error::EOF {
                expected,
                got: truncate_output_for_error(&clean_output(&got)),
                exit_code,
            }),
            other => Self::Pty(other),
        }
    }
}

/// A test case definition.
pub struct TestCase {
    pub name: &'static str,
    pub test_fn: fn(&mut Session) -> Result<(), Error>,
}

inventory::collect!(TestCase);

/// Generates the main function that runs all test cases.
#[macro_export]
macro_rules! nixos_test_main {
    () => {
        fn main() -> Result<(), Box<dyn std::error::Error>> {
            $crate::__nixos_test_main()
        }
    };
}

#[doc(hidden)]
pub fn __nixos_test_main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    // Check for --help flag
    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_help();
        return Ok(());
    }

    // Parse --qemu-cmd argument
    let qemu_cmd = parse_arg(&args, "--qemu-cmd").ok_or("Missing --qemu-cmd argument")?;

    // Parse optional --test argument
    let test_filter = parse_arg(&args, "--test");

    // Parse timeout from NIXOS_TEST_TIMEOUT environment variable
    let timeout_ms = env::var("NIXOS_TEST_TIMEOUT")
        .ok()
        .map(|v| parse_timeout(&v))
        .transpose()?
        .unwrap_or(600_000); // Default: 10 minutes

    let all_test_cases: Vec<&TestCase> = inventory::iter::<TestCase>().collect();

    if all_test_cases.is_empty() {
        return Err("No test cases found".into());
    }

    // Filter test cases if --test is specified
    let test_cases: Vec<&TestCase> = if let Some(ref filter) = test_filter {
        let filtered: Vec<&TestCase> = all_test_cases
            .into_iter()
            .filter(|tc| tc.name == filter)
            .collect();

        if filtered.is_empty() {
            return Err(format!("Test case '{}' not found", filter).into());
        }

        filtered
    } else {
        all_test_cases
    };

    if let Some(ref filter) = test_filter {
        println!("=== Running single test case: {} ===", filter);
    } else {
        println!("=== Found {} test case(s) ===", test_cases.len());
        for tc in &test_cases {
            println!("  - {}", tc.name);
        }
        println!();
    }

    let prompt_wait_start = Instant::now();
    let mut session = rexpect::spawn(&qemu_cmd, Some(timeout_ms)).map_err(Error::from)?;
    let boot_contract = load_boot_contract(&qemu_cmd)?;

    println!("--> Waiting for boot-ready marker...");
    let boot_marker = match wait_for_clean_marker(&mut session, &boot_contract.boot_ready_markers) {
        Ok(matched_marker) => matched_marker,
        Err(error) => {
            println!(
                "--> Boot-ready marker acquisition failed after {:?}",
                prompt_wait_start.elapsed()
            );
            return Err(Box::new(error));
        }
    };
    println!(
        "--> Boot-ready marker acquired after {:?}: {}",
        prompt_wait_start.elapsed(),
        boot_marker
    );

    println!("--> Waiting for interactive prompt...");
    let init_prompt = match wait_for_clean_marker(
        &mut session,
        std::slice::from_ref(&boot_contract.interactive_prompt),
    ) {
        Ok(matched_marker) => matched_marker,
        Err(error) => {
            println!(
                "--> Interactive prompt acquisition failed after {:?}",
                prompt_wait_start.elapsed()
            );
            return Err(Box::new(error));
        }
    };
    println!(
        "--> Interactive prompt acquired after {:?}: {}",
        prompt_wait_start.elapsed(),
        init_prompt
    );

    let desc = SessionDesc::new(init_prompt, "", boot_contract.shutdown_command)
        .shutdown_strategy(boot_contract.shutdown_strategy);
    let mut session = Session::new(desc, session);

    let mut passed = 0;
    let mut failed = 0;
    let mut failed_tests = Vec::new();

    for test_case in test_cases {
        println!("=== Running test case: {} ===", test_case.name);

        match (test_case.test_fn)(&mut session) {
            Ok(_) => {
                println!("✓ Test case '{}' passed\n", test_case.name);
                passed += 1;
            }
            Err(_) => {
                println!("✗ Test case '{}' failed\n", test_case.name);
                failed += 1;
                failed_tests.push(test_case.name);
            }
        }
    }

    println!("=== Test Summary ===");
    println!("Passed: {}", passed);
    println!("Failed: {}", failed);

    let res = if !failed_tests.is_empty() {
        println!("\nFailed tests:");
        for name in failed_tests {
            println!("  - {}", name);
        }

        Err("Some tests failed")
    } else {
        Ok(())
    };

    let shutdown_res = session.shutdown();

    res?;
    shutdown_res?;

    Ok(())
}

pub(crate) fn truncate_output_for_error(output: &str) -> String {
    const MAX_ERROR_OUTPUT_LEN: usize = 16 * 1024;

    if output.len() <= MAX_ERROR_OUTPUT_LEN {
        return output.to_string();
    }

    let mut start = output.len() - MAX_ERROR_OUTPUT_LEN;
    while !output.is_char_boundary(start) {
        start += 1;
    }

    let mut truncated = format!("... <truncated {} leading bytes of output>\n", start);
    truncated.push_str(&output[start..]);
    truncated
}

pub(crate) fn clean_output(output: &str) -> String {
    String::from_utf8_lossy(&strip_ansi_escapes::strip(output)).to_string()
}

/// Parses timeout string with units into milliseconds.
///
/// Supports formats:
/// - `<number>ms` - milliseconds
/// - `<number>s` - seconds
/// - `<number>min` - minutes
fn parse_timeout(timeout_str: &str) -> Result<u64, Box<dyn std::error::Error>> {
    let timeout_str = timeout_str.trim();

    if let Some(ms) = timeout_str.strip_suffix("ms") {
        return Ok(ms.trim().parse()?);
    }

    if let Some(s) = timeout_str.strip_suffix('s') {
        let seconds: u64 = s.trim().parse()?;
        return Ok(seconds * 1000);
    }

    if let Some(m) = timeout_str.strip_suffix("min") {
        let minutes: u64 = m.trim().parse()?;
        return Ok(minutes * 60000);
    }

    Err(format!(
        "Invalid timeout format '{}'. Use: <number>ms, <number>s, or <number>min",
        timeout_str
    )
    .into())
}

/// Parse command line argument in the form --flag <value>
fn parse_arg(args: &[String], flag: &str) -> Option<String> {
    for i in 0..args.len() {
        if args[i] == flag {
            return args.get(i + 1).cloned();
        }
    }
    None
}

fn print_help() {
    println!(
        "\
NixOS-Based Test Framework

USAGE:
    <test-binary> --qemu-cmd <COMMAND> [OPTIONS]

REQUIRED ARGUMENTS:
    --qemu-cmd <COMMAND>    Command to launch QEMU with the test environment

OPTIONS:
    --test <TEST_NAME>      Run only the specified test case
    -h, --help              Print this help message

ENVIRONMENT VARIABLES:
    NIXOS_TEST_TIMEOUT      Timeout for command execution
                            Supports: <number>ms, <number>s, <number>min
                            Examples: 300000ms, 300s, 5min
                            (default: 10min = 600000ms)

"
    );
}

struct BootContract {
    boot_ready_markers: Vec<String>,
    interactive_prompt: String,
    shutdown_command: String,
    shutdown_strategy: ShutdownStrategy,
}

impl Default for BootContract {
    fn default() -> Self {
        Self {
            boot_ready_markers: DEFAULT_BOOT_READY_MARKERS
                .iter()
                .map(|marker| marker.to_string())
                .collect(),
            interactive_prompt: DEFAULT_ROOT_SHELL_PROMPT.to_string(),
            shutdown_command: DEFAULT_SHUTDOWN_COMMAND.to_string(),
            shutdown_strategy: DEFAULT_SHUTDOWN_STRATEGY,
        }
    }
}

fn load_boot_contract(qemu_cmd: &str) -> Result<BootContract, Box<dyn std::error::Error>> {
    let Some(manifest_path) = boot_manifest_path(qemu_cmd) else {
        return Ok(BootContract::default());
    };

    if !manifest_path.is_file() {
        return Ok(BootContract::default());
    }

    let manifest = fs::read_to_string(&manifest_path)?;
    let mut entries = HashMap::new();

    for line in manifest.lines() {
        let trimmed_line = line.trim();
        if trimmed_line.is_empty() || trimmed_line.starts_with('[') || trimmed_line.starts_with('#')
        {
            continue;
        }

        let Some((key, value)) = trimmed_line.split_once('=') else {
            continue;
        };
        entries.insert(key.trim().to_string(), value.trim().to_string());
    }

    let mut contract = BootContract::default();

    if let Some(markers) = entries.get("boot_ready_markers") {
        let parsed_markers: Vec<String> = markers
            .split('|')
            .map(str::trim)
            .filter(|marker| !marker.is_empty())
            .map(ToString::to_string)
            .collect();
        if !parsed_markers.is_empty() {
            contract.boot_ready_markers = parsed_markers;
        }
    }

    if let Some(prompt) = entries.get("interactive_prompt") {
        if !prompt.is_empty() {
            contract.interactive_prompt = prompt.clone();
        }
    }

    if let Some(shutdown_command) = entries.get("shutdown_command") {
        contract.shutdown_command = shutdown_command.clone();
    }

    if let Some(shutdown_strategy) = entries.get("shutdown_strategy") {
        contract.shutdown_strategy = parse_shutdown_strategy(shutdown_strategy)?;
    }

    Ok(contract)
}

fn boot_manifest_path(qemu_cmd: &str) -> Option<PathBuf> {
    if let Ok(path) = env::var(BOOT_MANIFEST_ENV) {
        let trimmed_path = path.trim();
        if !trimmed_path.is_empty() {
            return Some(PathBuf::from(trimmed_path));
        }
    }

    let qemu_cmd_path = Path::new(qemu_cmd.trim());
    if !qemu_cmd_path.is_absolute() && qemu_cmd_path.components().count() == 0 {
        return None;
    }

    qemu_cmd_path
        .parent()
        .map(|parent| parent.join("manifest.txt"))
}

fn parse_shutdown_strategy(
    shutdown_strategy: &str,
) -> Result<ShutdownStrategy, Box<dyn std::error::Error>> {
    match shutdown_strategy {
        "guest-command" => Ok(ShutdownStrategy::GuestCommand),
        "host-terminate" => Ok(ShutdownStrategy::HostTerminate),
        _ => Err(format!("Unknown shutdown strategy '{}'.", shutdown_strategy).into()),
    }
}

fn sanitize_terminal_output(raw_output: &[u8]) -> String {
    let stripped_output = strip_ansi_escapes::strip(raw_output);
    let text = String::from_utf8_lossy(&stripped_output);

    text.chars()
        .filter(|ch| !matches!(ch, '\u{07}' | '\u{08}' | '\r'))
        .filter(|ch| !ch.is_control() || matches!(ch, '\n' | '\t'))
        .collect()
}

fn tail_chars(text: &str, max_chars: usize) -> String {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        return text.to_string();
    }

    let tail_start = text
        .char_indices()
        .nth(char_count - max_chars)
        .map(|(index, _)| index)
        .unwrap_or(0);
    text[tail_start..].to_string()
}

fn wait_for_clean_marker(session: &mut PtySession, markers: &[String]) -> Result<String, Error> {
    let mut raw_output = Vec::new();

    loop {
        match session
            .reader
            .read_until(&ReadUntil::NBytes(MATCH_READ_SIZE))
        {
            Ok((_, chunk)) => {
                if chunk.is_empty() {
                    continue;
                }

                raw_output.extend_from_slice(chunk.as_bytes());
                let cleaned_output = sanitize_terminal_output(&raw_output);

                if let Some(marker) = markers
                    .iter()
                    .find(|marker| cleaned_output.contains(marker.as_str()))
                {
                    return Ok(marker.clone());
                }
            }
            Err(error) => {
                let cleaned_output = sanitize_terminal_output(&raw_output);
                if !cleaned_output.trim().is_empty() {
                    println!(
                        "--> Sanitized output before failure (tail):\n{}",
                        tail_chars(&cleaned_output, FAILURE_OUTPUT_TAIL_CHARS)
                    );
                }
                return Err(Error::from(error));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_timeout;

    #[test]
    fn parse_timeout_supports_all_units() {
        assert_eq!(parse_timeout("300000ms").unwrap(), 300_000);
        assert_eq!(parse_timeout("300s").unwrap(), 300_000);
        assert_eq!(parse_timeout("5min").unwrap(), 300_000);
    }

    #[test]
    fn parse_timeout_reports_supported_units() {
        let error = parse_timeout("5m").unwrap_err().to_string();

        assert!(error.contains("<number>ms, <number>s, or <number>min"));
    }
}
