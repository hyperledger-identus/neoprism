# NeoPRISM UI designs

Prism is the selected visual direction for the NeoPRISM node UI. These are
editable static mockups, not implemented pages or wired interactions.

## Design deliverables

- [neoprism.fig](neoprism.fig) — editable design library; open in OpenPencil.
  Start on **Page 1** for the guide, then use the Resolver, Explorer and
  Components pages.
- [neoprism.svg](neoprism.svg) — original NeoPRISM logo.
- [PRISM-SPEC.md](PRISM-SPEC.md) — visual system, feature parity, intended
  interactions and implementation constraints.

The library contains desktop and mobile Resolver and Explorer screens, component
specimens and state examples. API Docs retains its navigation link; the generated
Scalar page is outside the redesign scope. Sample values are not live telemetry.

## Repository policy

Only the final design, logo, this README and the specification are tracked by
default. Prototype sources, alternative designs, previews, evidence, review
artifacts and `workspace/` are local-only and ignored by Git.

Add an explicit exception in the root `.gitignore` only when another durable
deliverable needs to be shared. Avoid committing duplicate design files or
generated exports that can be recreated from the final design.
