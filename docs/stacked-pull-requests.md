# Stacked pull requests

A stack is a chain of pull requests where each is based on the branch of the one below it and the lowest is based on `main`: a large change split into parts that are reviewed one at a time, the second written while the first is still in review. This page says how the repository supports them and how a stack is merged.

## What the repository does

- **Checks for every pull request of a stack.** The continuous integration runs on a pull request whatever its base (`.github/workflows/ci.yml` has no branch filter on `pull_request`), so the parts above the bottom are checked against the branch below them.
- **Retargeting when the one below merges.** The repository deletes the head branch of a merged pull request (`delete_branch_on_merge`), and GitHub then changes the base of the pull requests that were based on it to the base of the merged one: the next part of the stack becomes based on `main` by itself.
- **Checks again after a retarget.** `scripts/stack/stack.py` brings a retargeted pull request up to date with `main` (`gh pr update-branch`, a merge commit), so its new head is checked against `main` before it is merged, not only against the branch it had. The workflow does not run on `edited`: a run for an edit of the title skipped every job, and those skipped checks replaced the results of the real run of the same commit, so a failure looked like a skip. After retargeting a pull request by hand, run `gh pr update-branch <number>`.
- **Merge commits only for a stack.** A squash or a rebase of a part rewrites its commits, and every pull request above it then carries commits that `main` does not have. Merge a stack with merge commits; `scripts/stack/stack.py` does.

## GitHub stacks (the fast way)

The repository uses GitHub's stacked pull requests (the `gh stack` extension, `github/gh-stack`). A stack on GitHub shows its parts together and merges them in one atomic operation, so a part above does not wait for a retarget and a second run of its checks after the part below merges.

- **Make a stack.** Create the branches as before and open their pull requests, each based on the branch below. Then link them, bottom first: `gh stack link 361 362` (or `gh stack init` and `gh stack add` for new branches, `gh stack submit` to open their pull requests).
- **Land a stack.** `scripts/stack/stack.py land <any part>`. It takes the highest part that can land (based on the branch below it, not a draft, no conflict) and decides on the checks of that top part alone, because its head contains every part below it. The checks of the lower parts may still be running or may have failed. Two guards apply: the head of every lower part must be an ancestor of the top head (merge it up if not), and if `main` has moved since the top was last merged with it, `main` must not have changed files of the stack (`CHANGELOG.md` aside; otherwise merge `main` into the top and let its checks run). It then merges every part up to the top into `main` at once (`gh stack merge <pr> --merge --yes`, merge commits). `--dry-run` says what it would land. GitHub itself checks only that the parts are open and not drafts, so this script is what keeps a failing stack out.
- **CI of the lower parts.** A part with an open pull request above it runs only the changed-areas and conventions checks of `ci.yml`, plus a marker check, `Reduced CI (the pull request above runs the full checks)`. The top part runs the full jobs. The workflows of a single area still run on their own paths. If a lower part has to land as the top (the parts above stay drafts or were closed), `land` refuses it until `gh workflow run ci.yml --ref <branch>` has run the full checks on its head. A pull request based on a stack branch that is not a part of the stack, such as an experiment, carries the label `stack-independent`, so it does not reduce the CI of the branch below it.
- **Keep branches with merge commits.** Bring a stack up to date by merging `main` into the lowest branch and each branch into the one above it. Do not use `gh stack sync` or `gh stack rebase`: they rebase every branch and force-push, which rewrites commits that other workers and reviews refer to.

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
