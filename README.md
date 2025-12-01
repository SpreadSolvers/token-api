# token-api

## Getting Started

1. Install diesel CLI

Using Cargo

```bash
cargo install diesel_cli --no-default-features --features sqlite
```

If you ran into any errors while installing refer to diesel's "Getting Started" guide, which provides info about common installation errors and alternative installation method

### 2. Setup diesel dependency

Everything should work fine after first step, but just in case...

For MacOS

```bash
brew install libmysqlclient
```

Other installation methods described here:
https://dev.mysql.com/downloads/c-api/

### 3. Setup DB

Run:

```bash
chmod +x ./setup_db.sh
./setup_db.sh
```

Which:

1. Creates `db` folder in root
2. Runs `diesel setup` to setup db
3. Runs migrations via `diesel migrations run`

### 4. Enjoy or develop your API

Run `cargo run` to run an API in development mode and enjoy!
