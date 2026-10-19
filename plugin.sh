#!/bin/sh
here="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
sys="$(uname -s)"
case "$sys" in
MINGW* | MSYS* | CYGWIN*) rel="x86_64-pc-windows-msvc/jira.exe" ;;
Darwin*)
    case "$(uname -m)" in
    arm64) rel="aarch64-apple-darwin/jira" ;;
    *) rel="x86_64-apple-darwin/jira" ;;
    esac ;;
*)
    case "$(uname -m)" in
    aarch64 | arm64) rel="aarch64-unknown-linux-musl/jira" ;;
    *) rel="x86_64-unknown-linux-musl/jira" ;;
    esac ;;
esac
if [ -x "$here/$rel" ]; then exec "$here/$rel" "$@"; fi
for cand in "$here/target/release/jira.exe" "$here/target/release/jira" "$here/target/debug/jira.exe" "$here/target/debug/jira"; do
    if [ -x "$cand" ]; then exec "$cand" "$@"; fi
done
echo "jira binary not found in $here" >&2
exit 1
