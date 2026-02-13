#!/bin/bash
# Wrapper to start the bot
# Ensure you have your .env file or export variables here

cd "$(dirname "$0")"

# Compile release if not exists (optional, mostly for dev/test)
# cargo build --release

# Run
./target/release/frankfurt_sentinel
