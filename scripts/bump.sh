#!/bin/sh

crates=("thalo" "thalo_cli" "thalo_macros" "thalo_registry" "thalo_runtime")

latest=$1
version=$2

[ -z "$latest" ] && echo "missing from version" && exit 1
[ -z "$version" ] && echo "missing to version" && exit 1

echo "bumping from $1 to $2"

for crate in ${crates[@]}; do
  sh -c "sed -i '' -e 's/^version.*$/version = \"$version\"/' ./crates/$crate/Cargo.toml"
done
