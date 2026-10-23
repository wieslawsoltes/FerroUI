#!/usr/bin/env python3
"""Extract executed libtest cases and results from the exact native CI log."""
import hashlib
import json
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent
source = json.loads((ROOT / "source-provenance.json").read_text())
raw = (ROOT / "job.log").read_bytes()
text = raw.decode("utf-8")
ansi = re.compile(r"\x1b\[[0-?]*[ -/]*[@-~]")
timestamp = re.compile(r"^\d{4}-\d{2}-\d{2}T[0-9:.]+Z\s+")
test_line = re.compile(r"^test (\S+)(?: - should panic)? \.\.\. (.*)$")
result_line = re.compile(r"^test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out; finished in ([0-9.]+)s")
suites = []
current = None
pending_case = None
failures = []
checkout_proof = []
gpu_required_lines = []
for line_number, original in enumerate(text.splitlines(), 1):
    line = timestamp.sub("", ansi.sub("", original))
    if source["commit"] in line:
        checkout_proof.append({"line": line_number, "text": line})
    if "FERROUI_VELLO_REQUIRE_GPU" in line:
        gpu_required_lines.append({"line": line_number, "text": line})
    match = re.fullmatch(r"running (\d+) tests?", line)
    if match:
        current = {"start_line": line_number, "announced_test_count": int(match[1]), "tests": []}
        pending_case = None
        suites.append(current)
        continue
    match = test_line.match(line)
    if match and current is not None:
        outcome = re.match(r"^(ok|FAILED|ignored)(?:\s|$)", match[2])
        pending_case = {"name": match[1], "outcome": outcome[1] if outcome else "pending", "line": line_number, "suffix": match[2]}
        current["tests"].append(pending_case)
        if outcome:
            pending_case = None
    elif pending_case is not None and line.strip() in ("ok", "FAILED", "ignored"):
        pending_case["outcome"] = line.strip()
        pending_case["outcome_line"] = line_number
        pending_case = None
    match = result_line.match(line)
    if match and current is not None:
        current["result"] = {"outcome": match[1], "passed": int(match[2]), "failed": int(match[3]), "ignored": int(match[4]), "measured": int(match[5]), "filtered_out": int(match[6]), "duration_seconds": float(match[7]), "line": line_number}
        current = None
    if re.search(r"(^error(?:\[|:)|^test .* \.\.\. FAILED|^test result: FAILED|Process completed with exit code [1-9])", line):
        failures.append({"line": line_number, "text": line})

all_tests = [case for suite in suites for case in suite["tests"]]
new_tests = {}
for name in source["new_regression_test_names"]:
    full = "text_box_tests::" + name
    new_tests[full] = [case for case in all_tests if case["name"] == full]

def identify(suite):
    names = [case["name"] for case in suite["tests"]]
    if names and all(name.startswith("gpu::fragment_tests::") for name in names):
        return "required_gpu_fragments"
    if names and all(name.startswith("text_box_tests::") for name in names):
        return "full_text_box" if len(names) > 50 else "text_box_subset"
    if names and all(name.startswith(("text_box_tests::", "masked_text_box_tests::")) for name in names):
        return "full_text_box_filter_including_masked"
    if names and all(name.startswith("text_box::text_box_history::") for name in names):
        return "native_text_history"
    return "inspect_names"

for suite in suites:
    suite["identity_hint"] = identify(suite)
    result = suite.get("result")
    suite["test_lines_match_announced_count"] = len(suite["tests"]) == suite["announced_test_count"]
    suite["result_count_matches_announced_count"] = bool(result and result["passed"] + result["failed"] + result["ignored"] + result["measured"] == suite["announced_test_count"])

report = {
    "repository": source["repository"], "commit": source["commit"], "tree": source["tree"],
    "log": {"file": "job.log", "bytes": len(raw), "sha256": hashlib.sha256(raw).hexdigest()},
    "suite_count": len(suites), "suites": suites, "new_regressions": new_tests,
    "all_14_new_regressions_executed_and_passed": all(cases and all(case["outcome"] == "ok" for case in cases) for cases in new_tests.values()),
    "failure_lines": failures, "commit_occurrences": checkout_proof, "required_gpu_environment_occurrences": gpu_required_lines,
    "scope": "Parsed actual job-log libtest lines; compare run/job API provenance and source workflow before asserting final CI success."
}
(ROOT / "log-analysis.json").write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps({"suite_count": len(suites), "all_14_new_regressions_executed_and_passed": report["all_14_new_regressions_executed_and_passed"], "failure_lines": failures, "suites": [{"start_line": suite["start_line"], "count": suite["announced_test_count"], "result": suite.get("result"), "identity_hint": suite["identity_hint"]} for suite in suites]}, indent=2))
