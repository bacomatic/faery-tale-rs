use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use reqwest::blocking::Client;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(name = "rag")]
#[command(about = "Local RAG utility (SQLite + Ollama embeddings)")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Index(IndexArgs),
    Query(QueryArgs),
    Ask(AskArgs),
}

#[derive(Args, Debug)]
struct CommonEmbeddingArgs {
    #[arg(long, default_value = "http://127.0.0.1:11434")]
    ollama_url: String,

    #[arg(long, default_value = "nomic-embed-text")]
    model: String,
}

#[derive(Args, Debug)]
struct IndexArgs {
    #[arg(long, default_value = ".")]
    root: PathBuf,

    #[arg(long, default_value = ".rag/index.sqlite")]
    db: PathBuf,

    #[arg(long, default_value_t = 40)]
    chunk_lines: usize,

    #[arg(long, default_value_t = 8)]
    overlap_lines: usize,

    #[arg(long)]
    reset: bool,

    #[arg(long = "ext", value_delimiter = ',', default_value = "rs,md,toml")]
    exts: Vec<String>,

    #[arg(long = "path")]
    paths: Vec<PathBuf>,

    #[command(flatten)]
    embed: CommonEmbeddingArgs,
}

#[derive(Args, Debug)]
struct QueryArgs {
    #[arg(long, default_value = ".rag/index.sqlite")]
    db: PathBuf,

    #[arg(long, default_value_t = 5)]
    top_k: usize,

    #[arg(long)]
    show_text: bool,

    #[arg(long)]
    code_first: bool,

    #[arg(long = "only-ext", value_delimiter = ',')]
    only_ext: Vec<String>,

    #[command(flatten)]
    embed: CommonEmbeddingArgs,

    query: String,
}

#[derive(Args, Debug)]
struct AskArgs {
    #[arg(long, default_value = ".rag/index.sqlite")]
    db: PathBuf,

    #[arg(long, default_value_t = 5)]
    top_k: usize,

    #[arg(long, default_value_t = 8000)]
    max_context_chars: usize,

    #[arg(long, default_value = "llama3.2")]
    llm_model: String,

    #[arg(long)]
    show_sources: bool,

    #[arg(long)]
    code_first: bool,

    #[arg(long = "only-ext", value_delimiter = ',')]
    only_ext: Vec<String>,

    #[command(flatten)]
    embed: CommonEmbeddingArgs,

    question: String,
}

#[derive(Debug)]
struct Chunk {
    path: String,
    start_line: usize,
    end_line: usize,
    text: String,
}

#[derive(Debug)]
struct StoredChunk {
    path: String,
    start_line: usize,
    end_line: usize,
    text: String,
    embedding: Vec<f32>,
}

#[derive(Serialize)]
struct OllamaEmbeddingRequest<'a> {
    model: &'a str,
    prompt: &'a str,
}

#[derive(Deserialize)]
struct OllamaEmbeddingResponse {
    embedding: Vec<f32>,
}

#[derive(Serialize)]
struct OllamaGenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaGenerateResponse {
    response: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Index(args) => run_index(args),
        Command::Query(args) => run_query(args),
        Command::Ask(args) => run_ask(args),
    }
}

fn run_index(args: IndexArgs) -> Result<()> {
    if args.chunk_lines == 0 {
        bail!("--chunk-lines must be > 0");
    }
    if args.overlap_lines >= args.chunk_lines {
        bail!("--overlap-lines must be less than --chunk-lines");
    }

    if let Some(parent) = args.db.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create db directory {}", parent.display()))?;
    }

    let conn = Connection::open(&args.db)
        .with_context(|| format!("failed to open sqlite db {}", args.db.display()))?;
    init_schema(&conn)?;

    if args.reset {
        conn.execute("DELETE FROM chunks", [])
            .context("failed to reset chunks table")?;
    }

    let normalized_exts = normalize_extensions(&args.exts);
    let files = if args.paths.is_empty() {
        collect_files(&args.root, &normalized_exts)?
    } else {
        collect_selected_files(&args.root, &normalized_exts, &args.paths)?
    };
    if files.is_empty() {
        println!("No matching files found under {}", args.root.display());
        return Ok(());
    }

    let client = Client::builder()
        .build()
        .context("failed to build HTTP client")?;

    let mut inserted = 0usize;
    for file in files {
        let chunks = chunk_file(&file, &args.root, args.chunk_lines, args.overlap_lines)?;
        for chunk in chunks {
            let embedding = embed_text(&client, &args.embed, &chunk.text)
                .with_context(|| format!("embedding failed for {}:{}", chunk.path, chunk.start_line))?;

            conn.execute(
                "INSERT INTO chunks(path, start_line, end_line, text, embedding_json)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    chunk.path,
                    chunk.start_line as i64,
                    chunk.end_line as i64,
                    chunk.text,
                    serde_json::to_string(&embedding)?
                ],
            )
            .context("failed inserting chunk")?;

            inserted += 1;
            if inserted % 50 == 0 {
                println!("Indexed {inserted} chunks...");
            }
        }
    }

    println!("Done. Indexed {inserted} chunks into {}", args.db.display());
    Ok(())
}

fn run_query(args: QueryArgs) -> Result<()> {
    let conn = Connection::open(&args.db)
        .with_context(|| format!("failed to open sqlite db {}", args.db.display()))?;

    let client = Client::builder()
        .build()
        .context("failed to build HTTP client")?;

    let only_ext = normalize_extensions(&args.only_ext);
    let scored = retrieve_scored_chunks(
        &conn,
        &client,
        &args.embed,
        &args.query,
        args.code_first,
        &only_ext,
    )?;

    for (idx, (score, chunk)) in scored.into_iter().take(args.top_k).enumerate() {
        println!(
            "{}. {:.4}  {}:{}-{}",
            idx + 1,
            score,
            chunk.path,
            chunk.start_line,
            chunk.end_line
        );
        if args.show_text {
            println!("{}", chunk.text);
            println!("---");
        }
    }

    Ok(())
}

fn run_ask(args: AskArgs) -> Result<()> {
    if args.top_k == 0 {
        bail!("--top-k must be > 0");
    }

    let conn = Connection::open(&args.db)
        .with_context(|| format!("failed to open sqlite db {}", args.db.display()))?;

    let client = Client::builder()
        .build()
        .context("failed to build HTTP client")?;

    let only_ext = normalize_extensions(&args.only_ext);
    let scored = retrieve_scored_chunks(
        &conn,
        &client,
        &args.embed,
        &args.question,
        args.code_first,
        &only_ext,
    )?;
    let top = scored.into_iter().take(args.top_k).collect::<Vec<_>>();
    if top.is_empty() {
        bail!("no matches found in index");
    }

    let (context, used_sources) = build_context(&top, args.max_context_chars);

    let prompt = format!(
        "You are answering questions about a local codebase. Use only the provided context. \
If the context is insufficient, say so clearly.\n\nQuestion:\n{}\n\nContext:\n{}\n\nAnswer:",
        args.question, context
    );

    let answer = generate_text(&client, &args.embed.ollama_url, &args.llm_model, &prompt)?;
    println!("{}", answer.trim());

    if args.show_sources {
        println!("\nSources:");
        for (idx, score, chunk) in used_sources {
            println!(
                "[{}] {:.4}  {}:{}-{}",
                idx, score, chunk.path, chunk.start_line, chunk.end_line
            );
        }
    }

    Ok(())
}

fn retrieve_scored_chunks(
    conn: &Connection,
    client: &Client,
    embed: &CommonEmbeddingArgs,
    query: &str,
    code_first: bool,
    only_ext: &[String],
) -> Result<Vec<(f32, StoredChunk)>> {
    let mut all_chunks = load_chunks(conn)?;
    if all_chunks.is_empty() {
        bail!("no chunks in db; run `cargo run --bin rag -- index` first");
    }

    if !only_ext.is_empty() {
        all_chunks.retain(|chunk| {
            let ext = path_extension_lower(&chunk.path);
            only_ext.contains(&ext)
        });
        if all_chunks.is_empty() {
            bail!(
                "no chunks matched --only-ext filter; available index may not contain requested extensions"
            );
        }
    }

    let query_embedding = embed_text(client, embed, query).context("failed to embed query text")?;

    let query_tokens = tokenize_query(query);

    let mut scored = all_chunks
        .into_iter()
        .map(|chunk| {
            let base = cosine_similarity(&query_embedding, &chunk.embedding);
            let score = rerank_score(base, &chunk.path, &chunk.text, &query_tokens, code_first);
            (score, chunk)
        })
        .collect::<Vec<_>>();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));
    Ok(scored)
}

fn rerank_score(
    base: f32,
    path: &str,
    text: &str,
    query_tokens: &[String],
    code_first: bool,
) -> f32 {
    if !code_first {
        return base;
    }

    let lower = path.to_ascii_lowercase();

    let mut weighted = base;

    if lower.ends_with(".rs") {
        weighted *= 1.20;
    }

    if lower.ends_with(".toml") {
        weighted *= 0.92;
    }

    if lower == "plan.md" || lower.ends_with("/plan.md") {
        weighted *= 0.80;
    }

    if lower == "plan_status.toml" || lower.ends_with("/plan_status.toml") {
        weighted *= 0.75;
    }

    if !query_tokens.is_empty() {
        let path_l = lower;
        let text_l = text.to_ascii_lowercase();
        let mut path_hits = 0u32;
        let mut text_hits = 0u32;

        for token in query_tokens {
            if path_l.contains(token) {
                path_hits += 1;
            }
            if text_l.contains(token) {
                text_hits += 1;
            }
        }

        let lexical_bonus = (path_hits as f32 * 0.06 + text_hits as f32 * 0.015).min(0.30);
        weighted += lexical_bonus;
    }

    weighted
}

fn tokenize_query(query: &str) -> Vec<String> {
    let stopwords = [
        "the", "and", "for", "with", "from", "that", "this", "where", "what", "how", "is", "are", "was", "were", "to", "in", "of", "on", "a", "an",
    ];

    query
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .map(|s| s.to_ascii_lowercase())
        .filter(|s| s.len() >= 3)
        .filter(|s| !stopwords.contains(&s.as_str()))
        .collect()
}

fn path_extension_lower(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default()
}

fn build_context(
    top: &[(f32, StoredChunk)],
    max_context_chars: usize,
) -> (String, Vec<(usize, f32, StoredChunk)>) {
    let mut context = String::new();
    let mut used = Vec::new();

    for (idx0, (score, chunk)) in top.iter().enumerate() {
        let idx = idx0 + 1;
        let section = format!(
            "[{}] {}:{}-{} (score {:.4})\n{}\n\n",
            idx, chunk.path, chunk.start_line, chunk.end_line, score, chunk.text
        );

        if !context.is_empty() && context.len() + section.len() > max_context_chars {
            break;
        }

        context.push_str(&section);
        used.push((idx, *score, StoredChunk {
            path: chunk.path.clone(),
            start_line: chunk.start_line,
            end_line: chunk.end_line,
            text: chunk.text.clone(),
            embedding: chunk.embedding.clone(),
        }));
    }

    if context.is_empty() {
        if let Some((score, chunk)) = top.first() {
            context = format!(
                "[1] {}:{}-{} (score {:.4})\n{}\n",
                chunk.path, chunk.start_line, chunk.end_line, score, chunk.text
            );
            used.push((1, *score, StoredChunk {
                path: chunk.path.clone(),
                start_line: chunk.start_line,
                end_line: chunk.end_line,
                text: chunk.text.clone(),
                embedding: chunk.embedding.clone(),
            }));
        }
    }

    (context, used)
}

fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS chunks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL,
            start_line INTEGER NOT NULL,
            end_line INTEGER NOT NULL,
            text TEXT NOT NULL,
            embedding_json TEXT NOT NULL
        );
        ",
    )
    .context("failed creating schema")?;
    Ok(())
}

fn load_chunks(conn: &Connection) -> Result<Vec<StoredChunk>> {
    let mut stmt = conn
        .prepare("SELECT path, start_line, end_line, text, embedding_json FROM chunks")
        .context("failed preparing chunk select")?;

    let rows = stmt
        .query_map([], |row| {
            let embedding_json: String = row.get(4)?;
            let embedding: Vec<f32> = serde_json::from_str(&embedding_json).map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(
                    embedding_json.len(),
                    rusqlite::types::Type::Text,
                    Box::new(err),
                )
            })?;

            Ok(StoredChunk {
                path: row.get(0)?,
                start_line: row.get::<_, i64>(1)? as usize,
                end_line: row.get::<_, i64>(2)? as usize,
                text: row.get(3)?,
                embedding,
            })
        })
        .context("failed querying chunks")?;

    let mut out = Vec::new();
    for row in rows {
        out.push(row.context("failed reading chunk row")?);
    }
    Ok(out)
}

fn normalize_extensions(exts: &[String]) -> Vec<String> {
    exts.iter()
        .map(|e| e.trim().trim_start_matches('.').to_ascii_lowercase())
        .filter(|e| !e.is_empty())
        .collect()
}

fn collect_files(root: &Path, exts: &[String]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| !is_skipped_dir(entry.path()))
        .filter_map(|entry| entry.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();

        if !exts.contains(&extension) {
            continue;
        }

        if is_likely_binary(path)? {
            continue;
        }

        files.push(path.to_path_buf());
    }

    files.sort();
    Ok(files)
}

fn collect_selected_files(root: &Path, exts: &[String], paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for input in paths {
        let candidate = if input.is_absolute() {
            input.clone()
        } else {
            root.join(input)
        };

        if !candidate.is_file() {
            continue;
        }

        let extension = candidate
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();

        if !exts.contains(&extension) {
            continue;
        }

        if is_likely_binary(&candidate)? {
            continue;
        }

        files.push(candidate);
    }

    files.sort();
    files.dedup();
    Ok(files)
}

fn is_skipped_dir(path: &Path) -> bool {
    let skipped = [
        ".git",
        "target",
        ".rag",
        "original",
        "build",
        "doc",
    ];

    path.file_name()
        .and_then(|s| s.to_str())
        .map(|name| skipped.contains(&name))
        .unwrap_or(false)
}

fn is_likely_binary(path: &Path) -> Result<bool> {
    let bytes = fs::read(path).with_context(|| format!("failed reading {}", path.display()))?;
    let sample = bytes.get(..4096).unwrap_or(&bytes);
    Ok(sample.contains(&0))
}

fn chunk_file(path: &Path, root: &Path, chunk_lines: usize, overlap_lines: usize) -> Result<Vec<Chunk>> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed reading text file {}", path.display()))?;
    let lines = contents.lines().collect::<Vec<_>>();

    if lines.is_empty() {
        return Ok(Vec::new());
    }

    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");

    let step = chunk_lines - overlap_lines;
    let mut out = Vec::new();
    let mut start = 0usize;

    while start < lines.len() {
        let end = (start + chunk_lines).min(lines.len());
        let text = lines[start..end].join("\n");

        if !text.trim().is_empty() {
            out.push(Chunk {
                path: rel.clone(),
                start_line: start + 1,
                end_line: end,
                text,
            });
        }

        if end == lines.len() {
            break;
        }
        start += step;
    }

    Ok(out)
}

fn embed_text(client: &Client, common: &CommonEmbeddingArgs, text: &str) -> Result<Vec<f32>> {
    let url = format!("{}/api/embeddings", common.ollama_url.trim_end_matches('/'));
    let req = OllamaEmbeddingRequest {
        model: &common.model,
        prompt: text,
    };

    let response = client
        .post(&url)
        .json(&req)
        .send()
        .with_context(|| format!("failed POST {url}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        bail!("embedding request failed ({status}): {body}");
    }

    let parsed: OllamaEmbeddingResponse = response
        .json()
        .context("failed to decode embedding response")?;

    if parsed.embedding.is_empty() {
        bail!("embedding response was empty");
    }

    Ok(parsed.embedding)
}

fn generate_text(client: &Client, ollama_url: &str, model: &str, prompt: &str) -> Result<String> {
    let url = format!("{}/api/generate", ollama_url.trim_end_matches('/'));
    let req = OllamaGenerateRequest {
        model,
        prompt,
        stream: false,
    };

    let response = client
        .post(&url)
        .json(&req)
        .send()
        .with_context(|| format!("failed POST {url}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        bail!("generation request failed ({status}): {body}");
    }

    let parsed: OllamaGenerateResponse = response
        .json()
        .context("failed to decode generation response")?;

    if parsed.response.trim().is_empty() {
        bail!("generation response was empty");
    }

    Ok(parsed.response)
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return -1.0;
    }

    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for (&x, &y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom == 0.0 {
        -1.0
    } else {
        dot / denom
    }
}
