---
id: TASK-1
title: Upgrade images version in docker compose setup
status: To Do
assignee: []
created_date: '2026-03-06 05:04'
updated_date: '2026-03-06 05:23'
labels: []
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Review and upgrade container image versions used in the docker compose setup to their latest stable releases. Ensure compatibility by running integration tests and get approval before updating images that may introduce breaking changes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 All images that are safe to update are up-to-date
- [ ] #2 No regression introduced and verified by integration testing
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Check docker images for newer version, ask for approval for images that are not safe or might have breaking changes
<!-- SECTION:PLAN:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Images version in docker files are updated
- [ ] #2 Python script which is used to generate docker compose files are up-to-date
<!-- DOD:END -->
