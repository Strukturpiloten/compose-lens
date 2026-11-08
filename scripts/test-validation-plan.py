#!/usr/bin/env python3
"""Independent ComposeLens expectations for the canonical planner and gate."""

from __future__ import annotations

import copy
import json
import unittest
from pathlib import Path
from typing import Any

from boxferry_ci.contracts import ContractError
from boxferry_ci.gate import GateFailure, verify_gate
from boxferry_ci.planner import Comparison, make_plan, parse_diff, path_profiles
from boxferry_ci.policy import load_policy

ROOT = Path(__file__).resolve().parents[1]
POLICY = load_policy(ROOT / "scripts/validation-policy.json")
SHA = "a" * 40


class ValidationPlanTests(unittest.TestCase):
    def test_prose_and_executable_examples_have_distinct_selection(self) -> None:
        for contents, expected in (
            (b"# Concepts\n\nOrdinary prose.\n", {"prose"}),
            (b"```console\ncargo test\n```\n", {"executable-docs"}),
            (b"Use `cargo test`.\n", {"executable-docs"}),
            (b"    cargo test\n", {"executable-docs"}),
        ):
            with self.subTest(contents=contents):
                self.assertEqual(
                    path_profiles("docs/public/concepts/index.md", (contents,), POLICY, set()),
                    expected,
                )

    def test_configuration_native_inputs_and_unknown_paths_force_complete(self) -> None:
        for path in (
            "scripts/validation-policy.json",
            ".github/workflows/ci.yml",
            ".devcontainer/Dockerfile",
            "Cargo.lock",
            "src/lib.rs",
            "conformance/provider-config-matrix.toml",
            "unknown.md",
        ):
            with self.subTest(path=path):
                self.assertEqual(path_profiles(path, (b"text\n",), POLICY, set()), {"full"})

    def test_non_pr_events_select_every_job_without_force_full(self) -> None:
        for event in ("push", "workflow_dispatch", "workflow_call"):
            with self.subTest(event=event):
                plan = make_plan(ROOT, POLICY, event=event, tested_sha=SHA)
                self.assertEqual(plan["profiles"], ["full"])
                self.assertEqual(plan["jobs"], dict.fromkeys(POLICY.jobs, True))

    def test_diff_parser_preserves_both_rename_paths_and_rejects_corruption(self) -> None:
        self.assertEqual(
            parse_diff(b"R100\0old.md\0src/lib.rs\0"), [("R", ("old.md", "src/lib.rs"))]
        )
        for raw in (b"M\0missing-delimiter", b"R100\0only-one-path\0", b"M\0../outside\0"):
            with self.subTest(raw=raw), self.assertRaises(ContractError):
                parse_diff(raw)

    def test_every_ordinary_job_and_exact_candidate_are_required(self) -> None:
        self.assertEqual(
            set(POLICY.jobs),
            {
                "rust",
                "msrv",
                "dependencies",
                "api",
                "documentation",
                "coverage",
                "lockfile-release-age",
                "devcontainer-validation",
            },
        )
        plan = make_plan(ROOT, POLICY, event="push", tested_sha=SHA)
        needs: dict[str, Any] = {
            job: {"result": "success"} for job in ("validation-plan", *POLICY.jobs)
        }
        comparison = Comparison("push", SHA, SHA, SHA)
        verify_gate(plan, needs, POLICY, comparison, expected_plan=plan)
        with self.assertRaises(ContractError):
            verify_gate(
                plan,
                needs,
                POLICY,
                Comparison("push", "b" * 40, "b" * 40, "b" * 40),
                expected_plan=plan,
            )
        for job in POLICY.jobs:
            for result in ("failure", "cancelled", "skipped"):
                changed = copy.deepcopy(needs)
                changed[job]["result"] = result
                with self.subTest(job=job, result=result), self.assertRaises(GateFailure):
                    verify_gate(plan, changed, POLICY, comparison, expected_plan=plan)
        changed = copy.deepcopy(needs)
        changed["unplanned-job"] = {"result": "success"}
        with self.assertRaises(ContractError):
            verify_gate(plan, changed, POLICY, comparison, expected_plan=plan)

    def test_missing_job_and_candidate_authored_skip_are_rejected(self) -> None:
        plan = make_plan(ROOT, POLICY, event="push", tested_sha=SHA)
        needs: dict[str, Any] = {
            job: {"result": "success"} for job in ("validation-plan", *POLICY.jobs)
        }
        del needs["coverage"]
        with self.assertRaises(ContractError):
            verify_gate(plan, needs, POLICY, Comparison("push", SHA, SHA, SHA), expected_plan=plan)
        changed = json.loads(json.dumps(plan))
        changed["jobs"]["coverage"] = False
        needs["coverage"] = {"result": "skipped"}
        with self.assertRaises(ContractError):
            verify_gate(
                changed, needs, POLICY, Comparison("push", SHA, SHA, SHA), expected_plan=plan
            )


if __name__ == "__main__":
    unittest.main()
