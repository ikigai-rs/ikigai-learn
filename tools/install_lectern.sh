#!/bin/sh
# Install Lectern at a pinned commit into a fresh virtual environment, reproducibly.
#
#   LECTERN_COMMIT=<40-hex commit> tools/install_lectern.sh VENV
#
# Every dependency (and the build backend) comes from tools/lectern-requirements.txt under
# --require-hashes, so nothing resolves fresh. Lectern itself installs from git at exactly
# LECTERN_COMMIT with --no-deps and --no-build-isolation, so it pulls in nothing beyond the
# locked set, and pip records the commit in the package's direct_url.json, which
# tools/build_release.py checks against LECTERN_COMMIT before it builds.
#
# Lectern (github.com/bsletten/lectern-slides) is read-only to this repository: installed
# from a pinned commit, never forked or changed.
set -eu

venv=${1:?usage: LECTERN_COMMIT=<commit> tools/install_lectern.sh VENV}
case ${LECTERN_COMMIT:-} in
  [0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f][0-9a-f]*)
    if [ ${#LECTERN_COMMIT} -ne 40 ]; then
      echo "install_lectern: LECTERN_COMMIT must be a full 40-character commit, not ${LECTERN_COMMIT}." >&2
      exit 2
    fi ;;
  *)
    echo "install_lectern: LECTERN_COMMIT is not set: set it to the full 40-character Lectern commit to install." >&2
    exit 2 ;;
esac

here=$(cd "$(dirname "$0")" && pwd)
python=${PYTHON:-python3.12}
"$python" -m venv "$venv"
"$venv/bin/python" -m pip install --quiet --no-input --disable-pip-version-check \
  --require-hashes --only-binary :all: -r "$here/lectern-requirements.txt"
"$venv/bin/python" -m pip install --quiet --no-input --disable-pip-version-check \
  --no-deps --no-build-isolation \
  "lectern-slides @ git+https://github.com/bsletten/lectern-slides@${LECTERN_COMMIT}"
"$venv/bin/python" -m pip check
