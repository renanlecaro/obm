#!/bin/bash

cargo build --release

wasm-pack build --target web --out-dir docs --release

rm docs/.gitignore
rm docs/*.d.ts
rm docs/package.json
  
cp target/release/obm docs/obm
