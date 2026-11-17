# Issue tracker: GitHub

Issues and specs live in GitHub Issues for `hyperledger-identus/neoprism`.
Use the `gh` CLI from this clone.

## Conventions

- Search existing issues before creating a ticket:
  `gh issue list --state all --search "<terms>"`.
- Confirm the target before the first issue creation in a session:
  `gh repo view --json nameWithOwner`.
- Create: `gh issue create --title "..." --body-file <file>`.
  Use a temporary file or heredoc for multiline bodies.
- Read: `gh issue view <number> --json number,title,body,labels,comments`.
- List: `gh issue list --state open --json number,title,labels`.
- Comment: `gh issue comment <number> --body "..."`.
- Label: `gh issue edit <number> --add-label "..."` or
  `--remove-label "..."`. Use `docs/agents/triage-labels.md`.
- Close: post the explanation with `gh issue comment`, then
  `gh issue close <number>`. Comment separately even if already closed.

Use native GitHub sub-issues and issue dependencies when supported by the
installed CLI and repository. Otherwise put `Part of #<parent>` and
`Blocked by: #<number>` at the top of the child issue body, and list children
in the parent's task list. A ticket is startable only when all blockers
are closed.

File issues only in the confirmed project repository, not the upstream
skills repository.

## Pull requests as a triage surface

**PRs as a request surface: no.**

## Skill instructions

- "Publish to the issue tracker": create a GitHub issue.
- "Fetch the relevant ticket": read the GitHub issue and its comments.
