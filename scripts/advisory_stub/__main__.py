#!/usr/bin/env python3
"""
CLI entry point for the advisory stub.

Usage:
    python -m scripts.advisory-stub --workload sprint_closeout
    python -m scripts.advisory-stub --all
    python -m scripts.advisory-stub --validate <output-file.json>

Advisory-only — emits contract-valid decision objects without
changing router behavior, runtime HTTP, model execution, or
any production route.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from .stub_engine import (
    AdvisoryStubError,
    generate_all_decisions,
    generate_decision,
    save_decision,
    validate_output,
    WORKLOAD_TYPES,
)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Advisory stub — emit contract-valid context decisions.",
    )
    parser.add_argument(
        "--workload", "-w",
        type=str,
        default=None,
        help=f"Workload type. One of: {', '.join(WORKLOAD_TYPES)}",
    )
    parser.add_argument(
        "--all", "-a",
        action="store_true",
        help="Generate decisions for all 9 workload types",
    )
    parser.add_argument(
        "--validate",
        type=str,
        default=None,
        metavar="FILE",
        help="Validate an existing decision file against the contract",
    )
    parser.add_argument(
        "--output", "-o",
        type=str,
        default=None,
        metavar="FILE",
        help="Save output to JSON file",
    )
    parser.add_argument(
        "--request-id",
        type=str,
        default=None,
        help="Override request ID (auto-generated if omitted)",
    )

    args = parser.parse_args()

    # --- Validate mode ---
    if args.validate:
        path = Path(args.validate)
        if not path.exists():
            print(f"Error: file not found: {path}", file=sys.stderr)
            return 1
        with open(path, "r", encoding="utf-8") as f:
            data = json.load(f)
        violations = validate_output(data)
        if violations:
            print(f"Validation FAILED — {len(violations)} violation(s):")
            for v in violations:
                print(f"  - {v}")
            return 1
        else:
            print(f"Validation PASSED — {path.name} conforms to contract v0.1")
            return 0

    # --- Generate mode ---
    if args.all:
        print(f"Generating decisions for all {len(WORKLOAD_TYPES)} workload types...")
        results = generate_all_decisions()
        passed = sum(1 for v in results.values() if "error" not in v)
        failed = sum(1 for v in results.values() if "error" in v)

        for wl, output in results.items():
            status = "PASS" if "error" not in output else "FAIL"
            route = output.get("context_route", {}).get("selected_route", "N/A") if "error" not in output else "N/A"
            print(f"  [{status}] {wl}: {route}")

        if args.output:
            output_dir = Path(args.output)
            output_dir.mkdir(parents=True, exist_ok=True)
            for wl, output in results.items():
                if "error" not in output:
                    fname = output_dir / f"stub-decision-{wl}.json"
                    save_decision(output, fname)
            print(f"\nSaved to: {output_dir}")

        print(f"\nSummary: {passed} passed, {failed} failed")
        return 0 if failed == 0 else 1

    # --- Single workload ---
    if args.workload:
        if args.workload not in WORKLOAD_TYPES:
            print(
                f"Error: invalid workload_type '{args.workload}'. "
                f"Valid: {', '.join(WORKLOAD_TYPES)}",
                file=sys.stderr,
            )
            return 1

        try:
            output = generate_decision(
                workload_type=args.workload,
                request_id=args.request_id,
                validate=True,
            )
            print(json.dumps(output, indent=2, ensure_ascii=False))

            if args.output:
                path = save_decision(output, args.output)
                print(f"\nSaved to: {path}")

            return 0
        except AdvisoryStubError as e:
            print(f"Error: {e}", file=sys.stderr)
            return 1

    parser.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
