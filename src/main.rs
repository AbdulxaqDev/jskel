use std::io::{self, IsTerminal, Read, Write};
use std::process::ExitCode;

mod cli;
mod clipboard;
mod filter;
mod json;
mod skel;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("error: {msg}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), String> {
    match cli::parse()? {
        cli::ParsedArgs::Help => {
            cli::print_help();
            Ok(())
        }
        cli::ParsedArgs::Version => {
            cli::print_version();
            Ok(())
        }
        cli::ParsedArgs::Run(args) => process(args),
    }
}

fn process(args: cli::Args) -> Result<(), String> {
    let raw = read_input(&args.input)?;

    let mut value = json::parse(&raw).map_err(|e| e.to_string())?;

    if let Some(keys) = &args.pick {
        value = filter::pick(value, keys);
    }
    if let Some(keys) = &args.omit {
        value = filter::omit(value, keys);
    }

    let value = skel::skeletonize(value, args.strategy);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let use_color = !args.no_color && out.is_terminal();

    let opts = json::WriteOpts {
        indent: args.indent,
        sort_keys: args.sort_keys,
        color: use_color,
    };
    let rendered = json::to_string(&value, opts);

    if args.copy {
        // Copy the un-colored version. ANSI codes in the clipboard would be
        // useless and confusing.
        let plain_opts = json::WriteOpts { color: false, ..opts };
        let plain = json::to_string(&value, plain_opts);
        clipboard::copy(&plain).map_err(|e| format!("clipboard: {e}"))?;
    }

    out.write_all(rendered.as_bytes())
        .map_err(|e| e.to_string())?;
    out.write_all(b"\n").map_err(|e| e.to_string())?;
    Ok(())
}

fn read_input(src: &cli::InputSource) -> Result<String, String> {
    match src {
        cli::InputSource::Inline(s) => Ok(s.clone()),
        cli::InputSource::File(p) => std::fs::read_to_string(p)
            .map_err(|e| format!("cannot read {p}: {e}")),
        cli::InputSource::Stdin => {
            let stdin = io::stdin();
            if stdin.is_terminal() {
                return Err(
                    "no input provided. Run `jskel --help` for usage.".into(),
                );
            }
            let mut buf = String::new();
            stdin
                .lock()
                .read_to_string(&mut buf)
                .map_err(|e| e.to_string())?;
            Ok(buf)
        }
    }
}
