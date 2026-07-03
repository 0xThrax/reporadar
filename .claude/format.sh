#!/usr/bin/env bash
# Project formatter. Called by global format-dispatcher.sh after every edit.
# Argument: the path of the edited file.

FILE="$1"
case "$FILE" in
  *.rs) cargo fmt -- "$FILE" 2>/dev/null ;;
  *.go) gofmt -w "$FILE" 2>/dev/null ;;
  *.py) ruff format "$FILE" 2>/dev/null ;;
  *.ts|*.tsx|*.js|*.jsx) prettier --write "$FILE" 2>/dev/null ;;
  *.json|*.md|*.yaml|*.yml) prettier --write "$FILE" 2>/dev/null ;;
  *) ;;
esac
