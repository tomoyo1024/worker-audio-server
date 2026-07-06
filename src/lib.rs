mod error;
mod jp;
mod query;
mod sources;

use axum::extract::OriginalUri;
use axum::http::{HeaderMap, StatusCode};
use axum::{routing::get, Router};
use serde::{Deserialize, Serialize};
use tower_service::Service;
use worker::d1::D1Type;
use worker::*;

use crate::error::AppError;

/// Server version, mirroring `demo/version.txt`.
const VERSION: &str = "0.0.1";

/// File extension → MIME type, mirroring `LocalAudioHandler.SUFFIX_TO_MIME_TYPE`.
fn suffix_to_mime(ext: &str) -> Option<&'static str> {
    match ext.to_ascii_lowercase().as_str() {
        ".mp3" => Some("audio/mpeg"),
        ".aac" => Some("audio/aac"),
        ".m4a" => Some("audio/mp4"),
        ".ogg" => Some("audio/ogg"),
        ".oga" => Some("audio/ogg"),
        ".opus" => Some("audio/ogg"),
        ".flac" => Some("audio/flac"),
        _ => None,
    }
}

fn add_cors_axum(headers: &mut HeaderMap) {
    headers.insert("Access-Control-Allow-Origin", "*".parse().unwrap());
}

fn worker_cors_headers() -> Result<worker::Headers> {
    let headers = worker::Headers::new();
    headers.set("Access-Control-Allow-Origin", "*")?;
    Ok(headers)
}

/// Build the router. `env` is passed as axum state so handlers can reach the
/// D1 and R2 bindings.
fn router(env: Env) -> Router {
    Router::new()
        .route("/", get(root))
        .fallback(get(catch_all))
        .with_state(env)
}

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    env: Env,
    _ctx: Context,
) -> Result<axum::http::Response<axum::body::Body>> {
    Ok(router(env).call(req).await?)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `GET /`
///
/// If the request carries `term`/`expression` query parameters it is treated as
/// an audio source lookup; otherwise the server version is returned (mirroring
/// the Python `do_GET` behavior for the bare `/` path).
#[worker::send]
async fn root(
    axum::extract::State(env): axum::extract::State<Env>,
    OriginalUri(original): OriginalUri,
) -> Result<axum::http::Response<axum::body::Body>, AppError> {
    let base = base_url(&original);
    match parse_query(&original) {
        None => Ok(version_response()?),
        Some(qc) => Ok(audio_list_response(&env, &base, qc).await?),
    }
}

/// Fallback handler for every non-`/` path.
///
/// Handles `/favicon.ico`, audio file requests (`/{source_id}/{file}`), and the
/// fallthrough audio source lookup for any other path (matching the original
/// server's behavior).
#[worker::send]
async fn catch_all(
    axum::extract::State(env): axum::extract::State<Env>,
    OriginalUri(original): OriginalUri,
) -> Result<axum::http::Response<axum::body::Body>, AppError> {
    let path = original.path();

    if path == "/favicon.ico" {
        return Ok(cors_axum(StatusCode::BAD_REQUEST, ""));
    }

    // `/{source_id}/{file}` ?
    let trimmed = path.trim_start_matches('/');
    if let Some((first, file_part)) = trimmed.split_once('/') {
        if sources::is_known_source(first) {
            // percent-decode the file portion so it matches the R2 key exactly
            let file_decoded = percent_encoding::percent_decode_str(file_part)
                .decode_utf8_lossy()
                .into_owned();
            let key = format!("{first}_files/{file_decoded}");
            return Ok(audio_file_response(&env, &key).await?);
        }
    }

    // Fallthrough: treat as an audio source lookup.
    let base = base_url(&original);
    match parse_query(&original) {
        None => Ok(cors_axum(StatusCode::BAD_REQUEST, "")),
        Some(qc) => Ok(audio_list_response(&env, &base, qc).await?),
    }
}

// ---------------------------------------------------------------------------
// Endpoint implementations
// ---------------------------------------------------------------------------
fn version_response() -> Result<axum::http::Response<axum::body::Body>> {
    let payload = format!("Worker Audio Server v{VERSION}");
    let resp = ResponseBuilder::new()
        .with_status(200)
        .with_headers(worker_cors_headers()?)
        .ok(payload)?;
    Ok(resp.into())
}

/// Stream an audio object from R2.
async fn audio_file_response(
    env: &Env,
    key: &str,
) -> Result<axum::http::Response<axum::body::Body>> {
    let bucket = env.bucket("AUDIO")?;

    let Some(object) = bucket.get(key).execute().await? else {
        return Ok(cors_axum(StatusCode::BAD_REQUEST, ""));
    };

    let mime_type = path_extension(key)
        .and_then(suffix_to_mime)
        .ok_or_else(|| Error::RustError(format!("unsupported audio extension for {key}")))?;

    let size = object.size();
    let Some(body) = object.body() else {
        return Ok(cors_axum(StatusCode::BAD_REQUEST, ""));
    };
    let response_body = body.response_body()?;

    let headers = worker::Headers::new();
    headers.set("Content-Type", mime_type)?;
    headers.set("Access-Control-Allow-Origin", "*")?;
    headers.set("Content-Length", &size.to_string())?;
    headers.set("Cache-Control", "public, max-age=31536000, immutable")?;

    let resp = ResponseBuilder::new()
        .with_status(200)
        .with_headers(headers)
        .body(response_body);
    Ok(resp.into())
}

/// Look up matching entries in D1 and return the Yomitan audio source list JSON.
async fn audio_list_response(
    env: &Env,
    base: &str,
    qc: ParsedQuery,
) -> Result<axum::http::Response<axum::body::Body>> {
    let d1 = env.d1("DB")?;

    let total_sources = sources::SOURCES.len();
    let expression = qc.expression.as_str();
    let reading = qc.reading.as_deref();
    let comps = query::QueryComponents {
        expression,
        reading,
        sources: &qc.sources,
        user: &qc.user,
    };

    let (sql, params) = query::build_query(&comps, total_sources);

    // Convert owned params into D1Type::Text references valid for this scope.
    let d1_values: Vec<D1Type<'_>> = params.iter().map(|s| D1Type::Text(s.as_str())).collect();
    let statement = d1.prepare(&sql).bind_refs(&d1_values)?;
    let result = statement.all().await?;
    let rows: Vec<Entry> = result.results()?;

    let mut audio_sources: Vec<AudioSourceJsonEntry> = Vec::with_capacity(rows.len());
    for row in rows {
        let Some(display_template) = sources::display_for(&row.source) else {
            // unknown source — skip, matching the Python behavior
            continue;
        };

        let name = match &row.display {
            Some(d) => apply_display(display_template, d),
            None => display_template.to_string(),
        };

        // Append match-type label when a reading was provided (fallback logic).
        let name = match reading {
            Some(q_reading) => {
                let mut n = name;
                let expr_match = expression == row.expression;
                // row_reading may be NULL → treated as no match.
                let read_match = row.reading.as_deref() == Some(q_reading);
                if expr_match && read_match {
                    n.push_str(" (Expression+Reading)");
                } else if expr_match {
                    n.push_str(" (Only Expression)");
                } else if read_match {
                    n.push_str(" (Only Reading)");
                }
                n
            }
            None => name,
        };

        let url = format!("{}/{}/{}", base, row.source, row.file);

        audio_sources.push(AudioSourceJsonEntry { name, url });
    }

    // `.from_json()` sets Content-Type to application/json; CORS via headers.
    let resp = ResponseBuilder::new()
        .with_status(200)
        .with_headers(worker_cors_headers()?)
        .from_json(&AudioSourceList::new(audio_sources))?;
    Ok(resp.into())
}

// ---------------------------------------------------------------------------
// Query parsing
// ---------------------------------------------------------------------------

struct ParsedQuery {
    expression: String,
    reading: Option<String>,
    sources: Vec<String>,
    user: Vec<String>,
}

/// Parse the Yomitan query parameters, mirroring `parse_query_components`.
/// Returns `None` when no `term`/`expression` parameter is present.
fn parse_query(uri: &axum::http::Uri) -> Option<ParsedQuery> {
    let query = uri.query().unwrap_or("");
    let map = parse_query_string(query);

    // term takes precedence over expression
    let expression = map
        .get("term")
        .or_else(|| map.get("expression"))?
        .to_string();

    // reading is optional
    let reading = match map.get("reading") {
        Some(r) => {
            let r = r.trim();
            if r.is_empty() || r.eq_ignore_ascii_case("null") || r.eq_ignore_ascii_case("undefined")
            {
                None
            } else {
                Some(jp::katakana_to_hiragana(r))
            }
        }
        None => None,
    };

    // sources default to all configured sources
    let sources = match map.get("sources") {
        Some(s) => s.split(',').map(|x| x.trim().to_string()).collect(),
        None => sources::all_source_ids()
            .iter()
            .map(|s| s.to_string())
            .collect(),
    };

    // user (speakers) default to empty
    let user = match map.get("user") {
        Some(s) => s.split(',').map(|x| x.trim().to_string()).collect(),
        None => Vec::new(),
    };

    Some(ParsedQuery {
        expression,
        reading,
        sources,
        user,
    })
}

/// Parse an `application/x-www-form-urlencoded` query string into a map of the
/// first value for each key (matching Python's `parse_qs(...)[key][0]`).
fn parse_query_string(query: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for (k, v) in url::form_urlencoded::parse(query.as_bytes()) {
        map.entry(k.into_owned()).or_insert(v.into_owned());
    }
    map
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract the extension (including the dot) from the last path segment.
fn path_extension(path: &str) -> Option<&str> {
    let last_sep = path.rfind('/').map(|p| p + 1).unwrap_or(0);
    let last_segment = &path[last_sep..];
    let dot = last_segment.rfind('.')?;
    Some(&last_segment[dot..])
}

/// Apply the `%s` display substitution, matching Python's `template % value`
/// (exactly one substitution is performed).
fn apply_display(template: &str, value: &str) -> String {
    template.replacen("%s", value, 1)
}

/// Derive `scheme://host` from the request URI.
///
/// Workers-rs builds the `http::Request` URI from the full request URL, so both
/// the scheme and authority are populated.
fn base_url(uri: &axum::http::Uri) -> String {
    let scheme = uri.scheme_str().unwrap_or("https");
    let authority = uri.authority().map(|a| a.as_str()).unwrap_or("");
    format!("{scheme}://{authority}")
}

/// Build an axum response with CORS headers and the given status / body.
fn cors_axum(status: StatusCode, body: &str) -> axum::http::Response<axum::body::Body> {
    let mut headers = HeaderMap::new();
    add_cors_axum(&mut headers);
    (status, headers, body.to_string()).into_response()
}

// ---------------------------------------------------------------------------
// Serde models
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct Entry {
    expression: String,
    reading: Option<String>,
    source: String,
    #[allow(dead_code)]
    speaker: Option<String>,
    display: Option<String>,
    file: String,
}

#[derive(Debug, Serialize)]
struct AudioSourceJsonEntry {
    name: String,
    url: String,
}

#[derive(Debug, Serialize)]
struct AudioSourceList {
    #[serde(rename = "type")]
    type_field: &'static str,
    #[serde(rename = "audioSources")]
    audio_sources: Vec<AudioSourceJsonEntry>,
}

impl AudioSourceList {
    fn new(audio_sources: Vec<AudioSourceJsonEntry>) -> Self {
        Self {
            type_field: "audioSourceList",
            audio_sources,
        }
    }
}

use axum::response::IntoResponse;
