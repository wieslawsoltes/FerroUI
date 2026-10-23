#!/usr/bin/env python3
"""Stacked pull requests: show a stack and merge it from the bottom up (docs/stacked-pull-requests.md).

A stack is a chain of open pull requests, each based on the head branch of the one below it, the
lowest based on the default branch. Name any pull request of the stack; the chain is followed down
from it to the default branch (and up from it, for `list`).

  scripts/stack/stack.py list 290            # the stack of #290, bottom first, with the state of each
  scripts/stack/stack.py merge 290           # merges the stack up to #290 while each one is ready
  scripts/stack/stack.py merge 290 --wait    # and waits for the checks of the next one between merges
  scripts/stack/stack.py merge 290 --dry-run # says what it would merge

A pull request is merged only when:
- its base is the default branch and GitHub says it can be merged without conflicts (CLEAN);
- every check of its head commit has ended in success, was skipped or neutral, and the latest run
  of each check started after the last change of its base (so a pull request that was retargeted
  from the branch below it is merged on checks against the default branch, not the old base);
  a check whose latest run was cancelled (a newer run of the workflow superseded it, as the retarget
  does) is not a failure: it is waited for, as a running one, until a newer run of it ends;
- it is not a draft.

It is merged with a merge commit (never squashed or rebased: that would rewrite the commits the
pull requests above it contain) and only if its head is still the commit that was checked
(`--match-head-commit`). The repository deletes the head branch of a merged pull request, and
GitHub then retargets the pull request above it to the default branch; the script waits for that
(and retargets it itself if GitHub has not within two minutes). The retarget runs the checks
again (`edited` with a change of the base in .github/workflows/ci.yml): without `--wait` the
script stops there and says so; with it, it polls until they end.

Needs the GitHub CLI (`gh`), signed in with write access to the repository.
"""

import argparse
import json
import subprocess
import sys
import time
from datetime import datetime, timezone

FIELDS = "number,title,state,isDraft,baseRefName,headRefName,headRefOid,mergeable,mergeStateStatus,statusCheckRollup,url"
GOOD = {"SUCCESS", "SKIPPED", "NEUTRAL"}
# A run cancelled by a newer run of its workflow (the concurrency group of the CI cancels the run of
# the old base when a retarget starts one on the new base): neither passed nor failed.
SUPERSEDED = {"CANCELLED", "STALE"}


def gh(*args, parse=True):
    result = subprocess.run(["gh", *args], capture_output=True, text=True)
    if result.returncode != 0:
        raise SystemExit(f"gh {' '.join(args)}: {result.stderr.strip()}")
    return json.loads(result.stdout) if parse else result.stdout


def repository():
    return gh("repo", "view", "--json", "nameWithOwner,defaultBranchRef")


def pull(number):
    return gh("pr", "view", str(number), "--json", FIELDS)


def pull_with_head(branch):
    found = gh("pr", "list", "--state", "open", "--head", branch, "--json", "number")
    return pull(found[0]["number"]) if found else None


def pulls_based_on(branch):
    found = gh("pr", "list", "--state", "open", "--base", branch, "--json", "number")
    return [pull(item["number"]) for item in found]


def stack_below(top, default):
    """The pull requests from the bottom of the stack up to `top`."""
    chain = [top]
    while chain[0]["baseRefName"] != default:
        below = pull_with_head(chain[0]["baseRefName"])
        if below is None:
            raise SystemExit(
                f"#{chain[0]['number']} is based on {chain[0]['baseRefName']}, which is neither "
                f"{default} nor the head of an open pull request: not a stack on {default}"
            )
        if any(p["number"] == below["number"] for p in chain):
            raise SystemExit(f"the bases of #{below['number']} make a cycle")
        chain.insert(0, below)
    return chain


def stack_above(top):
    """The pull requests above `top`, following the first one based on each head."""
    chain = []
    current = top
    while True:
        above = pulls_based_on(current["headRefName"])
        if not above:
            return chain
        if len(above) > 1:
            print(f"note: several pull requests are based on {current['headRefName']}; following #{above[0]['number']}")
        chain.append(above[0])
        current = above[0]


def parse_time(text):
    return datetime.fromisoformat(text.replace("Z", "+00:00")) if text else None


def base_changed_at(repo, number):
    """When the base of the pull request was last changed (None if it never was)."""
    lines = gh("api", "--paginate", f"repos/{repo}/issues/{number}/timeline", "--jq",
               '.[] | select(.event == "base_ref_changed") | .created_at', parse=False)
    return max((parse_time(line.strip()) for line in lines.splitlines() if line.strip()), default=None)


def checks(pr, since):
    """(ready, summary): whether every check passed on runs newer than `since`, and a summary."""
    latest = {}
    for item in pr["statusCheckRollup"] or []:
        if item.get("__typename") == "StatusContext":
            name, state, started = item.get("context"), item.get("state"), item.get("startedAt")
            done = state not in ("PENDING", "EXPECTED")
            conclusion = "SUCCESS" if state == "SUCCESS" else state
        else:
            name, started = item.get("name"), item.get("startedAt")
            done = item.get("status") == "COMPLETED"
            conclusion = item.get("conclusion") or ""
        started = parse_time(started)
        previous = latest.get(name)
        # A run that has not started yet is newer than one that has ended.
        newer = started is None and not done and previous is not None and previous[1]
        if previous is None or newer or (started and previous[0] and started > previous[0]):
            latest[name] = (started, done, conclusion)
    if not latest:
        return False, "no checks"
    pending = [n for n, (_, done, _) in latest.items() if not done]
    cancelled = [n for n, (_, done, c) in latest.items() if done and c in SUPERSEDED]
    failed = [n for n, (_, done, c) in latest.items() if done and c not in GOOD and c not in SUPERSEDED]
    stale = [n for n, (s, _, _) in latest.items() if since and (s is None or s < since)]
    passed = len(latest) - len(pending) - len(cancelled) - len(failed)
    summary = f"{passed}/{len(latest)} passed"
    if failed:
        summary += f", failed: {', '.join(sorted(failed))}"
    if pending:
        summary += f", {len(pending)} running"
    if cancelled:
        summary += f", cancelled (waiting for a newer run): {', '.join(sorted(cancelled))}"
    if stale:
        summary += f", {len(stale)} older than the last change of the base"
    return not pending and not cancelled and not failed and not stale, summary


def readiness(repo, pr, default):
    """(ready, reasons) of a pull request for merging now."""
    reasons = []
    if pr["state"] != "OPEN":
        reasons.append(pr["state"].lower())
    if pr["isDraft"]:
        reasons.append("a draft")
    if pr["baseRefName"] != default:
        reasons.append(f"based on {pr['baseRefName']}")
    if pr["mergeStateStatus"] != "CLEAN":
        reasons.append(f"merge state {pr['mergeStateStatus']}")
    ready, summary = checks(pr, base_changed_at(repo, pr["number"]))
    if not ready:
        reasons.append(f"checks: {summary}")
    return not reasons, reasons, summary


def wait_for_retarget(number, default, seconds=120):
    deadline = time.time() + seconds
    while time.time() < deadline:
        pr = pull(number)
        if pr["baseRefName"] == default:
            return pr
        time.sleep(5)
    print(f"#{number} was not retargeted by GitHub; retargeting it to {default}")
    gh("pr", "edit", str(number), "--base", default, parse=False)
    return pull(number)


def command_list(args):
    repo = repository()
    default = repo["defaultBranchRef"]["name"]
    top = pull(args.pr)
    chain = stack_below(top, default) + stack_above(top)
    print(f"the stack on {default}, bottom first:")
    for pr in chain:
        _, reasons, summary = readiness(repo["nameWithOwner"], pr, default)
        state = "ready to merge" if not reasons else "; ".join(reasons)
        print(f"  #{pr['number']} {pr['headRefName']} -> {pr['baseRefName']}: {pr['title']}")
        print(f"      checks {summary}; {state}")


def command_merge(args):
    repo = repository()
    name, default = repo["nameWithOwner"], repo["defaultBranchRef"]["name"]
    chain = stack_below(pull(args.pr), default)
    print("merging, bottom first: " + ", ".join(f"#{pr['number']}" for pr in chain))
    for index, pr in enumerate(chain):
        pr = pull(pr["number"])
        if index > 0 and pr["baseRefName"] != default:
            pr = wait_for_retarget(pr["number"], default)
        deadline = time.time() + args.timeout * 60
        while True:
            ready, reasons, _ = readiness(name, pr, default)
            if ready:
                break
            if not args.wait or any(r.startswith("checks:") and "failed" in r for r in reasons) or time.time() > deadline:
                print(f"#{pr['number']} is not ready: {'; '.join(reasons)}")
                if index > 0:
                    print(f"merged #{', #'.join(str(p['number']) for p in chain[:index])}; run this again when #{pr['number']} is ready")
                return 1
            print(f"#{pr['number']}: waiting ({'; '.join(reasons)})")
            time.sleep(60)
            pr = pull(pr["number"])
        if args.dry_run:
            print(f"would merge #{pr['number']} at {pr['headRefOid'][:10]} ({pr['title']})")
            if index + 1 < len(chain):
                print(f"would then wait for #{chain[index + 1]['number']} to be retargeted to {default} and checked again; stopping")
            return 0
        gh("pr", "merge", str(pr["number"]), "--merge", "--match-head-commit", pr["headRefOid"], parse=False)
        print(f"merged #{pr['number']} ({pr['title']})")
    print("the stack is merged")
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = parser.add_subparsers(dest="command", required=True)
    show = sub.add_parser("list", help="show the stack of a pull request")
    show.add_argument("pr", type=int)
    merge = sub.add_parser("merge", help="merge the stack up to a pull request, bottom first")
    merge.add_argument("pr", type=int)
    merge.add_argument("--wait", action="store_true", help="wait for the checks of each pull request")
    merge.add_argument("--timeout", type=int, default=180, help="minutes to wait for one pull request (with --wait)")
    merge.add_argument("--dry-run", action="store_true", help="say what would be merged first, and merge nothing")
    args = parser.parse_args()
    return command_list(args) if args.command == "list" else command_merge(args)


if __name__ == "__main__":
    sys.exit(main() or 0)
