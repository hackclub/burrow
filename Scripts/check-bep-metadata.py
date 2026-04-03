#!/usr/bin/env python3

from __future__ import annotations

import pathlib
import re
import sys


REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
PROPOSALS_DIR = REPO_ROOT / "evolution" / "proposals"
ALLOWED_STATUSES = {
    "Pitch",
    "Draft",
    "In Review",
    "Accepted",
    "Implemented",
    "Rejected",
    "Returned for Revision",
    "Superseded",
    "Archived",
}
REQUIRED_FIELDS = [
    "Status",
    "Proposal",
    "Authors",
    "Coordinator",
    "Reviewers",
    "Constitution Sections",
    "Implementation PRs",
    "Decision Date",
]


def text_block_lines(path: pathlib.Path) -> list[str]:
    content = path.read_text(encoding="utf-8")
    match = re.search(r"```text\n(.*?)\n```", content, re.DOTALL)
    if not match:
        raise ValueError("missing leading ```text metadata block")
    return [line.rstrip() for line in match.group(1).splitlines() if line.strip()]


def validate(path: pathlib.Path) -> list[str]:
    errors: list[str] = []
    proposal_id = path.name.split("-", 2)[:2]
    expected_id = "-".join(proposal_id).removesuffix(".md")

    try:
        lines = text_block_lines(path)
    except ValueError as exc:
        return [f"{path}: {exc}"]

    field_names = [line.split(":", 1)[0] for line in lines]
    if field_names != REQUIRED_FIELDS:
        errors.append(
            f"{path}: metadata fields must appear in order {', '.join(REQUIRED_FIELDS)}"
        )
        return errors

    fields = dict(line.split(":", 1) for line in lines)
    fields = {key.strip(): value.strip() for key, value in fields.items()}

    if fields["Status"] not in ALLOWED_STATUSES:
        errors.append(f"{path}: invalid Status {fields['Status']!r}")

    if fields["Proposal"] != expected_id:
        errors.append(
            f"{path}: Proposal field {fields['Proposal']!r} does not match filename id {expected_id!r}"
        )

    if fields["Status"] in {"Accepted", "Implemented", "Superseded", "Rejected", "Archived"} and fields["Decision Date"] == "Pending":
        errors.append(
            f"{path}: Decision Date must not be Pending once status is {fields['Status']}"
        )

    return errors


def main() -> int:
    errors: list[str] = []
    for path in sorted(PROPOSALS_DIR.glob("BEP-*.md")):
        errors.extend(validate(path))

    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1

    print(f"checked {len(list(PROPOSALS_DIR.glob('BEP-*.md')))} BEPs")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
