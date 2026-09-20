# Embedding diagrams

**Who this is for:** Authors editing Takehome docs or Astro pages who need a consistent SVG figure block.

**When you finish:** You can drop a diagram ID from `docs/diagrams/` into a page with alt text and a caption.

Build or copy the SVG into the site static path (for example `web/site/public/diagrams/`). Use a figure block so screen readers get alt text and a caption:

```html
<figure>
  <img src="/diagrams/ID.svg" alt="TITLE" />
  <figcaption>CAPTION</figcaption>
</figure>
```

Set these placeholders:

- `ID`: file stem (for example `01-request-lifecycle`)
- `TITLE`: short description for alt text
- `CAPTION`: sentence under the image

Captions for numbered diagrams also live in `captions.json` beside the Mermaid sources.

**Last reviewed:** 2026-09-20  
**Engine:** 0.1.0
