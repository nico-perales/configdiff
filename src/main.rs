//! The configdiff command-line interface.

use std::io::{IsTerminal, Read, Write};
use std::path::Path as FsPath;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};

use configdiff::render::{json, pretty};
use configdiff::{ArrayStrategy, ChangeKind, DiffOptions, Format, diff, parse_auto};

#[derive(Debug, Parser)]
#[command(name = "configdiff", version, about, long_about = None)]
#[allow(clippy::struct_excessive_bools)]
struct Cli {
    /// The original file (use `-` for stdin).
    old: String,

    /// The updated file (use `-` for stdin).
    new: String,

    /// Force the format of both inputs.
    #[arg(short, long, value_enum)]
    format: Option<FormatArg>,

    /// Force the format of the OLD input (overrides `--format`).
    #[arg(long, value_enum)]
    old_format: Option<FormatArg>,

    /// Force the format of the NEW input (overrides `--format`).
    #[arg(long, value_enum)]
    new_format: Option<FormatArg>,

    /// Output format.
    #[arg(short, long, value_enum, default_value_t = OutputArg::Pretty)]
    output: OutputArg,

    /// When to colorize pretty output.
    #[arg(long, value_enum, default_value_t = ColorArg::Auto)]
    color: ColorArg,

    /// Ignore paths matching this glob (repeatable).
    #[arg(long = "ignore", value_name = "GLOB")]
    ignore: Vec<String>,

    /// Array diffing strategy.
    #[arg(long = "array", value_enum, default_value_t = ArrayArg::Lcs)]
    array: ArrayArg,

    /// Key field for matching array-of-table elements (repeatable). Implies keyed.
    #[arg(long = "array-key", value_name = "KEY")]
    array_key: Vec<String>,

    /// Treat integers and floats with equal value as equal (`1` == `1.0`).
    #[arg(long)]
    loose_numbers: bool,

    /// Absolute tolerance when comparing floating-point numbers.
    #[arg(long, value_name = "EPSILON")]
    float_tolerance: Option<f64>,

    /// Expand added/removed subtrees, reporting each leaf on its own line.
    #[arg(long)]
    expand: bool,

    /// Only exit non-zero on these change kinds (repeatable).
    #[arg(long = "fail-on", value_enum, value_name = "KIND")]
    fail_on: Vec<FailKind>,

    /// Suppress output; communicate only through the exit code.
    #[arg(short, long)]
    quiet: bool,

    /// Always exit 0, even when the documents differ.
    #[arg(long)]
    exit_zero: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FormatArg {
    Json,
    Toml,
    Yaml,
    Ini,
    Env,
}

impl From<FormatArg> for Format {
    fn from(f: FormatArg) -> Self {
        match f {
            FormatArg::Json => Format::Json,
            FormatArg::Toml => Format::Toml,
            FormatArg::Yaml => Format::Yaml,
            FormatArg::Ini => Format::Ini,
            FormatArg::Env => Format::Dotenv,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum FailKind {
    Added,
    Removed,
    Changed,
    TypeChanged,
}

impl FailKind {
    fn matches(self, kind: &ChangeKind) -> bool {
        matches!(
            (self, kind),
            (FailKind::Added, ChangeKind::Added { .. })
                | (FailKind::Removed, ChangeKind::Removed { .. })
                | (FailKind::Changed, ChangeKind::Changed { .. })
                | (FailKind::TypeChanged, ChangeKind::TypeChanged { .. })
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum OutputArg {
    Pretty,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ColorArg {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum ArrayArg {
    Lcs,
    Positional,
    Keyed,
}

impl From<ArrayArg> for ArrayStrategy {
    fn from(a: ArrayArg) -> Self {
        match a {
            ArrayArg::Lcs => ArrayStrategy::Lcs,
            ArrayArg::Positional => ArrayStrategy::Positional,
            ArrayArg::Keyed => ArrayStrategy::Keyed,
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(differs) => {
            if differs && !cli.exit_zero {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(err) => {
            eprintln!("configdiff: {err:#}");
            ExitCode::from(2)
        }
    }
}

// Runs the diff; returns Ok(true) when the documents differ.
fn run(cli: &Cli) -> Result<bool> {
    let old_src = read_input(&cli.old)?;
    let new_src = read_input(&cli.new)?;

    let old_fmt = resolve_format(cli.old_format.or(cli.format), &cli.old);
    let new_fmt = resolve_format(cli.new_format.or(cli.format), &cli.new);

    let old = parse_auto(&old_src, old_fmt)
        .with_context(|| format!("failed to parse {}", label(&cli.old)))?;
    let new = parse_auto(&new_src, new_fmt)
        .with_context(|| format!("failed to parse {}", label(&cli.new)))?;

    let opts = build_options(cli)?;
    let d = diff(&old, &new, &opts);

    if !cli.quiet {
        let rendered = match cli.output {
            OutputArg::Pretty => pretty::render(&d, use_color(cli.color)),
            OutputArg::Json => {
                let mut s = json::render(&d, true);
                s.push('\n');
                s
            }
        };
        let mut stdout = std::io::stdout().lock();
        if let Err(e) = stdout.write_all(rendered.as_bytes()) {
            if e.kind() != std::io::ErrorKind::BrokenPipe {
                return Err(e).context("failed to write output");
            }
        }
    }

    Ok(should_fail(&d, &cli.fail_on))
}

// No gates => any change counts; with gates => only listed kinds count.
fn should_fail(d: &configdiff::Diff, fail_on: &[FailKind]) -> bool {
    if fail_on.is_empty() {
        return !d.is_empty();
    }
    d.changes()
        .iter()
        .any(|c| fail_on.iter().any(|k| k.matches(&c.kind)))
}

fn build_options(cli: &Cli) -> Result<DiffOptions> {
    let strategy = if !cli.array_key.is_empty() && cli.array == ArrayArg::Lcs {
        ArrayStrategy::Keyed
    } else {
        cli.array.into()
    };

    let opts = DiffOptions::default()
        .numbers_loose(cli.loose_numbers)
        .float_tolerance(cli.float_tolerance)
        .array_strategy(strategy)
        .array_keys(cli.array_key.clone())
        .expand(cli.expand)
        .ignore(&cli.ignore)
        .context("invalid --ignore pattern")?;

    Ok(opts)
}

// Reads a file, or stdin when spec is `-`.
fn read_input(spec: &str) -> Result<String> {
    if spec == "-" {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .context("failed to read from stdin")?;
        Ok(buf)
    } else {
        std::fs::read_to_string(spec).with_context(|| format!("failed to read {spec}"))
    }
}

fn resolve_format(explicit: Option<FormatArg>, spec: &str) -> Option<Format> {
    explicit.map(Into::into).or_else(|| {
        if spec == "-" {
            None
        } else {
            Format::from_path(FsPath::new(spec))
        }
    })
}

fn label(spec: &str) -> String {
    if spec == "-" {
        "<stdin>".to_owned()
    } else {
        spec.to_owned()
    }
}

fn use_color(when: ColorArg) -> bool {
    match when {
        ColorArg::Always => true,
        ColorArg::Never => false,
        ColorArg::Auto => std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal(),
    }
}
