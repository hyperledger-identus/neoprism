# Domain docs

## Layout and reading rules

This repository uses a single shared domain context across its crates
and embedded wallet.

Before exploring domain concepts or proposing architectural changes:

- Read root `GLOSSARY.md`.
- Read decisions in `docs/adr/` relevant to the work.

If these files or directories do not exist, proceed silently.
Create them lazily through domain-modeling work when terminology or
decisions are resolved, rather than scaffolding empty documents.

## Vocabulary

Use glossary terms in issue titles, proposals, hypotheses, and test names.
If a needed concept is absent, reconsider the term or note the gap for
domain-modeling work.

## Architectural decisions

Explicitly flag proposals that contradict an existing ADR. Identify the
ADR and explain why reopening the decision is warranted.
