# Karoowa Enterprise — Licensing Notice

Code under the `enterprise/` directory of this repository is **proprietary**
and is **not** covered by the Apache License 2.0 that governs the `core/`
directory.

## Status

The formal Karoowa Enterprise licence text (Business Source Licence 1.1
with a four-year conversion to Apache 2.0) is in legal review. Until it is
published in this file, the following terms apply.

## Until the licence is published

- All files under `enterprise/` are made available **for reading and review only**.
- You may **not** distribute, modify, run in production, or otherwise exercise
  any rights normally granted by an open-source licence.
- Every crate under `enterprise/` is marked `publish = false` and is **not**
  pushed to crates.io.
- Enterprise features require a signed licence file at runtime; see
  `enterprise/README.md` for details.

## Inquiries

For commercial licensing, evaluation copies, or any other enterprise-licensing
question, contact: **enterprise@karoowa.io**.

## Why publish the source at all?

Karoowa follows an open-core model. Publishing the enterprise source — even
under restrictive terms — gives operators full visibility into how their data
is handled and lets the community audit the proprietary surface area. The
restriction is on *use*, not on *inspection*.
