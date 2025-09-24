//! Command-line entry point for the PDF password brute-force utility.

use clap::{ArgAction, Parser};
use indicatif::{ProgressBar, ProgressStyle};
use lopdf::Document;
use lopdf::Error as LopdfError;
use lopdf::encryption::DecryptionError;
use rayon::{ThreadPool, ThreadPoolBuilder, prelude::*};

use std::process::exit;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Instant;

/// Extended symbol characters that can be included in the brute-force alphabet.
const SYMBOLS: &[char] = &[
    '!', '@', '#', '$', '%', '^', '&', '*', '(', ')', '-', '_', '=', '+', '[', ']', '{', '}', '|',
    '\\', ':', ';', '"', '\'', ',', '.', '<', '>', '/', '?', '`', '~',
];

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
/// CLI arguments supported by pdf-pwbf.
struct Cli {
    /// Path to the password-protected PDF
    #[arg(short = 'i', long = "input", value_name = "PDF", required = true)]
    input: String,

    /// Minimum password length to brute-force
    #[arg(long = "min", default_value_t = 1)]
    min: usize,

    /// Maximum password length to brute-force
    #[arg(long = "max", default_value_t = 8)]
    max: usize,

    /// Include digits in the candidate alphabet
    #[arg(short = 'd', long = "digit", action = ArgAction::SetTrue)]
    digit: bool,

    /// Include alphabetic characters in the candidate alphabet
    #[arg(short = 'a', long = "alphabet", action = ArgAction::SetTrue)]
    alphabet: bool,

    /// Include common symbols in the candidate alphabet
    #[arg(short = 's', long = "symbol", action = ArgAction::SetTrue)]
    symbol: bool,

    /// Number of worker threads to use for brute-force attempts
    #[arg(short = 't', long = "threads", default_value_t = 1)]
    threads: usize,
}

/// Entrypoint that validates flags, prepares shared state, and triggers brute-force search.
fn main() {
    let args = Cli::parse();

    if args.min > args.max {
        eprintln!(
            "Error: --min ({}) must be less than or equal to --max ({}).",
            args.min, args.max
        );
        exit(1);
    }

    if args.threads == 0 {
        eprintln!("Error: --threads must be at least 1.");
        exit(1);
    }

    let mut charset: Vec<char> = Vec::new();
    if args.digit {
        charset.extend(['0', '1', '2', '3', '4', '5', '6', '7', '8', '9']);
    }
    if args.alphabet {
        charset.extend(('a'..='z').chain('A'..='Z'));
    }
    if args.symbol {
        charset.extend(SYMBOLS.iter().copied());
    }

    if charset.is_empty() {
        eprintln!("Error: provide at least one candidate set via --digit and/or --alphabet.");
        exit(1);
    }

    let start = Instant::now();

    println!("PDF: {}", args.input);
    println!("Min length: {}", args.min);
    println!("Max length: {}", args.max);
    println!("Charset size: {}", charset.len());

    let charset_len = charset.len();
    let mut total_attempts: u128 = 0;
    for len in args.min..=args.max {
        let combos = (charset_len as u128)
            .checked_pow(len as u32)
            .unwrap_or_else(|| {
                eprintln!(
                    "Error: search space too large (overflow when computing {}-length candidates).",
                    len
                );
                exit(1);
            });
        total_attempts = total_attempts.checked_add(combos).unwrap_or_else(|| {
            eprintln!("Error: total search space exceeds supported size.");
            exit(1);
        });
    }

    if total_attempts == 0 {
        println!("Nothing to brute-force: empty search space.");
        println!("Elapsed: {:.2?}", start.elapsed());
        return;
    }

    if total_attempts > u64::MAX as u128 {
        eprintln!("Error: total search space too large to track with progress bar.");
        exit(1);
    }

    let progress = ProgressBar::new(total_attempts as u64);
    progress.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} {percent:>3}% [{wide_bar:.cyan/blue}] ({eta} remaining)",
        )
        .expect("valid progress template"),
    );

    let template_doc = match Document::load(&args.input) {
        Ok(doc) => Arc::new(doc),
        Err(e) => {
            eprintln!("Failed to load PDF: {}", e);
            exit(1);
        }
    };

    if !template_doc.is_encrypted() {
        println!("Not Encrypted");
        println!("Elapsed: {:.2?}", start.elapsed());
        return;
    }

    let thread_pool = ThreadPoolBuilder::new()
        .num_threads(args.threads)
        .build()
        .unwrap_or_else(|e| {
            eprintln!("Failed to initialize thread pool: {}", e);
            exit(1);
        });

    let found_flag = Arc::new(AtomicBool::new(false));

    let result = brute_force(
        &thread_pool,
        &template_doc,
        &charset,
        args.min,
        args.max,
        &progress,
        &found_flag,
    );

    progress.finish_and_clear();

    let elapsed = start.elapsed();

    match result {
        Ok(Some(password)) => {
            println!("Password found: {}", password);

            let mut doc = (*template_doc).clone();

            match doc.decrypt(&password) {
                Ok(()) => println!("Done"),
                Err(e) => {
                    eprintln!(
                        "Unexpected error re-opening PDF with discovered password: {}",
                        e
                    );
                    println!("Elapsed: {:.2?}", elapsed);
                    exit(3);
                }
            }

            println!("Elapsed: {:.2?}", elapsed);
        }
        Ok(None) => {
            println!("Password not found in provided search space.");
            println!("Elapsed: {:.2?}", elapsed);
            exit(2);
        }
        Err(e) => {
            eprintln!("Decryption error: {}", e);
            println!("Elapsed: {:.2?}", elapsed);
            exit(3);
        }
    }
}

/// Iterate over candidate lengths and delegate password search per length.
fn brute_force(
    pool: &ThreadPool,
    doc: &Arc<Document>,
    charset: &[char],
    min_len: usize,
    max_len: usize,
    progress: &ProgressBar,
    found_flag: &Arc<AtomicBool>,
) -> Result<Option<String>, LopdfError> {
    for target_len in min_len..=max_len {
        if let Some(found) =
            brute_force_length(pool, doc, charset, target_len, progress, found_flag)?
        {
            return Ok(Some(found));
        }
    }

    Ok(None)
}

/// Exhaustively tries all passwords of `target_len`, returning the first success.
fn brute_force_length(
    pool: &ThreadPool,
    doc: &Arc<Document>,
    charset: &[char],
    target_len: usize,
    progress: &ProgressBar,
    found_flag: &Arc<AtomicBool>,
) -> Result<Option<String>, LopdfError> {
    let charset_len = charset.len() as u128;
    let combos = charset_len
        .checked_pow(target_len as u32)
        .expect("search space validated earlier");

    if combos == 0 {
        return Ok(None);
    }

    let combos_u64 = combos as u64;

    let doc_clone = Arc::clone(doc);
    let found_flag = Arc::clone(found_flag);
    // Chunk size balances worker utilization and ensures progress updates stay responsive.
    let chunk_size = std::cmp::max(
        1000_u64,
        combos_u64 / (pool.current_num_threads() as u64 * 4),
    );
    let num_chunks = (combos_u64 + chunk_size - 1) / chunk_size;

    let search = pool.install(|| {
        (0..num_chunks).into_par_iter().find_map_any(|chunk_idx| {
            if found_flag.load(Ordering::Acquire) {
                return None;
            }

            let start = chunk_idx * chunk_size;
            let end = std::cmp::min(start + chunk_size, combos_u64);
            let mut buffer = String::new();

            for index in start..end {
                if found_flag.load(Ordering::Acquire) {
                    return None;
                }

                index_to_password_with_buffer(index, target_len, charset, &mut buffer);
                let attempt = try_password(&doc_clone, &buffer);
                progress.inc(1);

                match attempt {
                    Ok(true) => {
                        found_flag.store(true, Ordering::Release);
                        return Some(Ok(buffer.clone()));
                    }
                    Ok(false) => continue,
                    Err(e) => return Some(Err(e)),
                }
            }
            None
        })
    });

    match search {
        Some(Ok(password)) => Ok(Some(password)),
        Some(Err(e)) => Err(e),
        None => Ok(None),
    }
}

/// Translate a numeric index into a password string using the selected charset.
fn index_to_password_with_buffer(
    mut index: u64,
    target_len: usize,
    charset: &[char],
    buffer: &mut String,
) {
    buffer.clear();
    if target_len == 0 {
        return;
    }

    buffer.reserve(target_len);
    let base = charset.len() as u64;
    let mut chars_reversed = Vec::with_capacity(target_len);

    for _ in 0..target_len {
        let char_index = (index % base) as usize;
        chars_reversed.push(charset[char_index]);
        index /= base;
    }

    for &ch in chars_reversed.iter().rev() {
        buffer.push(ch);
    }
}

/// Authenticate a candidate password against the encrypted document.
fn try_password(doc: &Arc<Document>, password: &str) -> Result<bool, LopdfError> {
    match doc.authenticate_password(password) {
        Ok(()) => Ok(true),
        Err(LopdfError::Decryption(DecryptionError::IncorrectPassword)) => Ok(false),
        Err(e) => Err(e),
    }
}
