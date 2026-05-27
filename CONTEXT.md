# DFTFlow

DFTFlow coordinates high-throughput first-principles workflows and their post-processing summaries.

## Language

**Phonon Stability**:
A binary classification of a computed structure as **stable** or **unstable** based on the minimum phonon DOS frequency.
_Avoid_: Dynamical stability, imaginary-frequency status

**Unstable Phonon Case**:
A computed structure whose minimum phonon DOS frequency is lower than -1 THz.
_Avoid_: Has imaginary frequency, failed phonon

**Stable Phonon Case**:
A computed structure whose minimum phonon DOS frequency is not lower than -1 THz.
_Avoid_: No imaginary frequency, passed phonon

## Relationships

- A **Phonon Stability** classification belongs to exactly one pressure point of one substituted structure.
- An **Unstable Phonon Case** and a **Stable Phonon Case** are mutually exclusive outcomes of **Phonon Stability**.

## Example Dialogue

> **Dev:** "If a case has a small negative phonon DOS frequency, is it unstable?"
> **Domain expert:** "Only if the minimum phonon DOS frequency is lower than -1 THz; otherwise the **Phonon Stability** is **stable**."

## Flagged Ambiguities

- "虚频" was initially framed as vibration frequency in general, then resolved to mean the frequency grid in the phonon DOS summary used by DFTFlow's post-processing.
