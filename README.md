# worker-audio-server

A reimplementation of the `local audio server` on **Cloudflare
Workers + Rust**.

HTTP interface stays identical to the original:

| Original (Python / Anki)      | This project (Workers / Rust)        |
| ----------------------------- | ------------------------------------ |
| Local audio files on disk     | **R2** bucket (`AUDIO` binding)       |
| `entries.db` (SQLite)         | **D1** database (`DB` binding)        |
| `http.server` on `127.0.0.1`  | Cloudflare Worker (axum router)      |
| `default_config.json`         | embedded in `src/sources.rs`         |

## HTTP interface (unchanged)

- `GET /` → `Worker Audio Server v0.0.1` (plain text)
- `GET /?term=<term>&reading=<reading>&sources=<a,b>&user=<speaker>` →
  Yomitan [`audioSourceList`](https://github.com/yomidevs/yomitan/blob/master/ext/data/schemas/custom-audio-list-schema.json) JSON
- `GET /{source_id}/{file}` → audio bytes (streamed from R2)
- `GET /favicon.ico` → 400

Query parameters mirror `demo/server.py::parse_query_components`:

- `term` (or `expression`) — required
- `reading` — optional; normalized katakana→hiragana, `null`/`undefined`/empty treated as absent
- `sources` — comma-separated source ids; defaults to all configured sources
- `user` — comma-separated speaker names (forvo)

All responses include `Access-Control-Allow-Origin: *`.

## Project layout

```
src/
├── lib.rs       # axum router + all request handlers
├── sources.rs   # source config (the embedded default_config.json)
├── query.rs     # dynamic D1 SQL builder (port of execute_query)
├── jp.rs        # katakana→hiragana (port of jp_util)
└── error.rs     # worker::Error → axum IntoResponse bridge
schema.sql       # D1 schema (identical to the Python entries table)
wrangler.toml    # Worker + D1 + R2 bindings
```
## One-click deployment
[![Deploy to Cloudflare Workers](https://deploy.workers.cloudflare.com/button)](https://deploy.workers.cloudflare.com/?url=https://github.com/tomoyo1024/worker-audio-server)

## Manual deployment

### 1. Create the D1 database and R2 bucket

```sh
wrangler d1 create audio-db
wrangler r2 bucket create audio-bucket
```

Put the printed `database_id` into `wrangler.toml`.

### 2. Create the schema

```sh
wrangler d1 execute audio-db --remote --file=migrations/00_schema.sql
wrangler d1 execute audio-db --remote --file=migrations/01_entries.sql
```
### 3. Upload audio files to bucket

### 4. Run / deploy

```sh
wrangler dev      # local (uses local D1 + R2 state under .wrangler)
wrangler deploy   # production
```

## Pointing Yomitan at it

Set the custom audio source URL to your worker, e.g.:

```
https://<worker>.<subdomain>.workers.dev/?term={term}&reading={reading}
```
