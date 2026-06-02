#!/usr/bin/env bash
# Calculator helpers.
set -euo pipefail

greet() {
  echo "Calculator"
}

banner() {
  echo "this banner is intentionally long enough to truncate past the configured default string limit of two hundred and fifty six bytes so the truncation marker is emitted in the golden snapshot output for the bash fixture and here is some extra padding text appended to comfortably exceed the limit"
}

NAME="value"
