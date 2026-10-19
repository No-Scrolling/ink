---
title: "Minato–Morreale ISOP and Ink"
description: "Where Boolean cover generation could apply to Ink, and why it does not establish a rendering speedup."
---

Minato–Morreale ISOP has a possible future role in Ink's compiler, but the current code offers no compelling immediate application. It generates Boolean expressions; it does not directly accelerate layout, text, image decoding or Vulkan drawing. This assessment combines published algorithm results with a source review on 19 October 2026. No Ink implementation or benchmark of ISOP was performed.

## What the recurrence does

An irredundant sum of products (ISOP) represents a Boolean function as an OR of AND terms. Each term is a prime implicant: no literal can be removed while keeping that term inside the permitted function. No whole term can be removed while preserving required coverage. These are local guarantees, not a guarantee of the fewest terms, literals or executed operations. [Minato, 1993, sections 4.1–4.4](https://eprints.lib.hokudai.ac.jp/repo/huscap/all/47468/59_IEICE76_967.pdf).

The input bounds satisfy `L ⇒ U`. `L` marks combinations that must return true; `U` marks combinations allowed to return true. The result `G` satisfies `L ⇒ G ⇒ U`. Where `U` is true and `L` false, either result is acceptable: these are don't-care combinations. Set `L = U = f` when exact equivalence is required. Such freedom must come from a proved constraint, rather than states absent from observed runs. [CUDD implementation](https://github.com/cuddorg/cudd/blob/main/cudd/cuddZddIsop.c).

For a selected variable `x`, write `L0`, `L1`, `U0`, `U1` for the bounds with `x` fixed false or true. Using `I` for the recursive result, the recurrence is:

```text
A = I(L0 ∧ ¬U1, U0)
B = I(L1 ∧ ¬U0, U1)
C = I((L0 ∧ ¬A) ∨ (L1 ∧ ¬B), U0 ∧ U1)
G = (¬x ∧ A) ∨ (x ∧ B) ∨ C
```

The first two branches cover obligations needing a particular value of `x`; the third covers remaining obligations independently of `x`. The base cases return false for `L = 0` and true for `U = 1`. CUDD caches recursive results and can return both the function as a binary decision diagram (BDD) and the term set as a zero-suppressed decision diagram (ZDD). Variable order affects the result. [CUDD source](https://github.com/cuddorg/cudd/blob/main/cudd/cuddZddIsop.c).

Cost includes constructing and manipulating the diagrams and emitting the cover. Minato discusses variable ordering and exponential diagram/cover growth; his original implementation emits cubes directly rather than requiring a ZDD. Its reported speedups concern generating covers for logic circuits, not executing app predicates. [Minato, sections 2, 4.4 and 5](https://eprints.lib.hokudai.ac.jp/repo/huscap/all/47468/59_IEICE76_967.pdf). Sasao and Butler prove that Minato–Morreale can produce a worst ISOP whose term count is arbitrarily far from minimum. [Sasao and Butler, 2001](https://www.lsi-cad.com/sasao/Papers/files/IEEETC2001.pdf). The 1998 decomposition paper shows a useful factorisation property, not a general execution-time optimum. [Minato and De Micheli, 1998](https://si2.epfl.ch/demichel/publications/archive/1998/ICCADconf98pg111.pdf).

## Where it could apply to Ink

**Compiler predicates are the plausible candidate.** The native-list compiler accepts some conditional expressions in properties and preserves their JavaScript in projection fields. It requires fixed JSX structure; conditional child trees currently retain the React path. A future compiler could lower supported conditional trees into native templates, then use ISOP to simplify repeated Boolean guards. Template support and a suitable intermediate representation are prerequisites; minimisation alone cannot provide them. See [the native-list compiler](../../crates/ink-compiler/src/native-lists.js).

Any rewrite would need proved Boolean semantics, including short-circuit behaviour, nullable reads and exceptions. The existing runtime already rejects getters, proxies and unsupported prototypes for native-list data, but the compiler's syntactic `pure()` classification alone does not establish that arbitrary JavaScript predicates can be reordered or expanded. See [native-list data validation](../../packages/ink/src/native-list.ts).

**Rendering is a weak match.** Ink already caches scenes, geometry and text, and uploads only the changed instance range. Its masks contain antialiased alpha values; toggle masks are cached. Boolean cover minimisation does not preserve fractional coverage or ordered alpha compositing. Encoding pixel coordinates as bits also produces Boolean cubes, not general screen rectangles. See [renderer caches and uploads](../../crates/ink-renderer-vulkan/src/compact.rs), [masks](../../crates/ink-core/src/masks.rs) and [shaders](../../crates/ink-renderer-vulkan/src/shader.wgsl).

Dirty-region simplification would additionally require a design for retained pixels and damage propagation: the current Vulkan path clears its render attachment and uses the full render area. ISOP alone cannot provide partial repainting. See [Vulkan rendering](../../crates/ink-renderer-vulkan/src/gpu.rs).

## Performance decision

Treat a bounded, build-time predicate experiment as an option if profiling identifies repeated Boolean guards as material work. Cap variables, diagram nodes and output terms; retain the original expression unless generated code wins on the relevant workload. Compare simple constant folding and common-subexpression reuse first. These are engineering recommendations, not measured results.

ISOP does not remove the remaining O(n) list identity scan documented in [runtime ownership](../runtime-ownership.md). Ink's existing performance harness, described in `test/performance/README.md`, provides the starting point for evaluating any future change. Current evidence does not establish an Ink speedup from this recurrence.
