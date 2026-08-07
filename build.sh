#!/usr/bin/env bash
# Vercel build entrypoint (see vercel.json). Kept as a script rather than an inline
# buildCommand because Vercel caps that field at 256 characters — the Rust toolchain
# install alone blows past that.
set -euo pipefail

cd app

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
# shellcheck disable=SC1091
. "$HOME/.cargo/env"
rustup target add wasm32-unknown-unknown

curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

npm install
npm run build
