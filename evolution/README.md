# Burrow Evolution

Burrow Evolution Proposals (BEPs) are the repository's durable design record for protocol work, control-plane changes, forge infrastructure, and operational policy.

## Goals

1. Capture intent before implementation outruns the architecture.
2. Give contributors and agents enough context to work safely without re-discovering prior decisions.
3. Tie ambitious work to concrete validation, rollout, and rollback criteria.

## When a BEP is required

Open a BEP for:

- new transports or protocol families
- control-plane and identity changes
- deployment, forge, runner, or secrets changes
- data model migrations
- user-visible behavior that changes security or routing semantics

Small bug fixes and isolated refactors do not need a BEP unless they materially change one of the areas above.

## Lifecycle

1. Pitch
   Capture the problem and why it matters now.
2. Draft
   Copy `evolution/proposals/0000-template.md` to `evolution/proposals/BEP-XXXX-short-slug.md`.
3. Review
   Collect feedback, tighten the design, and document unresolved concerns.
4. Decision
   Mark the proposal `Accepted`, `Rejected`, or `Returned for Revision`.
5. Implementation
   Link code changes, tests, and rollout evidence.
6. Supersession
   Keep historical proposals in-tree and point forward to the replacing BEP.

## Status Values

- `Pitch`
- `Draft`
- `In Review`
- `Accepted`
- `Implemented`
- `Rejected`
- `Returned for Revision`
- `Superseded`
- `Archived`

## Layout

```text
evolution/
  README.md
  proposals/
    0000-template.md
    BEP-0001-...
```

Use ASCII Markdown. Keep metadata at the top of each proposal so tooling and future agents can parse it quickly.
