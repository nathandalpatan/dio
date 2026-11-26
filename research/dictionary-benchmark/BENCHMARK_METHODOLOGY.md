# Dictionary benchmark methodology

## Corpus and reproducibility

`scripts/build_corpus.py` is the source for `benchmark_words.csv`. It contains a fixed, purposive challenge set of 500 distinct casefolded terms: 50 each in everyday, advanced, polysemous, modern slang, computing, academic/scientific, idioms, phrasal verbs, inflected/irregular, and New Zealand/British regional vocabulary. All 28 examples specified in the brief are present. The two contextual sentences for every polysemous term deliberately target different uses. Other sentences target one ordinary use. Sentences were authored for this benchmark and are **usage prompts, not reference definitions**.

The term lists were curated to exercise likely weaknesses; they are not a probability sample of English usage. Seed `20261125` shuffles fixed rows and drives blind packets and bootstrap resampling. It does not make the vocabulary representative. One term has exactly one primary stratum even if it fits several. Do not replace missed terms after viewing results. Record the corpus SHA-256 and source revision with each run.

## Retrieval configurations

| Configuration | Retrieval rule |
| --- | --- |
| `dio_offline` | The production `LocalDictionary::special` then `words` behavior, against the bundled SQLite databases, with internet disabled in the harness. |
| `dio_online` | The production `Dictionary::lookup` parser and merge, with an empty cache; online-provider failures are recorded as errors rather than treated as vocabulary misses. |
| `opengloss_standard` | Actual v2.4 canonical `neutral/plain` text, in released sense order. |
| `opengloss_simplified` | Actual v2.4 `grade_5` text for each sense where supplied, without silently replacing missing variants with neutral text. |
| `hybrid` | Pre-registered experimental offline rule: unflagged OpenGloss neutral senses in released order, then bundled OEWN fallback when OpenGloss has a verified miss. If OpenGloss acquisition fails, status is `not_retrieved`, not an asserted fallback result. |

Pin immutable dataset revisions and hashes, provider retrieval time, code commit and working-tree state. Preserve source text, examples, part of speech, source URLs, ordering, status, errors, and lookup timing in `raw_results.jsonl`. The harness uses current production modules without modifying the app. A provider timeout, blocked network, unavailable Parquet, or parser error is `error` or `not_retrieved`, never `missing`. Exact headword coverage checks the actual matching surface form; a lemma or acronym fallback is separately labelled. Compare independent source configurations before assessing the hybrid.

## Sense matching and measures

Match senses by intended context, part of speech, semantic content, and independent references. Do not equate OpenGloss and OEWN identifiers. Mark each contextual target `yes`, `no`, or `uncertain` for relevant-sense coverage. Review any ambiguous alignment. Assess first-definition accuracy against each context from the source's original ordering. A first sense that is valid for the word but wrong for the context is a first-definition failure, not a semantic error in that sense.

`scripts/evaluate.py` produces 2,500 per-configuration rows, seeded blind packets, an order packet, and an empty ratings template. It computes counts, exact duplicate-text candidates, headword-reuse candidates, lengths, and approximate Flesch Reading Ease and Flesch–Kincaid grade. These are **automated descriptors**. Duplicate text is not the same as a duplicate sense; repeating the headword is not necessarily circularity. Flesch metrics are unstable for short definitions and use heuristic syllables. Semantic errors, jargon, true circularity, relevant-sense coverage, and first-definition accuracy require editorial assessment. No script invents those judgments.

For each available entry, a blinded evaluator receives the query, contexts, part of speech, original examples, and definitions in randomized order without source labels. A separate source-masked order packet shows the original ordering for prioritisation and first-definition checks. Reviewers record URLs to independent dictionaries or technical references when asserting a semantic error, unsupported detail, or missing distinction. WordNet alone is not independent evidence for OpenGloss. Use at least two independent references for serious disputed claims where possible; label unresolved cases uncertain. An independent reviewer re-scores a stratified 20% sample and every alleged semantic error. Record evaluator type; this project uses AI-assisted rather than human editorial judgments unless a human actually participates.

### Weighted 0–5 rubric

Each criterion is scored separately. The table gives anchors for every integer; use half points only when evidence falls between anchors and explain the reason. `weighted_quality_0_100 = Σ(score × weight) / 5` only when all seven scores are recorded. A verified missing entry has zero availability but is excluded from conditional definition-quality averages; report coverage separately. Never treat an unattempted retrieval as a miss or assign it a quality score.

| Criterion (weight) | 0 | 1 | 2 | 3 | 4 | 5 |
| --- | --- | --- | --- | --- | --- | --- |
| Semantic accuracy (30%) | No relevant meaning or false definition | Core meaning wrong | Material distinction lost or unsupported claim | Intended meaning substantially correct with a gap | Correct with minor imprecision | Precise meaning and distinctions, no unsupported information |
| Readability (20%) | No usable text | Nearly incomprehensible | Requires substantial decoding | Understandable after some effort | Clear on first reading with a small obstacle | Clear on first reading for the intended general reader |
| Conciseness (15%) | No definition | Mostly padding or unusably terse | Significant excess or missing explanatory core | Acceptably sized with avoidable words | Compact with minor excess | Shortest natural wording that preserves precision |
| Common meaning prioritisation (15%) | No relevant sense | Rare or unrelated sense leads | Less common sense leads despite clear alternative | Plausible order, common use delayed | Common use near the front | Contextually common meaning leads and other senses follow coherently |
| Vocabulary coverage (10%) | No entry or target sense | Wrong entry or use only | Headword present, intended use unclear | One of two intended contexts covered | Intended use covered with a small gap | All intended contexts and part-of-speech uses available |
| Usage example quality (5%) | No example when needed, or misleading | Example contradicts meaning | Generic, awkward, or weakly informative | Natural and useful enough | Natural and clarifies a sense | Natural, distinctive, and clearly disambiguates the sense |
| Editorial style consistency (5%) | No comparable entry or chaotic | Strongly inconsistent presentation | Several style shifts | Mostly consistent | Minor style variation | Consistent voice, form, and sense labels |

Example-quality zero can mean absent; report example availability separately so absence is not hidden. For single-context terms, coverage score 5 means the intended use is present; for two-context terms, 3 means one is present and 5 means both are present. Do not use lexical overlap with a reference gloss as a substitute for semantic judgment.

## Statistics and limitations

`scripts/aggregate.py` reports retrieval and exact-headword rates with 95% Wilson intervals, length quantiles, and available score counts. For pairs with ratings on the same items, it uses seed `20261125` and 2,000 stratified bootstrap draws to report a 95% interval for mean score differences. Intervals summarize variability within this purposive set, not uncertainty for English vocabulary as a whole. Report all denominators and errors. Do not report an overall winner when source acquisition or ratings are incomplete.

Run `python scripts/build_corpus.py`, then the source extractors/harness, then `python scripts/evaluate.py --raw raw_results.jsonl`. Once reviewer ratings exist, rerun with `--ratings <completed-ratings.csv>`, then run `python scripts/aggregate.py`. Review raw records for every headline metric and preserve a sampled audit trail of sense matches and independent references.
