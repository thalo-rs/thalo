#!/bin/bash
set -e

echo "thalo-macros"
cd thalo-macros
sed -i '' -e 's/^version.*$/version = "'$1'"/' Cargo.toml
git commit -am "chore(thalo-macros): version $1"
cargo publish
cd ..
sleep 10

echo "thalo"
cd thalo
sed -i '' -e 's/^version.*$/'"version = \"$1\"/" Cargo.toml
sed -i '' -e 's/^thalo-macros [^,]*,/thalo-macros = { version = "'$1'",/' Cargo.toml
git commit -am "chore(thalo): version $1"
cargo publish
cd ..
sleep 10
