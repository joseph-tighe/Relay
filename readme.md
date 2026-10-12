# Relay

Relay is a minamalistic chatting site built specifically for school clubs.
<img width="1654" height="1010" alt="Screenshot 2026-10-11 180151" src="https://github.com/user-attachments/assets/1f352184-b031-466f-be65-98bf847bea19" />

## Features

Subspaces to organize clubs.

## How it works

Relay uses Axum for handing requests, and sqlight for database interactions.
Fot some schools who don't like any communication platform for their clubs, axum will eventually have the option to use the canvas api, to let them handle the backend.

## Prerequisites

- Rust
- Cargo

## Running

```bash
cargo run --bin make_db
cargo run --bin main
```

Then open http://localhost:3000 in your browser.
