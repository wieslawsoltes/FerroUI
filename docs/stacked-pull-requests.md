# Stacked pull requests

A stack is a chain of pull requests where each is based on the branch of the one below it and the lowest is based on `main`: a large change split into parts that are reviewed one at a time, the second written while the first is still in review. This page says how the repository supports them and how a stack is merged.

## What the repository does

- **Checks for every pull request of a stack.** The continuous integration runs on a pull request whatever its base (`.github/workflows/ci.yml` has no branch filter on `pull_request`), so the parts above the bottom are checked against the branch below them.
- **Retargeting when the one below merges.** The repository deletes the head branch of a merged pull request (`delete_branch_on_merge`), and GitHub then changes the base of the pull requests that were based on it to the base of the merged one: the next part of the stack becomes based on `main` by itself.
- **Checks again after a retarget.** The workflow also runs on `edited` when the base changed, so a retargeted pull request is checked against `main` before it is merged, not only against the branch it had. Every job starts with that condition, and an edit of the title or the description runs nothing (and, with a concurrency group of its own, does not cancel a run in progress).
- **Merge commits only for a stack.** A squash or a rebase of a part rewrites its commits, and every pull request above it then carries commits that `main` does not have. Merge a stack with merge commits; `scripts/stack/stack.py` does.

## Making a stack

```bash
git switch -c part-1 main            # the first part
gh pr create --base main
git switch -c part-2 part-1          # the second, on the first
gh pr create --base part-1
```

A change to a lower part is merged or rebased into the parts above it as for any branch; `git rebase --update-refs` keeps the branches of a stack together when it is rebased as a whole.

## Merging a stack

```bash
scripts/stack/stack.py list 290             # the stack of #290, bottom first, with the state of each part
scripts/stack/stack.py merge 290 --dry-run  # what would be merged
scripts/stack/stack.py merge 290            # merges from the bottom while each part is ready
scripts/stack/stack.py merge 290 --wait     # and waits for the checks of the next part between merges
```

A part is merged only when its base is `main`, GitHub reports it mergeable without conflicts, it is not a draft, and the latest run of every check of its head commit passed and started after the last change of its base. A check whose latest run was cancelled, because a newer run of the workflow superseded it (the retarget starts one), is not a failure: `--wait` waits for a newer run of it, and stops at the timeout if none comes. It is merged with a merge commit and only if its head is still the commit that was checked. After a merge the script waits for GitHub to retarget the next part (and retargets it itself after two minutes); without `--wait` it then stops, because the checks of that part run again, and the same command continues the stack when they pass.
