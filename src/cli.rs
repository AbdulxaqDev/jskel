//! Hand-rolled argument parsing. Small, predictable, and dep-free.
//! Supports both `--flag value` and `--flag=value` forms.

use std::env;

use crate::skel::Strategy;

pub struct Args {
    pub input: InputSource,
    pub copy: bool,
    pub indent: Option<usize>,
    pub sort_keys: bool,
    pub strategy: Strategy,
    pub pick: Option<Vec<String>>,
    pub omit: Option<Vec<String>>,
    pub no_color: bool,
}

pub enum InputSource {
    Inline(String),
    File(String),
    Stdin,
}

pub enum ParsedArgs {
    Run(Args),
    Help,
    Version,
}

pub fn parse() -> Result<ParsedArgs, String> {
    let argv: Vec<String> = env::args().skip(1).collect();
    parse_args(argv)
}

fn parse_args(argv: Vec<String>) -> Result<ParsedArgs, String> {
    let mut copy = false;
    let mut indent: Option<usize> = Some(2);
    let mut sort_keys = false;
    let mut strategy = Strategy::Default;
    let mut pick: Option<Vec<String>> = None;
    let mut omit: Option<Vec<String>> = None;
    let mut no_color = false;
    let mut force_stdin = false;
    let mut positional: Option<String> = None;
    let mut after_dashdash = false;

    let mut i = 0;
    while i < argv.len() {
        let arg = &argv[i];

        if after_dashdash {
            set_positional(&mut positional, arg)?;
            i += 1;
            continue;
        }

        match arg.as_str() {
            "--" => after_dashdash = true,
            "-h" | "--help" => return Ok(ParsedArgs::Help),
            "-V" | "--version" => return Ok(ParsedArgs::Version),
            "-c" | "--copy" => copy = true,
            "-m" | "--compact" => indent = None,
            "-s" | "--sort-keys" => sort_keys = true,
            "--no-color" => no_color = true,
            "--nulls" => strategy = Strategy::Nulls,
            "--types" => strategy = Strategy::Types,
            "--preserve-bool" => strategy = Strategy::PreserveBool,
            "-" => force_stdin = true,
            "--indent" => {
                i += 1;
                let v = argv
                    .get(i)
                    .ok_or_else(|| "--indent requires a value".to_string())?;
                indent = Some(parse_indent(v)?);
            }
            s if s.starts_with("--indent=") => {
                indent = Some(parse_indent(&s["--indent=".len()..])?);
            }
            "--pick" => {
                i += 1;
                let v = argv
                    .get(i)
                    .ok_or_else(|| "--pick requires a value".to_string())?;
                pick = Some(parse_keys(v));
            }
            s if s.starts_with("--pick=") => {
                pick = Some(parse_keys(&s["--pick=".len()..]));
            }
            "--omit" => {
                i += 1;
                let v = argv
                    .get(i)
                    .ok_or_else(|| "--omit requires a value".to_string())?;
                omit = Some(parse_keys(v));
            }
            s if s.starts_with("--omit=") => {
                omit = Some(parse_keys(&s["--omit=".len()..]));
            }
            s if is_flaglike(s) => return Err(format!("unknown flag: {s}")),
            _ => set_positional(&mut positional, arg)?,
        }

        i += 1;
    }

    let input = if force_stdin {
        InputSource::Stdin
    } else if let Some(p) = positional {
        if looks_like_inline_json(&p) {
            InputSource::Inline(p)
        } else if std::path::Path::new(&p).is_file() {
            InputSource::File(p)
        } else {
            InputSource::Inline(p)
        }
    } else {
        InputSource::Stdin
    };

    Ok(ParsedArgs::Run(Args {
        input,
        copy,
        indent,
        sort_keys,
        strategy,
        pick,
        omit,
        no_color,
    }))
}

fn set_positional(slot: &mut Option<String>, arg: &str) -> Result<(), String> {
    if slot.is_some() {
        return Err("only one input argument is allowed".into());
    }
    *slot = Some(arg.to_string());
    Ok(())
}

fn parse_indent(v: &str) -> Result<usize, String> {
    v.parse::<usize>()
        .map_err(|_| format!("invalid --indent value: {v}"))
}

fn parse_keys(s: &str) -> Vec<String> {
    s.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn is_flaglike(s: &str) -> bool {
    if !s.starts_with('-') || s.len() < 2 {
        return false;
    }
    // Allow inline JSON that begins with `-` (negative number scalars are
    // unusual at top level, but `-1` as the whole input shouldn't crash).
    let rest = &s[1..];
    !rest.starts_with(|c: char| c.is_ascii_digit() || c == '.')
}

fn looks_like_inline_json(s: &str) -> bool {
    let t = s.trim_start();
    t.starts_with('{') || t.starts_with('[') || t.starts_with('"')
}

pub fn print_help() {
    println!("{}", help_text());
}

pub fn print_version() {
    println!("jskel {}", env!("CARGO_PKG_VERSION"));
}

fn help_text() -> String {
    format!(
        "jskel {ver} — extract JSON structure by stripping values

USAGE:
    jskel [OPTIONS] [INPUT]
    cat file.json | jskel [OPTIONS]

INPUT:
    A JSON literal, a path to a .json file, or `-` for stdin.
    If omitted and stdin is piped, stdin is read.

OPTIONS:
    -c, --copy              Copy result to the system clipboard
    -m, --compact           Output compact JSON (no whitespace)
        --indent <N>        Indent with N spaces (default: 2)
    -s, --sort-keys         Sort object keys alphabetically

    --nulls                 Replace every scalar with null
    --types                 Replace scalars with their type name as a string
    --preserve-bool         Keep booleans as-is; strip other scalars

    --pick <KEYS>           Comma-separated keys to keep at top level
    --omit <KEYS>           Comma-separated keys to drop at top level

    --no-color              Disable ANSI color in terminal output
    -h, --help              Show this help
    -V, --version           Show version

EXAMPLES:
    jskel '{{\"name\":\"Brendan\",\"age\":40}}'
    jskel -c '{{\"x\":1,\"y\":[1,2]}}'
    cat payload.json | jskel --compact
    jskel --pick id,name users.json
    jskel --types '{{\"a\":1,\"b\":\"x\"}}'",
        ver = env!("CARGO_PKG_VERSION"),
    )
}
