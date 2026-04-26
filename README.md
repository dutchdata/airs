# airs

instant full-text search for your claude chat history. local, fast, single binary.

`ai` + `rs`

![main screen](screenshots/main_screen.webp)

## what it does

- imports your `conversations.json` claude export into lmdb
- substring search (case-insensitive) across thousands of chats in milliseconds
- per-thread search
- chat activity visualization (day / week / month / quarter / year)
- export titles, copy threads for substack, jump to original on claude.ai

![usage data](screenshots/usage_data.webp)

## requirements

- rust (edition 2024, so stable >= 1.85)
- your `conversations.json` from claude's data export

## setup

1. request your data from claude → settings → privacy → export data
2. unzip, drop `conversations.json` into `claude-data/` at the repo root
3. `cargo run --release`
4. open http://127.0.0.1:8080

first run imports into lmdb (`lmdb_convos/`, `lmdb_messages/`). subsequent runs skip import.

## config

| env | default | note |
|---|---|---|
| `AIRS_BIND` | `127.0.0.1:8080` | set to `0.0.0.0:8080` to expose on lan |
| `RUST_LOG` | `info` | standard tracing filter |

## reset

delete `lmdb_convos/` and `lmdb_messages/` to force reimport.