#!/usr/bin/env python3
"""Consumer-owned source integration expectations; no engines or publication."""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from boxferry_ci import __file__ as shared_module_file
from boxferry_ci import tooling as shared_tooling
from boxferry_ci.policy import load_policy

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = json.loads((ROOT / ".github/automation.json").read_text())
POLICY = load_policy(ROOT / "scripts/validation-policy.json")


class ConsumerContractTests(unittest.TestCase):
    def test_immutable_shared_source_has_one_authority(self) -> None:
        revision = MANIFEST["revision"]
        self.assertRegex(revision, r"^[a-f0-9]{40}$")
        for path in (ROOT / ".github/workflows").glob("*.yml"):
            for match in re.finditer(
                r"Strukturpiloten/boxferry/[^\s@]+@([a-f0-9]{40})", path.read_text()
            ):
                self.assertEqual(match[1], revision, path.name)
        config = json.loads((ROOT / ".github/renovate.json").read_text())
        self.assertEqual(
            config["extends"],
            [
                f"github>Strukturpiloten/boxferry//renovate-presets/{name}#{revision}"
                for name in ("common", "rust")
            ],
        )
        self.assertEqual(
            set(config) & {"minimumReleaseAge", "automerge", "lockFileMaintenance"}, set()
        )

    def assert_service_permissions(self, workflow: str) -> None:
        header, boundary, jobs = workflow.partition("\njobs:\n")
        self.assertEqual(boundary, "\njobs:\n")
        self.assertEqual(re.findall(r"(?m)^permissions:.*$", header), ["permissions: {}"])
        self.assertTrue(jobs.startswith("  eligibility:\n"))
        blocks = re.findall(r"(?ms)^    permissions:\n(.*?)(?=^    [a-z][a-z-]*:|\Z)", jobs)
        self.assertEqual(len(blocks), 1)
        rows = [
            line
            for line in blocks[0].splitlines()
            if line.strip() and not line.lstrip().startswith("#")
        ]
        scopes = re.findall(r"(?m)^      ([a-z-]+): ([a-z-]+)$", "\n".join(rows))
        self.assertEqual(len(scopes), len(rows))
        self.assertEqual(len(scopes), 5)
        self.assertEqual(
            dict(scopes),
            {
                "actions": "write",
                "checks": "read",
                "contents": "read",
                "pull-requests": "write",
                "statuses": "write",
            },
        )

    def test_dependency_service_scopes_are_job_local_and_match_the_callee(self) -> None:
        workflow = (ROOT / ".github/workflows/dependency-eligibility.yml").read_text()
        self.assert_service_permissions(workflow)
        self.assert_service_permissions(
            workflow.replace("      statuses: write\n", "      statuses: write\n\n", 1)
        )
        self.assertIn(
            "uses: Strukturpiloten/boxferry/.github/workflows/shared-dependency-eligibility.yml@"
            + MANIFEST["revision"],
            workflow,
        )
        ci = (ROOT / ".github/workflows/ci.yml").read_text()
        self.assertNotRegex(ci, r"(?m)^\s+(?:pull-requests|checks): write\s*$")

    def test_dependency_service_rejects_missing_or_excessive_permissions(self) -> None:
        workflow = (ROOT / ".github/workflows/dependency-eligibility.yml").read_text()
        for before, after in (
            ("      pull-requests: write\n", "      pull-requests: read\n"),
            ("      checks: read\n", ""),
            ("      checks: read\n", "      checks: read\n      packages: write\n"),
            ("      statuses: write\n", "      statuses: write\n\n      issues: write\n"),
            ("permissions: {}\n", "permissions: write-all\n"),
        ):
            with self.subTest(before=before, after=after):
                changed = workflow.replace(before, after, 1)
                self.assertNotEqual(changed, workflow)
                with self.assertRaises(AssertionError):
                    self.assert_service_permissions(changed)

    def test_native_and_coverage_commands_remain_consumer_owned(self) -> None:
        quality = POLICY.task("rust-quality").commands
        self.assertIn(("cargo", "ci-application"), quality)
        self.assertIn(("bash", "scripts/test-provider-python-bootstrap.sh"), quality)
        self.assertEqual(
            quality[0], ("bash", ".devcontainer/verify-tools.sh", "--check-python-venv")
        )
        self.assertEqual(
            POLICY.task("coverage").commands[-1][-6:],
            (
                "--fail-under-regions",
                "88",
                "--fail-under-functions",
                "87",
                "--fail-under-lines",
                "89",
            ),
        )
        self.assertEqual(
            POLICY.local_commands["full"], (("bash", "scripts/check-all.sh", "--check"),)
        )

    def test_ordinary_gate_defers_image_qualification_without_losing_jobs(self) -> None:
        ci = (ROOT / ".github/workflows/ci.yml").read_text()
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
            },
        )
        for value in (
            "--devcontainer-validation",
            "select_devcontainer_validation",
            "shared-devcontainer-validation.yml@",
            "shared-devcontainer-updates.yml@",
        ):
            self.assertNotIn(value, ci)
        for job in POLICY.jobs:
            self.assertIn(f"      - {job}\n", ci)
        self.assertIn("boxferry_ci gate", ci)
        self.assertIn('--plan-json "${PLAN_JSON}" --needs-json "${NEEDS_JSON}"', ci)

    def test_ordinary_local_gate_does_not_require_image_storage_or_privilege(self) -> None:
        script = (ROOT / "scripts/check-all.sh").read_text()
        for value in ("devcontainer-local", "--outer-mode", "graphroot", "sudo"):
            self.assertNotIn(value, script)
        self.assertIn("--task api-development", script)
        self.assertIn("scripts/test-provider-python-bootstrap.sh", script)
        self.assertNotIn("BOXFERRY_OUTER_MODE", script)

    def test_api_purpose_and_release_receipts_are_bound(self) -> None:
        ci = (ROOT / ".github/workflows/ci.yml").read_text()
        release = (ROOT / ".github/workflows/release.yml").read_text()
        for value in (
            "default: published",
            "api_purpose: ${{ inputs.api_purpose || 'development' }}",
            'test "${PUBLISHED_API_CANDIDATE_SHA}" = "${GITHUB_SHA}"',
        ):
            self.assertIn(value, ci)
        self.assertIn("api_purpose: published", release)
        self.assertIn("needs.crate-preparation.result == 'success'", release)
        self.assertEqual(
            release.count("uses: ./_automation/.github/actions/shared-crate-publish"), 2
        )
        self.assertNotIn("cargo publish --locked", release)
        command = POLICY.task("api-development").commands[0]
        self.assertIn("--release-type", command)
        self.assertEqual(command.count("--repository"), 1)
        self.assertEqual(command[command.index("--repository") + 1], "Strukturpiloten/compose-lens")

    def test_deferred_image_keeps_its_existing_integrity_and_installer(self) -> None:
        dockerfile = (ROOT / ".devcontainer/Dockerfile").read_text()
        images = re.findall(r"# release: ([^\n]+)\nFROM ([^\s]+)", dockerfile)
        self.assertEqual(len(images), 2)
        for release, reference in images:
            self.assertRegex(release, r"^[0-9]+\.[0-9]+\.[0-9]+-[^\s]+$")
            self.assertRegex(reference, r"@sha256:[a-f0-9]{64}$")
        self.assertIn("COPY scripts/install-file-tools.sh /tmp/install-file-tools.sh", dockerfile)
        self.assertNotIn("shared-ci.py", dockerfile)
        self.assertNotIn("--python-tools", dockerfile)

    def test_legacy_image_pins_have_one_held_owner_without_freezing_host_tools(self) -> None:
        config = json.loads((ROOT / ".github/renovate.json").read_text())
        owners = [
            manager
            for manager in config["customManagers"]
            if any("install-file-tools" in value for value in manager["managerFilePatterns"])
        ]
        self.assertEqual(len(owners), 1)
        self.assertEqual(
            owners[0]["matchStrings"],
            [
                r'# renovate: datasource=(?<datasource>[^\s]+) depName=(?<depName>[^\s]+)\nreadonly [a-z0-9_]+_version="(?<currentValue>[^"]+)"'
            ],
        )
        hold = config["packageRules"][-1]
        self.assertEqual(
            hold["matchFileNames"], [".devcontainer/**", "scripts/install-file-tools.sh"]
        )
        self.assertIs(hold["enabled"], False)
        self.assertIs(hold["dependencyDashboardApproval"], False)
        self.assertNotIn("matchPackageNames", hold)
        self.assertNotIn("Cargo.toml", hold["matchFileNames"])
        host = (ROOT / "scripts/install-host-tools.sh").read_text()
        self.assertIn('shared-ci.py" tools', host)
        self.assertNotIn("install-file-tools.sh", host)
        self.assertNotIn("curl", host)
        ci = (ROOT / ".github/workflows/ci.yml").read_text()
        self.assertIn("bash scripts/install-host-tools.sh", ci)
        self.assertNotIn("bash scripts/install-file-tools.sh", ci)

    def test_host_tool_wrapper_forwards_to_the_actual_selected_core_interface(self) -> None:
        with tempfile.TemporaryDirectory(prefix="compose tools ") as temporary:
            fixture = Path(temporary)
            scripts = fixture / "scripts"
            scripts.mkdir()
            wrapper = scripts / "install-host-tools.sh"
            shutil.copyfile(ROOT / "scripts/install-host-tools.sh", wrapper)
            binary = fixture / "bin"
            binary.mkdir()
            interpreter = binary / "python3"
            interpreter.write_text(
                f"#!{sys.executable}\n"
                "import json, os, sys\n"
                "from pathlib import Path\n"
                "from unittest.mock import patch\n"
                "from boxferry_ci import tooling\n"
                "Path(os.environ['ARGV_LOG']).write_text(json.dumps(sys.argv[1:]))\n"
                "def install(destination, tools):\n"
                "    Path(os.environ['INSTALL_LOG']).write_text(json.dumps({'destination': str(destination), 'tools': list(tools)}))\n"
                "    if os.environ.get('TOOL_FAILURE'): raise tooling.ToolError('synthetic installation refusal')\n"
                "    return []\n"
                "with patch.object(tooling, 'install', install): status = tooling.main(sys.argv[3:])\n"
                "raise SystemExit(int(os.environ.get('FORCE_STATUS', status)))\n"
            )
            interpreter.chmod(0o755)
            argv_log = fixture / "argv.json"
            install_log = fixture / "install.json"
            environment = {
                **os.environ,
                "PATH": str(binary) + os.pathsep + os.environ["PATH"],
                "ARGV_LOG": str(argv_log),
                "INSTALL_LOG": str(install_log),
            }
            for arguments, destination in (
                ([], "/usr/local/bin"),
                ([str(fixture / "destination spaces")], str(fixture / "destination spaces")),
            ):
                with self.subTest(arguments=arguments):
                    result = subprocess.run(
                        ["/bin/bash", str(wrapper), *arguments],
                        env=environment,
                        capture_output=True,
                        text=True,
                        check=False,
                        timeout=20,
                    )
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertEqual(
                        json.loads(argv_log.read_text()),
                        [
                            str(scripts / "shared-ci.py"),
                            "tools",
                            "--destination",
                            destination,
                            "--profile",
                            "core",
                        ],
                    )
                    self.assertEqual(
                        json.loads(install_log.read_text()),
                        {"destination": destination, "tools": list(shared_tooling.CORE)},
                    )
            refused = subprocess.run(
                ["/bin/bash", str(wrapper)],
                env={**environment, "TOOL_FAILURE": "1"},
                capture_output=True,
                text=True,
                check=False,
                timeout=20,
            )
            self.assertEqual(refused.returncode, 2)
            self.assertIn("Canonical tool installation failed", refused.stderr)
            failed = subprocess.run(
                ["/bin/bash", str(wrapper)],
                env={**environment, "FORCE_STATUS": "23"},
                capture_output=True,
                text=True,
                check=False,
                timeout=20,
            )
            self.assertEqual(failed.returncode, 23)
            argv_log.unlink()
            invalid = subprocess.run(
                ["/bin/bash", str(wrapper), "one", "two"],
                env=environment,
                capture_output=True,
                text=True,
                check=False,
                timeout=20,
            )
            self.assertEqual(invalid.returncode, 2)
            self.assertFalse(argv_log.exists())

    def test_hosted_node_uses_selected_authority_and_preserves_cache(self) -> None:
        workflow = (ROOT / ".github/workflows/ci.yml").read_text()
        documentation = workflow.split("  documentation:\n", 1)[1].split(
            "  lockfile-release-age:\n", 1
        )[0]
        steps = re.split(r"(?m)^      - name: ", documentation)[1:]
        checkout = next(
            i
            for i, step in enumerate(steps)
            if step.startswith("Check out immutable shared automation\n")
        )
        python = next(
            i
            for i, step in enumerate(steps)
            if "uses: ./_automation/.github/actions/shared-python\n" in step
        )
        node = next(
            i
            for i, step in enumerate(steps)
            if "uses: ./_automation/.github/actions/shared-node\n" in step
        )
        npm = next(i for i, step in enumerate(steps) if "npm ci --ignore-scripts\n" in step)
        self.assertLess(checkout, python)
        self.assertLess(python, node)
        self.assertLess(node, npm)
        self.assertEqual(
            sum("uses: ./_automation/.github/actions/shared-python\n" in step for step in steps), 1
        )
        self.assertEqual(
            sum("uses: ./_automation/.github/actions/shared-node\n" in step for step in steps), 1
        )
        self.assertIn("ref: " + MANIFEST["revision"] + "\n", steps[checkout])
        self.assertIn("automation-sha: " + MANIFEST["revision"] + "\n", steps[node])
        self.assertIn("cache: npm\n", steps[node])
        self.assertIn("cache-dependency-path: package-lock.json\n", steps[node])
        self.assertTrue((ROOT / "package-lock.json").is_file())
        self.assertNotRegex(workflow, r"(?m)^\s+node-version:")
        self.assertNotIn("uses: actions/setup-node@", workflow)
        self.assertNotIn("# renovate: datasource=node-version depName=node", workflow)

    def test_third_party_age_and_whole_first_party_exception_live_centrally(self) -> None:
        assert shared_module_file is not None
        shared = Path(shared_module_file).resolve().parents[3]
        common = json.loads((shared / "renovate-presets/common.json").read_text())
        rust = json.loads((shared / "renovate-presets/rust.json").read_text())
        self.assertEqual(common["minimumReleaseAge"], "3 days")
        rules = [rule for rule in rust["packageRules"] if rule.get("minimumReleaseAge") == "0 days"]
        self.assertEqual(len(rules), 1)
        self.assertEqual(rules[0]["matchManagers"], ["cargo"])
        self.assertEqual(rules[0]["matchDatasources"], ["crate"])
        self.assertTrue(
            {"compose-lens", "podman-lens", "quadlet-lens", "docker-lens", "kubernetes-lens"}
            <= set(rules[0]["matchPackageNames"])
        )

    def test_fixed_native_targets_retain_discovery_without_an_update_route(self) -> None:
        config = json.loads((ROOT / ".github/renovate.json").read_text())
        native_paths = {
            "conformance/provider-config-matrix.toml",
            ".github/workflows/provider-conformance.yml",
        }
        disabled = [
            rule
            for rule in config["packageRules"]
            if rule.get("enabled") is False
            and native_paths.intersection(rule.get("matchFileNames", []))
        ]
        self.assertEqual(len(disabled), 2)
        rules = [rule for rule in disabled if rule.get("matchManagers") == ["custom.regex"]]
        self.assertEqual(len(rules), 1)
        self.assertEqual(rules[0]["matchManagers"], ["custom.regex"])
        self.assertEqual(
            set(rules[0]["matchFileNames"]),
            {
                "conformance/provider-config-matrix.toml",
                ".github/workflows/provider-conformance.yml",
            },
        )
        self.assertIs(rules[0]["dependencyDashboardApproval"], False)
        native = [
            {key: value for key, value in rule.items() if key != "description"}
            for rule in disabled
            if rule.get("matchManagers") == ["github-actions"]
        ]
        self.assertEqual(
            native,
            [
                {
                    "matchManagers": ["github-actions"],
                    "matchDatasources": ["github-releases"],
                    "matchPackageNames": ["actions/python-versions"],
                    "matchDepTypes": ["uses-with"],
                    "matchFileNames": [".github/workflows/provider-conformance.yml"],
                    "enabled": False,
                    "dependencyDashboardApproval": False,
                }
            ],
        )
        for description in (
            "Discover fixed provider-conformance Python bootstrap pins",
            "Track the reviewed Python bootstrap runtime across the provider matrix and runner",
        ):
            managers = [
                manager
                for manager in config["customManagers"]
                if manager["description"] == description
            ]
            self.assertEqual(len(managers), 1)
            manager = managers[0]
            observed = []
            for expression in manager["matchStrings"]:
                pattern = re.sub(r"\(\?<([A-Za-z_]\w*)>", r"(?P<\1>", expression)
                for name in (
                    "conformance/provider-config-matrix.toml",
                    ".github/workflows/provider-conformance.yml",
                ):
                    if any(
                        re.search(pattern[1:-1], name) for pattern in manager["managerFilePatterns"]
                    ):
                        observed.extend(
                            match.groupdict()
                            for match in re.finditer(pattern, (ROOT / name).read_text())
                        )
            self.assertTrue(observed, description)

    def test_custom_runner_discovery_keeps_only_the_fixed_annotated_matrix(self) -> None:
        config = json.loads((ROOT / ".github/renovate.json").read_text())
        owners = [
            manager
            for manager in config["customManagers"]
            if manager.get("description") == "Track fixed GitHub-hosted runner environments"
        ]
        self.assertEqual(len(owners), 1)
        owner = owners[0]
        self.assertEqual(owner["datasourceTemplate"], "github-runners")
        self.assertEqual(len(owner["matchStrings"]), 1)
        expression = owner["matchStrings"][0]
        self.assertNotIn("runs-on:", expression)
        pattern = re.sub(r"\(\?<([A-Za-z_]\w*)>", r"(?P<\1>", expression)
        provider = (ROOT / ".github/workflows/provider-conformance.yml").read_text()
        discovered = [match.groupdict() for match in re.finditer(pattern, provider)]
        self.assertEqual(len(discovered), 1)
        self.assertIn(
            "- " + discovered[0]["depName"] + "-" + discovered[0]["currentValue"], provider
        )
        workflow = (ROOT / ".github/workflows/ci.yml").read_text()
        self.assertEqual(list(re.finditer(pattern, workflow)), [])

    def test_native_contract_requires_an_explicit_verified_cache_without_acquisition(self) -> None:
        from boxferry_ci import renovate_runtime

        with (
            tempfile.TemporaryDirectory() as temporary,
            patch.object(renovate_runtime, "_verify") as verify,
            patch.object(renovate_runtime, "run_command") as command,
        ):
            root = Path(temporary)
            for runtime in (root / "absent", root):
                with self.subTest(runtime=runtime), self.assertRaises(ValueError):
                    verify_cached_native_node_owner(runtime)
            (root / "source-image.txt").write_text("synthetic mismatched identity")
            with self.assertRaises(ValueError):
                verify_cached_native_node_owner(root)
            verify.assert_not_called()
            command.assert_not_called()

    def test_planner_regressions_and_mocked_gate(self) -> None:
        for command in (
            [sys.executable, "scripts/test-validation-plan.py"],
            ["bash", "scripts/test-check-all.sh"],
        ):
            result = subprocess.run(
                command, cwd=ROOT, capture_output=True, text=True, timeout=60, check=False
            )
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


def verify_cached_native_node_owner(runtime: Path) -> dict[str, object]:
    """Explicit native proof; consume a verified cache and never acquire one."""
    assert shared_module_file is not None
    selected = Path(shared_module_file).resolve().parents[3]
    from boxferry_ci import renovate_runtime

    pin, version = renovate_runtime._pin(selected / "automation/renovate-runtime.json")
    if runtime.is_symlink() or not runtime.is_dir():
        raise ValueError(
            "provide a previously verified BOXFERRY_RENOVATE_PACKAGE; this check never acquires a runtime"
        )
    marker = runtime / "source-image.txt"
    if marker.is_symlink() or not marker.is_file():
        raise ValueError("cached Renovate image lacks its immutable selected-source identity")
    with marker.open("rb") as stream:
        identity = stream.read(4097)
    if len(identity) > 4096 or identity.decode("utf-8").strip() != pin:
        raise ValueError("cached Renovate image must match the selected immutable source")
    renovate_runtime._verify(runtime, version, renovate_runtime.run_command)
    node = shutil.which("node")
    if node is None:
        raise ValueError("the selected Node executable is required for native contract evidence")
    script = r"""
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const [runtimeArg, sourceArg, selectedArg, outputArg] = process.argv.slice(1);
const [runtime, source, selected, output] = [runtimeArg, sourceArg, selectedArg, outputArg].map(value => path.resolve(value));
const NL = String.fromCharCode(10);
const load = async name => import(pathToFileURL(path.join(runtime, 'dist', name)));
const pin = JSON.parse(fs.readFileSync(path.join(selected, 'automation/renovate-runtime.json'), 'utf8')).image;
assert.equal(fs.readFileSync(path.join(runtime, 'source-image.txt'), 'utf8').trim(), pin);
const version = JSON.parse(fs.readFileSync(path.join(runtime, 'package.json'), 'utf8')).version;
assert.equal(version, pin.split(':').slice(-2)[0].split('@')[0]);
const { init } = await load('logger/index.js');
await init();
const { regexEngineStatus } = await load('util/regex.js');
assert.equal(regexEngineStatus.type, 'available');
const { GlobalConfig } = await load('config/global.js');
const scratch = path.join(path.dirname(output), 'native-node-scratch');
fs.mkdirSync(scratch, { recursive: true });
GlobalConfig.set({ localDir: scratch, cacheDir: scratch, baseDir: source });
const { extractPackageFile: actions } = await load('modules/manager/github-actions/extract.js');
const { extractPackageFile: regex } = await load('modules/manager/custom/regex/index.js');
const { applyPackageRules } = await load('util/package-rules/index.js');
const { matchRegexOrGlob } = await load('util/string-match.js');
const { resolveConfigPresets } = await load('config/presets/index.js');
const common = JSON.parse(fs.readFileSync(path.join(selected, 'renovate-presets/common.json'), 'utf8'));
const rust = JSON.parse(fs.readFileSync(path.join(selected, 'renovate-presets/rust.json'), 'utf8'));
const local = JSON.parse(fs.readFileSync(path.join(source, '.github/renovate.json'), 'utf8'));
const { config: base } = await resolveConfigPresets(common);
const config = {
  ...base, ...local,
  customManagers: [...base.customManagers, ...local.customManagers],
  packageRules: [...base.packageRules, ...rust.packageRules, ...local.packageRules],
};
const fileName = '.github/workflows/ci.yml';
const workflow = fs.readFileSync(path.join(source, fileName), 'utf8');
const declared = workflow.split(NL).filter(line => line.trim().startsWith('node-version:'));
assert.equal(declared.length, 0, 'consumer must not retain an independent Node input');
const extracted = await actions(workflow, fileName, config['github-actions'] ?? {});
const nativeRows = (extracted?.deps ?? []).filter(dep => dep.packageName === 'actions/node-versions');
const customRows = [];
for (const manager of config.customManagers) {
  if (manager.customType !== 'regex' || !(manager.managerFilePatterns ?? []).some(pattern => matchRegexOrGlob(fileName, pattern))) continue;
  for (const dep of regex(workflow, fileName, manager)?.deps ?? []) {
    if ((dep.packageName ?? dep.depName) === 'node' && dep.datasource === 'node-version') customRows.push(dep);
  }
}
assert.equal(nativeRows.length, 0, 'actual local shared-node caller must produce zero native Node-version rows');
assert.equal(customRows.length, 0, 'actual local shared-node caller must produce zero custom Node-version rows');
const active = [];
for (const dep of [...nativeRows, ...customRows]) {
  const effective = await applyPackageRules({ ...config, ...dep, packageFile: fileName, packageName: dep.packageName ?? dep.depName, updateType: 'patch' });
  if (effective.enabled !== false && !dep.skipReason) active.push(dep);
}
assert.equal(active.length, 0, 'consumer must have no independent Node lookup route');
// Raw native/custom rows are retained even when a narrow rule disables their update route.
// This proves fixed discovery and Action-SHA eligibility independently.
async function ownership(contents, file, configuration = config) {
  const found = [];
  async function retain(dependency, manager) {
    const effective = await applyPackageRules({ ...configuration, ...dependency, manager, packageFile: file, updateType: 'patch' });
    found.push({ manager, depName: dependency.depName, packageName: dependency.packageName,
      depType: dependency.depType, datasource: dependency.datasource,
      currentValue: dependency.currentValue, currentDigest: dependency.currentDigest,
      skipReason: dependency.skipReason, enabled: effective.enabled !== false,
      automerge: effective.automerge, approval: effective.dependencyDashboardApproval,
      minimumReleaseAge: effective.minimumReleaseAge,
      minimumReleaseAgeBehaviour: effective.minimumReleaseAgeBehaviour });
  }
  if ((!configuration.enabledManagers || configuration.enabledManagers.includes('github-actions')) && configuration['github-actions']?.enabled !== false) {
    for (const dependency of (await actions(contents, file, configuration['github-actions'] ?? {}))?.deps ?? []) await retain(dependency, 'github-actions');
  }
  if (!configuration.enabledManagers || configuration.enabledManagers.includes('custom.regex')) {
    for (const manager of configuration.customManagers) {
      if (manager.customType !== 'regex' || manager.enabled === false || !(manager.managerFilePatterns ?? []).some(pattern => matchRegexOrGlob(file, pattern))) continue;
      for (const dependency of regex(contents, file, manager)?.deps ?? []) await retain(dependency, manager.customType);
    }
  }
  return found;
}
const runnerRows = rows => rows.filter(row => row.datasource === 'github-runners');
const nativePythonRows = rows => rows.filter(row => row.manager === 'github-actions' && row.datasource === 'github-releases' && row.packageName === 'actions/python-versions' && row.depType === 'uses-with');
const actionRows = rows => rows.filter(row => row.manager === 'github-actions' && row.depType === 'action' && /^[a-f0-9]{40}$/.test(row.currentDigest ?? ''));
function ordinaryPolicy(row) {
  assert.equal(row.enabled, true, 'ordinary update must remain enabled');
  assert.equal(row.automerge, true);
  assert.equal(row.approval, false);
  assert.equal(row.minimumReleaseAge, '3 days');
}
function uniqueLiteralRunners(contents, rows) {
  const labels = [...contents.matchAll(/^\s+runs-on:\s*["']?((?:ubuntu|macos|windows)-[A-Za-z0-9.-]+)["']?\s*$/gm)].map(match => match[1]);
  assert.ok(labels.length > 0, 'positive runner control needs actual literal declarations');
  const runners = runnerRows(rows);
  assert.equal(runners.length, labels.length, 'each literal runner must have one discovered owner');
  assert.ok(runners.every(row => row.manager === 'github-actions'), 'literal runners must have only their native owner');
  assert.deepEqual(runners.map(row => row.depName + '-' + row.currentValue).sort(), labels.sort());
  for (const row of runners) ordinaryPolicy(row);
  return runners;
}
const ordinaryRunnerFiles = [];
for (const file of fs.readdirSync(path.join(source, '.github/workflows')).filter(name => /\.ya?ml$/.test(name)).sort()) {
  const filePath = '.github/workflows/' + file;
  const contents = fs.readFileSync(path.join(source, filePath), 'utf8');
  if (!/^\s+runs-on:\s*["']?(?:ubuntu|macos|windows)-[A-Za-z0-9.-]+["']?\s*$/m.test(contents)) continue;
  const rows = await ownership(contents, filePath);
  ordinaryRunnerFiles.push({ file: filePath, rows: uniqueLiteralRunners(contents, rows) });
}
assert.ok(ordinaryRunnerFiles.some(file => file.file === fileName));
const providerFile = '.github/workflows/provider-conformance.yml';
const provider = fs.readFileSync(path.join(source, providerFile), 'utf8');
function fixedProvider(rows) {
  const nativePython = nativePythonRows(rows);
  const customPython = rows.filter(row => row.manager === 'regex' && row.datasource === 'python-version' && row.depName === 'python');
  assert.equal(nativePython.length, 1, 'fixed provider native Python must remain discoverable');
  assert.equal(nativePython[0].enabled, false, 'fixed provider native Python must not update');
  assert.equal(customPython.length, 1, 'fixed provider custom Python must remain discoverable');
  assert.equal(customPython[0].enabled, false);
  assert.equal(nativePython[0].currentValue, customPython[0].currentValue);
  const matrix = runnerRows(rows);
  assert.equal(matrix.length, 1, 'annotated provider matrix must retain its one discovery owner');
  assert.equal(matrix[0].manager, 'regex');
  assert.equal(matrix[0].enabled, false);
  const actions = actionRows(rows);
  const declared = [...provider.matchAll(/^\s+uses:\s+[^\s]+@([a-f0-9]{40})\s/gm)];
  assert.equal(actions.length, declared.length, 'every provider Action SHA must remain discoverable');
  assert.ok(actions.length > 0);
  for (const row of actions) ordinaryPolicy(row);
  assert.equal(actions.filter(row => row.depName === 'actions/setup-python').length, 1);
  return { nativePython, customPython, matrix, actions };
}
const providerOwnership = fixedProvider(await ownership(provider, providerFile));
// Only the path changes in this inert source fixture: ordinary setup-python remains automatic.
const ordinaryPythonFile = '.github/workflows/ordinary-python-owner-fixture.yml';
const ordinaryPython = nativePythonRows(await ownership(provider, ordinaryPythonFile));
assert.equal(ordinaryPython.length, 1);
ordinaryPolicy(ordinaryPython[0]);
const fixedNativeRule = config.packageRules.find(rule => rule.matchManagers?.includes('github-actions') && rule.matchPackageNames?.includes('actions/python-versions') && rule.matchFileNames?.includes(providerFile));
assert.ok(fixedNativeRule);
const withoutNativeHold = { ...config, packageRules: config.packageRules.filter(rule => rule !== fixedNativeRule) };
const reopenedProvider = await ownership(provider, providerFile, withoutNativeHold);
assert.equal(nativePythonRows(reopenedProvider)[0].enabled, true);
assert.throws(() => fixedProvider(reopenedProvider), /fixed provider native Python must not update/);
const customHold = config.packageRules.find(rule => rule.enabled === false && rule.matchManagers?.includes('custom.regex') && rule.matchFileNames?.includes(providerFile));
assert.ok(customHold);
const broadHold = { ...config, packageRules: config.packageRules.map(rule => rule === customHold ? { ...rule, matchManagers: ['custom.regex', 'github-actions'] } : rule) };
const broadlyBlocked = await ownership(provider, providerFile, broadHold);
assert.ok(actionRows(broadlyBlocked).some(row => !row.enabled));
assert.throws(() => fixedProvider(broadlyBlocked), /ordinary update must remain enabled/);
const matrixOwner = config.customManagers.find(manager => manager.description === 'Track fixed GitHub-hosted runner environments');
assert.ok(matrixOwner);
const withoutMatrix = { ...config, customManagers: config.customManagers.filter(manager => manager !== matrixOwner) };
const lostMatrix = await ownership(provider, providerFile, withoutMatrix);
assert.equal(runnerRows(lostMatrix).length, 0);
assert.throws(() => fixedProvider(lostMatrix), /annotated provider matrix must retain/);
// Deliberately reintroduce the retired overlapping pattern as a negative configuration fixture.
const overlappingLiteral = String.raw`(?:^|\n)\s*runs-on:\s*(?<depName>ubuntu)-(?<currentValue>[0-9]+(?:\.[0-9]+)?(?:-[a-z0-9]+)*)\s*(?:\n|$)`;
const duplicateOwners = { ...config, customManagers: config.customManagers.map(manager => manager === matrixOwner ? { ...manager, matchStrings: [overlappingLiteral, ...manager.matchStrings] } : manager) };
const duplicateRunners = await ownership(workflow, fileName, duplicateOwners);
assert.throws(() => uniqueLiteralRunners(workflow, duplicateRunners), /one discovered owner/);
const disabledNative = { ...config, 'github-actions': { ...config['github-actions'], enabled: false } };
const absentRunners = await ownership(workflow, fileName, disabledNative);
assert.throws(() => uniqueLiteralRunners(workflow, absentRunners), /one discovered owner/);
const ownerControls = {
  ordinaryRunnerFiles, providerOwnership, ordinaryPython,
  negativeCases: ['missing fixed native Python exclusion', 'overbroad exclusion blocks Action SHA updates', 'removed annotated matrix owner', 'duplicate literal runner owner', 'disabled native runner owner'],
  rejectedDuplicateRunnerRows: runnerRows(duplicateRunners),
  rejectedBroadProviderActionRows: actionRows(broadlyBlocked),
  retainedMatrixDiscoveryRows: providerOwnership.matrix,
};
const policyCases = [];
for (const [label, dependency, expectedAge, fixed] of [
  ['ordinary Rust', { manager: 'cargo', datasource: 'crate', packageName: 'serde', packageFile: 'Cargo.toml' }, '3 days', false],
  ['exact first-party release', { manager: 'cargo', datasource: 'crate', packageName: 'podman-lens', packageFile: 'Cargo.toml' }, '0 days', false],
  ['similarly named third-party crate', { manager: 'cargo', datasource: 'crate', packageName: 'podman-lens-extra', packageFile: 'Cargo.toml' }, '3 days', false],
  ['fixed provider bootstrap', { manager: 'custom.regex', datasource: 'pypi', packageName: 'PyYAML', packageFile: 'conformance/provider-config-matrix.toml' }, '3 days', true],
  ['ordinary development Python', { manager: 'pep621', datasource: 'pypi', packageName: 'ruff', packageFile: 'pyproject.toml' }, '3 days', false],
]) {
  const effective = await applyPackageRules({ ...config, ...dependency, updateType: 'patch' });
  assert.equal(effective.minimumReleaseAge, expectedAge, label);
  assert.equal(effective.dependencyDashboardApproval, false, label);
  assert.equal(effective.enabled === false, fixed, label);
  if (!fixed) assert.equal(effective.automerge, true, label);
  policyCases.push({ label, dependency, enabled: effective.enabled !== false, automerge: effective.automerge, approval: effective.dependencyDashboardApproval, minimumReleaseAge: effective.minimumReleaseAge });
}
const report = { runtime: version, selectedRuntimeImage: pin, nativeRows, customRows, active, ownerControls, policyCases, scope: 'real frozen native/custom extraction and effective rules; Node inactivity, unique literal runner owners, disabled qualified provider Python/matrix routes, preserved Action SHA and ordinary Python updates; no registry lookup, updates, engine operations or repository writes' };
fs.writeFileSync(output, JSON.stringify(report, null, 2) + NL);
console.log(JSON.stringify({ nativeNodeRows: nativeRows.length, customNodeRows: customRows.length, activeNodeLookups: active.length }));

"""
    with tempfile.TemporaryDirectory(prefix="compose-native-node-") as temporary:
        output = Path(temporary) / "native-owner.json"
        result = subprocess.run(
            [
                node,
                "--input-type=module",
                "-e",
                script,
                str(runtime.resolve()),
                str(ROOT),
                str(selected),
                str(output),
            ],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
            timeout=60,
            env={**os.environ, "LOG_LEVEL": "fatal"},
        )
        if result.returncode:
            raise ValueError(
                "native extraction/effective-policy contract failed: "
                + result.stdout
                + result.stderr
            )
        report: dict[str, object] = json.loads(output.read_text())
        return report


if __name__ == "__main__":
    if len(sys.argv) in (2, 3) and sys.argv[1] == "--native-renovate-contract":
        provided = (
            sys.argv[2] if len(sys.argv) == 3 else os.environ.get("BOXFERRY_RENOVATE_PACKAGE")
        )
        if not provided:
            raise SystemExit(
                "--native-renovate-contract requires BOXFERRY_RENOVATE_PACKAGE or an explicit verified-cache path; no runtime is acquired"
            )
        cached = Path(provided)
        print(json.dumps(verify_cached_native_node_owner(cached), indent=2))
    else:
        unittest.main()
