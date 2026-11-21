//! In-process benchmark: `cargo run --release --example bench -- <cv.json> <template> [max_pages] [iterations]`
//!
//! Three numbers per scenario:
//! - same CV again: every build repeats the previous one (Typst's caches are warm)
//! - distinct CVs: every build changes every entry's text, so nothing is reused but fonts
//! - batch: distinct CVs on all cores
use std::path::Path;
use std::time::Instant;

use cvforge::{Builder, Cv, Options, load_cv};
use rayon::prelude::*;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn variant(cv: &Cv, n: usize) -> Cv {
    let mut cv = cv.clone();
    cv.basics.summary = cv.basics.summary.map(|s| format!("{s} (v{n})"));
    for w in &mut cv.work {
        for h in &mut w.highlights {
            h.push_str(&format!(" [{n}]"));
        }
    }
    for p in &mut cv.projects {
        p.description = p.description.take().map(|d| format!("{d} [{n}]"));
    }
    cv
}

fn build(cv: Cv, base: &Path, options: &Options) -> usize {
    let mut b = Builder::new(cv, base, options).expect("builder");
    if let Some(dpi) = std::env::var("THUMB_DPI").ok().and_then(|d| d.parse().ok()) {
        b.set_thumbnail(dpi);
    }
    let (pdf, result) = b.pdf(None, None).expect("pdf");
    if std::env::var_os("CVFORGE_TIMING").is_some() {
        let t = result.timings;
        eprintln!(
            "  {} measure {:.1} ms | typeset {:.1} ms | pdf {:.1} ms | thumbnail {:.1} ms",
            t.measure_passes, t.measure_ms, t.typeset_ms, t.pdf_ms, t.thumbnail_ms
        );
    }
    pdf.len()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = Path::new(&args[1]);
    let template = args.get(2).cloned();
    let max_pages = args
        .get(3)
        .and_then(|m| m.parse().ok())
        .filter(|&m: &i64| m > 0);
    let iterations: usize = args.get(4).and_then(|n| n.parse().ok()).unwrap_or(50);
    let cv = load_cv(path).expect("cv");
    let base = path
        .parent()
        .expect("folder")
        .canonicalize()
        .expect("folder");
    let options = Options {
        template,
        max_pages,
        ..Options::default()
    };

    let t = Instant::now();
    let size = build(cv.clone(), &base, &options);
    let first = t.elapsed();

    let t = Instant::now();
    for _ in 0..iterations {
        build(cv.clone(), &base, &options);
    }
    let same = t.elapsed() / iterations as u32;

    let variants: Vec<Cv> = (0..iterations).map(|n| variant(&cv, n + 1000)).collect();
    let t = Instant::now();
    for v in variants {
        build(v, &base, &options);
    }
    let distinct = t.elapsed() / iterations as u32;

    let batch = 20 * rayon::current_num_threads();
    let variants: Vec<Cv> = (0..batch).map(|n| variant(&cv, n + 5000)).collect();
    let t = Instant::now();
    variants.into_par_iter().for_each(|v| {
        build(v, &base, &options);
    });
    let elapsed = t.elapsed();
    println!(
        "{:<26} first {:>7.1} ms | same CV {:>6.1} ms | distinct {:>6.1} ms | batch {:>6.0} CVs/s on {} threads | {} KB",
        format!(
            "{}{}",
            options.template.as_deref().unwrap_or("default"),
            max_pages.map(|m| format!(" max {m}")).unwrap_or_default()
        ),
        first.as_secs_f64() * 1e3,
        same.as_secs_f64() * 1e3,
        distinct.as_secs_f64() * 1e3,
        batch as f64 / elapsed.as_secs_f64(),
        rayon::current_num_threads(),
        size / 1024
    );
}
