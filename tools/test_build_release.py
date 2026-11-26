"""The builder's refusals, pinned word for word.

Run it with the Python of the environment tools/install_lectern.sh made, and LECTERN_COMMIT
set to the commit installed there:

    LECTERN_COMMIT=<commit> VENV/bin/python tools/test_build_release.py
"""
import importlib.util
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
SAMPLE = HERE.parent / "sample" / "course"
PIN = os.environ["LECTERN_COMMIT"]


def refusal(edit, env=None):
    with tempfile.TemporaryDirectory() as tmp:
        course = Path(tmp) / "course"
        shutil.copytree(SAMPLE, course)
        edit(course)
        run = subprocess.run(
            [sys.executable, str(HERE / "build_release.py"), str(course), "v1", str(Path(tmp) / "out")],
            capture_output=True, text=True, env=dict(os.environ, **(env or {})),
        )
        assert run.returncode == 2, run
        assert "Traceback" not in run.stderr, run.stderr
        assert not (Path(tmp) / "out").exists(), "a refused release writes nothing"
        return run.stderr.strip()


def same(_course):
    pass


def url_asset_base(course):
    deck = course / "units" / "intro" / "deck.toml"
    deck.write_text(deck.read_text() + 'asset_base = "https://cdn.example/assets"\n')


def unknown_kind(course):
    manifest = course / "course.toml"
    manifest.write_text(manifest.read_text().replace(
        '[unit.recall]\nkind = "lectern-deck"', '[unit.recall]\nkind = "mdbook-chapter"'))


def no_deck(course):
    (course / "units" / "recall" / "deck.toml").unlink()


def bad_toml(course):
    (course / "units" / "intro" / "deck.toml").write_text('title = "unclosed\n')


checks = [
    (url_asset_base, None,
     "build_release: The unit 'intro' sets asset_base to the URL 'https://cdn.example/assets', "
     "and a release must be self-contained: set asset_base to a local directory, or remove it."),
    (unknown_kind, None,
     "build_release: The unit 'recall' has source kind 'mdbook-chapter', which this builder does "
     'not know: set kind = "lectern-deck".'),
    (no_deck, None,
     "build_release: The unit 'recall' has no deck.toml in units/recall: add the deck's deck.toml "
     "there, or set the unit's path in course.toml to the directory that holds it."),
    (same, {"LECTERN_COMMIT": ""},
     "build_release: LECTERN_COMMIT is not set to a full 40-character commit: set it to the "
     "Lectern commit tools/install_lectern.sh installed."),
    (same, {"LECTERN_COMMIT": "0" * 40},
     f"build_release: The installed Lectern is commit {PIN}, not LECTERN_COMMIT {'0' * 40}: "
     "reinstall Lectern at LECTERN_COMMIT with tools/install_lectern.sh, or set LECTERN_COMMIT "
     "to the installed commit."),
]
for edit, env, want in checks:
    got = refusal(edit, env)
    assert got == want, f"\n got: {got}\nwant: {want}"

# Malformed TOML is a sentence too, naming the file (the parser's detail varies by version).
got = refusal(bad_toml)
assert got.startswith("build_release: ") and "deck.toml is not valid UTF-8 TOML" in got, got
assert got.endswith(": fix it there."), got

# A built page that loads from another host is refused; a plain link is not a load.
spec = importlib.util.spec_from_file_location("build_release", HERE / "build_release.py")
builder = importlib.util.module_from_spec(spec)
spec.loader.exec_module(builder)
loads = [
    '<script src="https://cdn.example/reveal.js"></script>',
    '<link rel="stylesheet" href="//cdn.example/reveal.css">',
    "import mermaid from 'https://cdn.example/mermaid.mjs';",
    "body { background: url(https://cdn.example/bg.png) }",
    "![a chart](https://cdn.example/chart.png)",
]
for load in loads:
    with tempfile.TemporaryDirectory() as tmp:
        (Path(tmp) / "index.html").write_text(load)
        try:
            builder.check_self_contained("intro", Path(tmp))
        except builder.Refusal as refusal_:
            assert str(refusal_).startswith("The unit 'intro' builds a page that loads "), refusal_
            assert str(refusal_).endswith(
                "from another host, and a release must be self-contained: remove that load from "
                "the deck, or build with a Lectern that copies it into the release."), refusal_
        else:
            raise AssertionError(f"not refused: {load}")
with tempfile.TemporaryDirectory() as tmp:
    (Path(tmp) / "index.html").write_text('<a href="https://example.org/">a link</a> <script src="reveal/reveal.js"></script>')
    builder.check_self_contained("intro", Path(tmp))

print(f"ok: {len(checks) + 1} refusals, {len(loads)} remote loads refused, a link and a local load kept")
