#!/usr/bin/env bash
# Vercel build entrypoint (see vercel.json). Kept as a script rather than an inline
# buildCommand because Vercel caps that field at 256 characters — the Rust toolchain
# install alone blows past that.
set -euo pipefail

cd app

# Vercel's build image ships Rust pre-installed (under /rust, not the usual
# ~/.cargo), so only bootstrap a toolchain if none is already on PATH.
if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
fi

# Source whichever cargo env file actually exists rather than assuming a location.
for env_file in "$HOME/.cargo/env" "/rust/env" "/usr/local/cargo/env"; do
  if [ -f "$env_file" ]; then
    # shellcheck disable=SC1090
    . "$env_file"
    break
  fi
done

rustup target add wasm32-unknown-unknown
command -v wasm-pack >/dev/null 2>&1 || curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

npm install
npm run build
