use std::env;
use std::process::Command;

use anyhow::{ensure, Result};

/**
 * Run a command string and return the trimmed stdout.
 *
 * ```no_run
 * let output = run! { "kstat -p foo::bar" }?;
 * ```
 */
#[macro_export]
macro_rules! run {
    ( $s:expr ) => {{
        let args: Vec<_> = $s.split_whitespace().collect();
        $crate::util::run(&args)
    }};
}

/**
 * Run a command and return the trimmed stdout.
 *
 * ```no_run
 * let output = run(&["kstat", "-p", "foo::bar"])?;
 * ```
 */
pub fn run(args: &[&str]) -> Result<String> {
    let output = Command::new(args[0]).args(&args[1..]).output()?;
    ensure!(output.status.success(), "exec failed: {}", args.join(" "));
    let s = String::from_utf8(output.stdout)?.trim().to_string();
    Ok(s)
}

/**
 * Replace color codes with the ansi string to colorize the output.
 *
 * - $(c0) -> reset color/formatting
 * - $(c1) -> orange color
 * - $(c2) -> dim gray
 */
pub fn colorize(s: &str) -> String {
    if should_colorize() {
        s.replace("$(c0)", "\x1B[0m")
            .replace("$(c1)", "\x1B[0m\x1B[38;5;208m")
            .replace("$(c2)", "\x1B[0m\x1B[38;5;8m")
    } else {
        s.replace("$(c0)", "").replace("$(c1)", "").replace("$(c2)", "")
    }
}

/**
 * Check if we should emit color.
 */
fn should_colorize() -> bool {
    env::var_os("NO_COLOR").is_none() && nix::unistd::isatty(1).unwrap_or(false)
}
