#!/usr/bin/env bash
set -e

MYSELF=$(realpath "$0")
MYDIR=$(dirname "$MYSELF")

RULMEE_PATH=${RULMEE_PATH:-$(command which rulmee)}
echo "Using '$RULMEE_PATH'"
[[ -e "$RULMEE_PATH" ]] || { echo "'$RULMEE_PATH' is not executable" >&2; exit 1; }

echo "Press \`Ctrl + C\` once you are done viewing the theme"
sleep 3

for theme in "$MYDIR/"*.ini "$MYDIR/"*.toml; do
    RULMEE_CONF="$theme" "$RULMEE_PATH" || :
    echo "That was '$(basename "$theme")'"
    sleep 2 || :
done
