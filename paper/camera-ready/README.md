# ArgorixLang JCC 2026 camera-ready

This directory contains the condensed IEEEtran conference version prepared in
response to the JCC 2026 reviews. The long reproducibility manuscript remains
under `paper/`; this version is limited to the conference page budget and cites
the repository for code, row-level results, and artifacts.

## Evaluation

The end-to-end and adversarial campaign specified in
`adversarial-evaluation-plan.md` has been **executed**. Its implementation,
cases, oracle, raw rows, and checksums live in `evaluation/adversarial/`, and
its execution record is section 10 of the plan.

Every quantitative statement in `main.tex` comes from that campaign through
generated files — none is typed by hand:

| Generated file | Source |
| --- | --- |
| `tables/campaign-facts.tex` | macros the prose cites |
| `tables/campaign-results.tex` | Table II |
| `tables/tamper-by-class.tex` | Table III |
| `tables/snapshot-remeasured.tex` | Table I |
| `figures/adversarial-boundary.pdf` | Figure 2 |

Regenerate all of them with:

```bash
cargo build --locked --release \
  -p argorixc -p argorix-vm -p argorix-conformance -p argorix-sign
cargo build --release -p argorix-vm \
  --target-dir target/eval-tripwire --features eval-tripwire
python evaluation/adversarial/run.py --clean
```

The second build is the evaluation binary used to observe the VM's mediation
point. It is never the release: the release build does not compile the feature
and rejects its flags.

`run.py` runs the harness-validity (anti-circularity) test first and aborts the
campaign if it fails.

## Building the PDF

MiKTeX or TeX Live:

```bash
pdflatex -output-directory=build main.tex
bibtex build/main
pdflatex -output-directory=build main.tex
pdflatex -output-directory=build main.tex
```

Tectonic also works:

```bash
tectonic -X compile main.tex
```

The submission package includes `figures/system-pipeline.pdf`,
`figures/adversarial-boundary.pdf`, `tables/*.tex`, `main.tex`, and
`references.bib`.

## Validation targets, and the state of each

| Target | State |
| --- | --- |
| IEEEtran conference class, US Letter, two columns | met |
| No more than eight pages including references | 7 pages, one in reserve |
| All citations and cross-references resolved | 0 undefined |
| No overfull boxes | 0 (1 underfull: an unbreakable DOI URL) |
| Embedded fonts | 13 font objects, all embedded |
| Visual inspection of every rendered page | all 7 inspected at 144 dpi |
| Campaign reproduces from clean without manual edits | two identical runs |
| Workspace tests, fmt and clippy | 382 passing, both clean |

## Claim boundaries

The paper reports proportions as `n/N` with Wilson 95% intervals and gives a
rule-of-three upper bound wherever a numerator is zero. It states plainly that
the prompt-injection result is containment of a proposal made by a model outside
the system rather than resistance of an Argorix agent loop, that a coordinated
unsigned replacement of
the artifact set is not detected, that the declared sandboxed operation is not
rejected at a boundary, and that non-reachability of a real external adapter is
out of scope because the provider registry admits no instrumentable
implementation. Those statements must survive any future edit.

Three defects the campaign exposed — an unchecked provider payload type, the
absence of source binding, and the absence of any producer binding — were fixed
rather than written around. The pre-fix campaign is archived under
`evaluation/adversarial/baseline/prefix/`, and the paper reports both the before
and the after.
