//! `cv-forge build|validate|schema|templates|init`: the command line of cv-builder, with its
//! arguments, messages and exit codes (port of `cli.py`). The parser follows argparse's rules
//! and wording, since the recorded cases hold cv-builder's usage errors too.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use cvforge::model::{JSON_SCHEMA, load_cv};
use cvforge::{Builder, Options, Template, pyfmt};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// The program name in messages: cv-builder's, so output matches the original (DECISIONS.md).
const PROG: &str = "cv-builder";
const VERSION: &str = "0.1.0";
const STARTER: &str = include_str!("../../../original/src/cv_builder/assets/starter.json");

const MAIN_USAGE: &str =
    "usage: cv-builder [-h] [--version] {build,validate,schema,templates,init} ...";
const BUILD_USAGE: &str = "usage: cv-builder build [-h] [-o OUTPUT] [-t TEMPLATE] [--accent ACCENT]
                        [--page-size {A4,Letter}] [--html HTML]
                        [--max-pages MAX_PAGES] [-q]
                        cv";
const VALIDATE_USAGE: &str = "usage: cv-builder validate [-h] cv [cv ...]";
const SCHEMA_USAGE: &str = "usage: cv-builder schema [-h] [-o OUTPUT]";
const TEMPLATES_USAGE: &str = "usage: cv-builder templates [-h]";
const INIT_USAGE: &str = "usage: cv-builder init [-h] [--force] [path]";

const MAIN_HELP: &str = "
Build a PDF CV from a JSON file.

positional arguments:
  {build,validate,schema,templates,init}
    build               render a CV JSON file to PDF
    validate            check CV JSON files against the schema
    schema              print the JSON Schema of the CV format
    templates           list available templates
    init                write a starter CV JSON file

options:
  -h, --help            show this help message and exit
  --version             show program's version number and exit
";

/// An argparse usage error: usage, then `prog: error: message`, exit status 2.
struct UsageError {
    usage: &'static str,
    prog: String,
    message: String,
}

#[derive(Clone, Copy)]
struct Opt {
    short: Option<&'static str>,
    long: &'static str,
    value: Option<&'static str>,
    choices: Option<&'static [&'static str]>,
}

const fn flag(short: Option<&'static str>, long: &'static str) -> Opt {
    Opt {
        short,
        long,
        value: None,
        choices: None,
    }
}
const fn valued(short: Option<&'static str>, long: &'static str, metavar: &'static str) -> Opt {
    Opt {
        short,
        long,
        value: Some(metavar),
        choices: None,
    }
}

struct Parsed {
    values: Vec<(&'static str, Option<String>)>,
    positionals: Vec<String>,
    help: bool,
}

impl Parsed {
    fn get(&self, long: &str) -> Option<&str> {
        self.values
            .iter()
            .rev()
            .find(|(l, _)| *l == long)
            .and_then(|(_, v)| v.as_deref())
    }
    fn has(&self, long: &str) -> bool {
        self.values.iter().any(|(l, _)| *l == long)
    }
}

fn looks_negative(s: &str) -> bool {
    s.strip_prefix('-')
        .is_some_and(|r| !r.is_empty() && r.parse::<f64>().is_ok())
}

/// argparse's option matching: exact names, unique prefixes of long names, `--name=value`,
/// `-xVALUE`; values may not look like options; `--` ends options.
fn parse(
    args: &[String],
    opts: &[Opt],
    usage: &'static str,
    prog: &str,
) -> Result<Parsed, UsageError> {
    let fail = |message: String| UsageError {
        usage,
        prog: prog.to_string(),
        message,
    };
    let all: Vec<Opt> = std::iter::once(flag(Some("-h"), "--help"))
        .chain(opts.iter().copied())
        .collect();
    let name = |o: &Opt| match o.short {
        Some(s) => format!("{s}/{}", o.long),
        None => o.long.to_string(),
    };
    let mut out = Parsed {
        values: Vec::new(),
        positionals: Vec::new(),
        help: false,
    };
    let mut i = 0;
    let mut only_positionals = false;
    while i < args.len() {
        let arg = &args[i];
        i += 1;
        if only_positionals || !arg.starts_with('-') || arg == "-" || looks_negative(arg) {
            out.positionals.push(arg.clone());
            continue;
        }
        if arg == "--" {
            only_positionals = true;
            continue;
        }
        let (key, attached) = if arg.starts_with("--") {
            match arg.split_once('=') {
                Some((k, v)) => (k.to_string(), Some(v.to_string())),
                None => (arg.clone(), None),
            }
        } else if arg.len() > 2 && arg.is_char_boundary(2) {
            (arg[..2].to_string(), Some(arg[2..].to_string()))
        } else {
            (arg.clone(), None)
        };
        let found = all
            .iter()
            .find(|o| o.long == key || o.short == Some(key.as_str()))
            .copied()
            .or_else(|| {
                if !key.starts_with("--") {
                    return None;
                }
                let matches: Vec<&Opt> = all
                    .iter()
                    .filter(|o| o.long.starts_with(key.as_str()))
                    .collect();
                (matches.len() == 1).then(|| *matches[0])
            });
        let Some(opt) = found else {
            if key.starts_with("--") {
                let matches: Vec<&str> = all
                    .iter()
                    .filter(|o| o.long.starts_with(key.as_str()))
                    .map(|o| o.long)
                    .collect();
                if matches.len() > 1 {
                    return Err(fail(format!(
                        "ambiguous option: {key} could match {}",
                        matches.join(", ")
                    )));
                }
            }
            out.positionals.push(arg.clone());
            continue;
        };
        if opt.long == "--help" {
            out.help = true;
            continue;
        }
        let Some(_) = opt.value else {
            if attached.is_some() && arg.starts_with("--") {
                return Err(fail(format!(
                    "argument {}: ignored explicit argument {}",
                    name(&opt),
                    pyfmt::repr(attached.as_deref().unwrap_or(""))
                )));
            }
            out.values.push((opt.long, None));
            continue;
        };
        let value = match attached {
            Some(v) => v,
            None => match args.get(i) {
                Some(v) if !(v.starts_with('-') && v != "-" && !looks_negative(v)) => {
                    i += 1;
                    v.clone()
                }
                _ => {
                    return Err(fail(format!(
                        "argument {}: expected one argument",
                        name(&opt)
                    )));
                }
            },
        };
        if let Some(choices) = opt.choices
            && !choices.contains(&value.as_str())
        {
            return Err(fail(format!(
                "argument {}: invalid choice: {} (choose from {})",
                name(&opt),
                pyfmt::repr(&value),
                choices.join(", ")
            )));
        }
        out.values.push((opt.long, Some(value)));
    }
    Ok(out)
}

fn fail(message: &str, code: u8) -> u8 {
    eprintln!("error: {message}");
    code
}

fn cmd_build(p: &Parsed) -> Result<u8, UsageError> {
    let cv_arg = &p.positionals[0];
    let max_pages = match p.get("--max-pages") {
        Some(v) => Some(pyfmt::int(v).map_err(|_| UsageError {
            usage: BUILD_USAGE,
            prog: format!("{PROG} build"),
            message: format!(
                "argument --max-pages: invalid int value: {}",
                pyfmt::repr(v)
            ),
        })? as i64),
        None => None,
    };
    // Paths as pathlib reads them: '' is '.', './a//b' is 'a/b'.
    let shown = pyfmt::path_str(cv_arg);
    let cv_path = Path::new(&shown);
    let cv = match load_cv(cv_path) {
        Ok(cv) => cv,
        Err(e) => return Ok(fail(&e.report(&shown), 1)),
    };
    let output = match p.get("--output") {
        Some(o) => PathBuf::from(pyfmt::path_str(o)),
        None => cv_path.with_extension("pdf"),
    };
    let options = Options {
        template: p.get("--template").map(str::to_string),
        accent: p.get("--accent").map(str::to_string),
        page_size: p.get("--page-size").map(str::to_string),
        max_pages,
    };
    let base = cv_path
        .parent()
        .map(|d| {
            if d.as_os_str().is_empty() {
                Path::new(".")
            } else {
                d
            }
        })
        .unwrap_or(Path::new("."));
    let base = base.canonicalize().unwrap_or_else(|_| base.to_path_buf());
    let html = p.get("--html").map(|h| PathBuf::from(pyfmt::path_str(h)));
    let thumbnail_dpi = p
        .get("--thumbnail-dpi")
        .and_then(|d| d.parse::<f64>().ok())
        .unwrap_or(60.0);
    let wants_thumbnail = p.get("--thumbnail").is_some();
    let result = Builder::new(cv, &base, &options).and_then(|mut b| {
        if wants_thumbnail {
            b.set_thumbnail(thumbnail_dpi);
        }
        b.write_pdf(&output, html.as_deref(), None)
    });
    let result = match result {
        Ok(r) => r,
        Err(e) => return Ok(fail(&e.0, 1)),
    };
    if std::env::var_os("CVFORGE_TIMING").is_some() {
        let t = &result.timings;
        eprintln!(
            "timing: {} measure pass(es) {:.1} ms, typeset {:.1} ms, pdf {:.1} ms, thumbnail {:.1} ms",
            t.measure_passes, t.measure_ms, t.typeset_ms, t.pdf_ms, t.thumbnail_ms
        );
    }
    if let (Some(path), Some(png)) = (p.get("--thumbnail"), result.thumbnail.as_ref())
        && let Err(e) = write_bytes(Path::new(&pyfmt::path_str(path)), png)
    {
        return Ok(fail(&e, 1));
    }
    for note in &result.notes {
        eprintln!("warning: {note}");
    }
    if !p.has("--quiet") {
        let tightened = match &result.fit {
            Some(f) if f.tightness > 0.0 => format!(
                ", line spacing {} instead of {} to fit",
                pyfmt::fmt_g(f.line_height),
                pyfmt::fmt_g(f.designed_line_height)
            ),
            _ => String::new(),
        };
        let plural = if result.pages != 1 { "s" } else { "" };
        println!(
            "{} ({} page{plural}{tightened})",
            pyfmt::path_str(&output.to_string_lossy()),
            result.pages
        );
    }
    if let Some(f) = result.fit.as_ref().filter(|f| !f.fits) {
        return Ok(fail(
            &format!(
                "{} pages exceeds the limit of {}",
                result.pages, f.max_pages
            ),
            3,
        ));
    }
    Ok(0)
}

fn cmd_validate(p: &Parsed) -> u8 {
    let mut status = 0;
    for path in &p.positionals {
        let shown = pyfmt::path_str(path);
        match load_cv(Path::new(&shown)) {
            Ok(_) => println!("{shown}: ok"),
            Err(e) => {
                eprintln!("{}", e.report(&shown));
                status = 1;
            }
        }
    }
    status
}

fn write_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| {
        format!(
            "{}: {}",
            pyfmt::path_str(&path.to_string_lossy()),
            cvforge::model::io_message(&e)
        )
    })
}

fn write_text(path: &Path, text: &str) -> Result<(), String> {
    std::fs::write(path, text).map_err(|e| {
        format!(
            "{}: {}",
            pyfmt::path_str(&path.to_string_lossy()),
            cvforge::model::io_message(&e)
        )
    })
}

fn cmd_schema(p: &Parsed) -> u8 {
    match p.get("--output") {
        Some(out) => match write_text(Path::new(&pyfmt::path_str(out)), JSON_SCHEMA) {
            Ok(()) => {
                println!("{}", pyfmt::path_str(out));
                0
            }
            Err(e) => fail(&e, 1),
        },
        None => {
            print!("{JSON_SCHEMA}");
            0
        }
    }
}

fn cmd_templates() -> u8 {
    for t in Template::available() {
        println!("{:<10} {}", t.name, t.config.description);
        let order = if t.config.reading_order {
            "reads in order (safe for resume parsers)"
        } else {
            "for people; parsers mix up its text"
        };
        println!(
            "{:<10} columns: {}; {order}",
            "",
            t.column_names().join(", ")
        );
        if !t.config.aliases.is_empty() {
            println!("{:<10} formerly: {}", "", t.config.aliases.join(", "));
        }
    }
    0
}

fn cmd_init(p: &Parsed) -> u8 {
    let raw = p.positionals.first().map_or("cv.json", String::as_str);
    let shown = pyfmt::path_str(raw);
    let path = Path::new(&shown);
    if path.exists() && !p.has("--force") {
        return fail(&format!("{shown} exists (use --force to overwrite)"), 1);
    }
    let parent = path.parent().unwrap_or(Path::new(""));
    if !parent.as_os_str().is_empty() && std::fs::create_dir_all(parent).is_err() {
        return fail(&format!("cannot create {}", parent.display()), 1);
    }
    if let Err(e) = write_text(path, STARTER) {
        return fail(&e, 1);
    }
    println!("wrote {shown}");
    let schema_path = parent.join("cv.schema.json");
    if !schema_path.exists() {
        if let Err(e) = write_text(&schema_path, JSON_SCHEMA) {
            return fail(&e, 1);
        }
        println!("wrote {}", pyfmt::path_str(&schema_path.to_string_lossy()));
    }
    println!("edit {shown}, then run: {PROG} build {shown}");
    0
}

fn run(args: &[String]) -> Result<u8, UsageError> {
    let commands = ["build", "validate", "schema", "templates", "init"];
    let main_err = |message: String| UsageError {
        usage: MAIN_USAGE,
        prog: PROG.into(),
        message,
    };
    let i = 0;
    if i < args.len() && args[i].starts_with('-') && args[i] != "-" {
        match args[i].as_str() {
            "-h" | "--help" => {
                println!("{MAIN_USAGE}\n{MAIN_HELP}");
                return Ok(0);
            }
            "--version" => {
                println!("{PROG} {VERSION}");
                return Ok(0);
            }
            other => return Err(main_err(format!("unrecognized arguments: {other}"))),
        }
    }
    let Some(command) = args.get(i) else {
        return Err(main_err(
            "the following arguments are required: command".into(),
        ));
    };
    if !commands.contains(&command.as_str()) {
        return Err(main_err(format!(
            "argument command: invalid choice: {} (choose from {})",
            pyfmt::repr(command),
            commands.join(", ")
        )));
    }
    let rest = &args[i + 1..];
    let prog = format!("{PROG} {command}");
    let (usage, opts): (&'static str, Vec<Opt>) = match command.as_str() {
        "build" => (
            BUILD_USAGE,
            vec![
                valued(Some("-o"), "--output", "OUTPUT"),
                valued(Some("-t"), "--template", "TEMPLATE"),
                valued(None, "--accent", "ACCENT"),
                Opt {
                    short: None,
                    long: "--page-size",
                    value: Some("{A4,Letter}"),
                    choices: Some(&["A4", "Letter"]),
                },
                valued(None, "--html", "HTML"),
                valued(None, "--max-pages", "MAX_PAGES"),
                flag(Some("-q"), "--quiet"),
                // cv-forge's own: page 1 as a PNG too (not in cv-builder's usage text).
                valued(None, "--thumbnail", "PNG"),
                valued(None, "--thumbnail-dpi", "DPI"),
            ],
        ),
        "validate" => (VALIDATE_USAGE, vec![]),
        "schema" => (SCHEMA_USAGE, vec![valued(Some("-o"), "--output", "OUTPUT")]),
        "templates" => (TEMPLATES_USAGE, vec![]),
        _ => (INIT_USAGE, vec![flag(None, "--force")]),
    };
    let parsed = parse(rest, &opts, usage, &prog)?;
    if parsed.help {
        println!("{usage}");
        return Ok(0);
    }
    let required = |n: usize, what: &str| -> Result<(), UsageError> {
        if parsed.positionals.len() < n {
            return Err(UsageError {
                usage,
                prog: prog.clone(),
                message: format!("the following arguments are required: {what}"),
            });
        }
        Ok(())
    };
    let extra = |max: usize| -> Result<(), UsageError> {
        if parsed.positionals.len() > max {
            return Err(main_err(format!(
                "unrecognized arguments: {}",
                parsed.positionals[max..].join(" ")
            )));
        }
        Ok(())
    };
    match command.as_str() {
        "build" => {
            required(1, "cv")?;
            extra(1)?;
            cmd_build(&parsed)
        }
        "validate" => {
            required(1, "cv")?;
            Ok(cmd_validate(&parsed))
        }
        "schema" => {
            extra(0)?;
            Ok(cmd_schema(&parsed))
        }
        "templates" => {
            extra(0)?;
            Ok(cmd_templates())
        }
        _ => {
            extra(1)?;
            Ok(cmd_init(&parsed))
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let code = match run(&args) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("{}\n{}: error: {}", e.usage, e.prog, e.message);
            2
        }
    };
    let _ = std::io::stdout().flush();
    ExitCode::from(code)
}
