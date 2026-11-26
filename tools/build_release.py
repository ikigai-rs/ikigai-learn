#!/usr/bin/env python3
"""Build a course release: run Lectern for every unit and write a self-contained release.

The release builder of stage W1 (docs/lectern-wrap-or-native.md). CI runs it at a release's
git tag; Lectern runs only here, never inside an endpoint. The output is a directory the
ikigai-learn endpoints read through the kernel (ikigai-fs or ikigai-repo):

    {out}/{release}/course/{course}/manifest.toml   the course manifest, byte for byte
    {out}/{release}/unit/{unit}/unit.toml           the unit's source kind and Lectern pin
    {out}/{release}/unit/{unit}/assembled.md        `lectern assemble`: the @from map
    {out}/{release}/unit/{unit}/slides/index.html   `lectern build`: reveal HTML (+ assets/)
    {out}/{release}/unit/{unit}/outline.md          `lectern build -f outline`
    {out}/{release}/unit/{unit}/handout/            `lectern build -f pdf`, only with --pdf

It refuses, with one sentence that says what to change, a URL `asset_base`, a unit whose
source kind it does not know, a unit with no deck.toml, a name that cannot be a key in a
`urn:learn:` name, an unset or mismatched LECTERN_COMMIT, and a built page that loads anything
from another host (a release must be self-contained: a viewer's request to a CDN is a request
nobody here controls). A refused build writes nothing.

Run it with the Python of the environment tools/install_lectern.sh made, with LECTERN_COMMIT
set to the commit installed there:

    LECTERN_COMMIT=<commit> VENV/bin/python tools/build_release.py COURSE_DIR RELEASE OUT_DIR [--pdf]
"""
import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
from importlib import metadata
from pathlib import Path

MANIFEST_FORMAT = "urn:learn:manifest:v1"
# The one source kind this stage builds. `mdbook-chapter` attaches here as a second entry.
KINDS = {"lectern-deck"}
NAME = re.compile(r"^[a-z0-9][a-z0-9-]*$")
RELEASE = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]*$")
COMMIT = re.compile(r"^[0-9a-f]{40}$")
# A load from another host in a built page: a tag's src or a stylesheet's href, a module
# import, a CSS url() or @import, or a Markdown image. A plain link (`<a href>`) is not a load.
REMOTE = re.compile(
    r"""<(?:script|img|iframe|audio|video|source|track|embed|object)\b[^>]*?\s(?:src|data)\s*=\s*["']?((?:https?:)?//[^"'\s>]+)"""
    r"""|<link\b[^>]*?\shref\s*=\s*["']?((?:https?:)?//[^"'\s>]+)"""
    r"""|\bimport\b[^;'"]*?["']((?:https?:)?//[^"']+)["']"""
    r"""|\burl\(\s*["']?((?:https?:)?//[^"')\s]+)"""
    r"""|@import\s+["']((?:https?:)?//[^"']+)["']"""
    r"""|!\[[^\]]*\]\(\s*<?((?:https?:)?//[^)\s>]+)""",
    re.IGNORECASE,
)


class Refusal(Exception):
    """A release this builder will not write; the message is the sentence to show."""


def check_name(what, value):
    if not isinstance(value, str) or not NAME.match(value):
        raise Refusal(
            f"The {what} name {value!r} is not a name this module accepts: "
            "use lowercase letters, digits and hyphens, starting with a letter or digit."
        )


def check_release(value):
    if not RELEASE.match(value) or ".." in value:
        raise Refusal(
            f"The release {value!r} is not a tag this module accepts: use letters, digits, "
            "dots, hyphens and underscores, starting with a letter or digit, with no '..'."
        )


def is_url(value):
    return isinstance(value, str) and value.lower().startswith(("http://", "https://", "//"))


def read_toml(path, raw=None):
    try:
        text = (raw if raw is not None else path.read_bytes()).decode("utf-8")
        return tomllib.loads(text)
    except (UnicodeDecodeError, tomllib.TOMLDecodeError) as e:
        raise Refusal(f"{path} is not valid UTF-8 TOML ({e}): fix it there.") from None


def load_manifest(course_dir):
    path = course_dir / "course.toml"
    if not path.is_file():
        raise Refusal(f"There is no course.toml in {course_dir}: add the course manifest there.")
    raw = path.read_bytes()
    manifest = read_toml(path, raw)
    if manifest.get("format") != MANIFEST_FORMAT:
        raise Refusal(f'The course manifest must say format = "{MANIFEST_FORMAT}": add that line.')
    check_name("course", manifest.get("name"))
    units = manifest.get("unit", {})
    order = []
    for module in manifest.get("module", []):
        check_name("module", module.get("name"))
        for unit in module.get("units", []):
            check_name("unit", unit)
            if unit not in units:
                raise Refusal(
                    f"The unit {unit!r} is listed in module {module['name']!r} but has no "
                    f"[unit.{unit}] table: add one with its kind and path."
                )
            if unit in order:
                raise Refusal(f"The unit {unit!r} is listed twice: list each unit once.")
            order.append(unit)
    for unit in units:
        if unit not in order:
            raise Refusal(f"The unit {unit!r} has a [unit.{unit}] table but no module lists it: "
                          "list it in a module or remove the table.")
        kind = units[unit].get("kind")
        if kind not in KINDS:
            raise Refusal(
                f"The unit {unit!r} has source kind {kind!r}, which this builder does not know: "
                f'set kind = "lectern-deck".'
            )
        where = units[unit].get("path")
        if not isinstance(where, str) or not where:
            raise Refusal(f"The unit {unit!r} has no path: set path to its directory in the course, "
                          f'for example path = "units/{unit}".')
        deck_dir = (course_dir / where).resolve()
        if not deck_dir.is_relative_to(course_dir.resolve()):
            raise Refusal(f"The unit {unit!r} has path {where!r}, which leaves the course directory: "
                          "set a path inside it.")
    return raw, manifest, order


def check_deck(unit, deck_dir, where):
    path = deck_dir / "deck.toml"
    if not path.is_file():
        raise Refusal(
            f"The unit {unit!r} has no deck.toml in {where}: add the deck's deck.toml there, or "
            "set the unit's path in course.toml to the directory that holds it."
        )
    deck = read_toml(path)
    if is_url(deck.get("asset_base")):
        raise Refusal(
            f"The unit {unit!r} sets asset_base to the URL {deck['asset_base']!r}, and a release "
            "must be self-contained: set asset_base to a local directory, or remove it."
        )


def lectern_pin():
    """The commit the installed Lectern came from, checked against LECTERN_COMMIT."""
    pin = os.environ.get("LECTERN_COMMIT", "")
    if not COMMIT.match(pin):
        raise Refusal(
            "LECTERN_COMMIT is not set to a full 40-character commit: set it to the Lectern "
            "commit tools/install_lectern.sh installed."
        )
    try:
        direct = json.loads(metadata.distribution("lectern-slides").read_text("direct_url.json") or "{}")
    except metadata.PackageNotFoundError:
        raise Refusal(
            "Lectern is not installed in this Python: run the builder with the Python of the "
            "environment tools/install_lectern.sh made."
        ) from None
    installed = direct.get("vcs_info", {}).get("commit_id")
    if installed is None:
        raise Refusal(
            "The installed Lectern does not record the commit it came from: reinstall it with "
            "tools/install_lectern.sh."
        )
    if installed != pin:
        raise Refusal(
            f"The installed Lectern is commit {installed}, not LECTERN_COMMIT {pin}: reinstall "
            "Lectern at LECTERN_COMMIT with tools/install_lectern.sh, or set LECTERN_COMMIT to "
            "the installed commit."
        )
    return pin


def lectern_command():
    exe = Path(sys.executable).parent / ("lectern.exe" if os.name == "nt" else "lectern")
    if not exe.is_file():
        raise Refusal(
            f"There is no lectern command beside {sys.executable}: run the builder with the "
            "Python of the environment tools/install_lectern.sh made."
        )
    return str(exe)


def run(cmd, env, unit):
    done = subprocess.run(cmd, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    if done.returncode != 0:
        last = (done.stderr.strip().splitlines() or ["no message"])[-1]
        raise Refusal(
            f"Lectern failed on the unit {unit!r} ({cmd[1]}: {last}): fix the deck, then build again."
        )


def check_self_contained(unit, unit_dir):
    """Refuse a built page that loads anything from another host."""
    for page in sorted(unit_dir.rglob("*")):
        if page.suffix not in (".html", ".htm", ".css", ".js", ".mjs"):
            continue
        found = REMOTE.search(page.read_text("utf-8", errors="replace"))
        if found:
            url = next(g for g in found.groups() if g)
            raise Refusal(
                f"The unit {unit!r} builds a page that loads {url} from another host, and a "
                "release must be self-contained: remove that load from the deck, or build with "
                "a Lectern that copies it into the release."
            )


def build(course_dir, release, out_dir, pdf):
    check_release(release)
    raw, manifest, order = load_manifest(course_dir)
    for unit in order:
        where = manifest["unit"][unit]["path"]
        check_deck(unit, course_dir / where, where)
    pin = lectern_pin()
    lectern = lectern_command()
    final = out_dir.resolve() / release
    with tempfile.TemporaryDirectory() as scratch:
        scratch = Path(scratch)
        # Build beside, check, then move into place: a refused build writes nothing.
        root = scratch / "release"
        course_out = root / "course" / manifest["name"]
        course_out.mkdir(parents=True)
        (course_out / "manifest.toml").write_bytes(raw)
        # An empty user config, so nothing outside the tag changes the build.
        env = dict(os.environ, XDG_CONFIG_HOME=str(scratch / "config"), HOME=str(scratch))
        for unit in order:
            spec = manifest["unit"][unit]
            deck = scratch / "unit" / unit
            shutil.copytree(course_dir / spec["path"], deck)
            unit_out = root / "unit" / unit
            unit_out.mkdir(parents=True)
            run([lectern, "assemble", str(deck), "-o", str(unit_out / "assembled.md")], env, unit)
            run([lectern, "build", str(deck), "-o", str(unit_out / "slides")], env, unit)
            outline = scratch / "outline" / unit
            run([lectern, "build", str(deck), "-f", "outline", "-o", str(outline)], env, unit)
            shutil.copyfile(outline / "outline.md", unit_out / "outline.md")
            if pdf:
                run([lectern, "build", str(deck), "-f", "pdf", "-o", str(unit_out / "handout")],
                    env, unit)
            kind = spec["kind"]
            (unit_out / "unit.toml").write_text(f'kind = "{kind}"\nlectern = "{pin}"\n', "utf-8")
            check_self_contained(unit, unit_out)
        for part in ("course", "unit"):
            for built in sorted((root / part).iterdir()):
                target = final / part / built.name
                if target.exists():
                    shutil.rmtree(target)
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.move(str(built), str(target))


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("course_dir", type=Path)
    parser.add_argument("release")
    parser.add_argument("out_dir", type=Path)
    parser.add_argument("--pdf", action="store_true",
                        help="also build the PDF handout (needs Playwright and Chromium)")
    args = parser.parse_args(argv)
    try:
        build(args.course_dir, args.release, args.out_dir, args.pdf)
    except Refusal as refusal:
        print(f"build_release: {refusal}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
