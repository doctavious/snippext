#!/usr/bin/env sh

set -eu

version="$1"
target="$2"
echo "$version"
echo "$target"
uploads=$(find . -maxdepth 1 -name "snippext-$version-$target*")
for upload in $uploads
do
  echo "uploading $upload"
  gh release upload "$1" "$upload"
done

