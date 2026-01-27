# Code Cleanup and Simplification Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Systematically clean up the codebase - fix all compiler warnings, address clippy lints, remove dead code, and improve code quality.

**Architecture:** Work through warnings by category, starting with the most impactful (dead code, unused), then moving to style and documentation issues. Each phase can be committed independently.

**Tech Stack:** Rust, cargo clippy, cargo fmt

---

## Current State Summary

- **Total Lines**: ~26,000 lines of Rust code
- **Compiler Warnings**: 5 (dead code, unused fields)
- **Clippy Warnings**: ~800+ total
  - 368 `#[must_use]` suggestions
  - 65 documentation backtick issues
  - 42 redundant closures
  - 35 f32/f64 casts
  - 35 `Default::default()` vs explicit default
  - 25 missing `# Panics` doc sections
  - 25 usize→u32 truncation warnings
  - 22 u32→f32 precision loss
  - 18 format string improvements
  - Various other minor issues

---

## Phase 1: Dead Code and Unused Items (HIGH PRIORITY)

These cause actual compiler warnings and indicate real cleanup opportunities.

### Task 1.1: Fix SliceMeshRenderData Unused Fields

**Files:**
- Modify: `crates/polyscope-render/src/slice_mesh_render.rs`

**Issue:** Fields `index_buffer`, `barycentric_buffer`, and `edge_is_real_buffer` are never read.

**Step 1: Check if these fields are needed for future functionality**

Read the file and understand if these buffers should be used but aren't, or if they're truly dead code.

**Step 2: Either use the fields or remove them**

If truly unused, remove the fields and their initialization code.

**Step 3: Run build**

Run: `cargo build -p polyscope-render`
Expected: Warning gone

**Step 4: Commit**

```bash
git commit -m "fix(render): remove unused fields from SliceMeshRenderData"
```

---

### Task 1.2: Fix VolumeMeshVectorQuantity Unused Style Field

**Files:**
- Modify: `crates/polyscope-structures/src/volume_mesh/vector_quantity.rs`

**Issue:** Field `style` is never read in both `VolumeMeshVertexVectorQuantity` and `VolumeMeshCellVectorQuantity`.

**Step 1: Check if style should be used**

If the vector style (arrows, lines, etc.) isn't being applied, either implement it or remove the field.

**Step 2: Fix by removing or implementing**

**Step 3: Run build**

Run: `cargo build -p polyscope-structures`
Expected: Warnings gone

**Step 4: Commit**

```bash
git commit -m "fix(structures): remove unused style field from volume mesh vector quantities"
```

---

### Task 1.3: Fix Public Fields Prefixed with Underscore

**Files:** Multiple files with `pub _padding` fields

**Issue:** 13 instances of `field marked as public but also inferred as unused because it's prefixed with _`

**Step 1: Find all instances**

```bash
cargo clippy 2>&1 | grep "prefixed with" -A 3
```

**Step 2: Change `pub _padding` to just `_padding` (private)**

Padding fields don't need to be public.

**Step 3: Run clippy to verify**

**Step 4: Commit**

```bash
git commit -m "fix: make padding fields private"
```

---

## Phase 2: Clippy Quick Wins (MEDIUM PRIORITY)

These are easy fixes that improve code quality.

### Task 2.1: Fix Redundant Closures (42 instances)

**Issue:** Using `.map(|x| foo(x))` instead of `.map(foo)`

**Step 1: Run clippy with fix flag**

```bash
cargo clippy --fix --allow-dirty -- -W clippy::redundant_closure
```

**Step 2: Review changes**

**Step 3: Run tests**

Run: `cargo test`
Expected: PASS

**Step 4: Commit**

```bash
git commit -m "fix: replace redundant closures with direct function references"
```

---

### Task 2.2: Fix Format String Variables (18 instances)

**Issue:** Using `format!("{}", var)` instead of `format!("{var}")`

**Step 1: Run clippy fix**

```bash
cargo clippy --fix --allow-dirty -- -W clippy::uninlined_format_args
```

**Step 2: Review changes**

**Step 3: Run tests**

**Step 4: Commit**

```bash
git commit -m "style: use inline format args"
```

---

### Task 2.3: Fix map_or Simplifications (8 instances)

**Issue:** Using `.map(f).unwrap_or(default)` instead of `.map_or(default, f)`

**Step 1: Find instances**

```bash
cargo clippy 2>&1 | grep "map_or can be simplified" -B 5
```

**Step 2: Fix manually or with clippy --fix**

**Step 3: Run tests**

**Step 4: Commit**

```bash
git commit -m "fix: simplify map_or patterns"
```

---

### Task 2.4: Fix Unneeded Return Statements (6 instances)

**Step 1: Run clippy fix**

```bash
cargo clippy --fix --allow-dirty -- -W clippy::needless_return
```

**Step 2: Review and test**

**Step 3: Commit**

```bash
git commit -m "style: remove unnecessary return statements"
```

---

### Task 2.5: Use copied() Instead of cloned() (6 instances)

**Issue:** Using `.cloned()` on iterators of `Copy` types

**Step 1: Find and fix**

```bash
cargo clippy --fix --allow-dirty -- -W clippy::cloned_instead_of_copied
```

**Step 2: Commit**

```bash
git commit -m "fix: use copied() instead of cloned() for Copy types"
```

---

### Task 2.6: Fix Identical Match Arms (6 instances)

**Step 1: Find instances**

```bash
cargo clippy 2>&1 | grep "identical bodies" -B 10
```

**Step 2: Combine match arms using `|` pattern**

**Step 3: Commit**

```bash
git commit -m "fix: combine identical match arms"
```

---

## Phase 3: Type Cast Improvements (MEDIUM PRIORITY)

### Task 3.1: Use From for Infallible Casts (35 + 19 = 54 instances)

**Issue:** Using `x as f64` instead of `f64::from(x)` for infallible conversions

**Step 1: Fix f32→f64 casts**

Replace `x as f64` with `f64::from(x)` where x is f32.

**Step 2: Fix u32→u64 casts**

Replace `x as u64` with `u64::from(x)` where x is u32.

**Step 3: Review and commit**

```bash
git commit -m "fix: use From trait for infallible type conversions"
```

---

### Task 3.2: Address Precision Loss Casts (22 instances)

**Issue:** `u32 as f32` loses precision for large values

**Step 1: Review each instance**

Determine if precision loss is acceptable (often is for small values).

**Step 2: Either allow with annotation or fix**

Add `#[allow(clippy::cast_precision_loss)]` where acceptable, or use alternative approach.

**Step 3: Commit**

```bash
git commit -m "fix: address u32 to f32 precision loss casts"
```

---

### Task 3.3: Address Truncation Warnings (25 instances)

**Issue:** `usize as u32` may truncate on 64-bit platforms

**Step 1: Review each instance**

Most are for indices that won't exceed u32::MAX.

**Step 2: Use try_into or allow**

Add `#[allow(clippy::cast_possible_truncation)]` where safe, or use `.try_into().unwrap()`.

**Step 3: Commit**

```bash
git commit -m "fix: address usize to u32 truncation warnings"
```

---

## Phase 4: Pipeline Default Cleanup (LOW PRIORITY)

### Task 4.1: Use Default::default() Explicitly (35 instances)

**Issue:** Using `wgpu::PipelineCompilationOptions { .. }` instead of `Default::default()`

**Step 1: Replace in all pipeline creation code**

```rust
// Before
compilation_options: wgpu::PipelineCompilationOptions::default(),

// Already correct, but some may be:
compilation_options: Default::default(),
```

**Step 2: Commit**

```bash
git commit -m "style: use explicit Default::default() for pipeline options"
```

---

## Phase 5: Documentation Improvements (LOW PRIORITY)

### Task 5.1: Fix Missing Backticks in Docs (65 instances)

**Issue:** Code references in docs should use backticks

**Step 1: Find instances**

```bash
cargo clippy 2>&1 | grep "missing backticks" -B 3
```

**Step 2: Add backticks around code references**

Example: `u32::MAX` instead of u32::MAX in doc comments

**Step 3: Commit**

```bash
git commit -m "docs: add backticks around code references"
```

---

### Task 5.2: Add # Panics Sections (25 instances)

**Issue:** Functions that can panic should document when

**Step 1: Find instances**

**Step 2: Add `# Panics` doc sections or add `#[allow]` if panics are internal**

**Step 3: Commit**

```bash
git commit -m "docs: add # Panics sections to panicking functions"
```

---

### Task 5.3: Add # Errors Sections (8 instances)

**Issue:** Functions returning Result should document errors

**Step 1: Find and add `# Errors` sections**

**Step 2: Commit**

```bash
git commit -m "docs: add # Errors sections to Result-returning functions"
```

---

## Phase 6: #[must_use] Attributes (OPTIONAL)

### Task 6.1: Add #[must_use] to Important Methods

**Issue:** 368 + 39 methods/functions could have `#[must_use]`

This is a large undertaking. Consider:
- Adding to builder pattern methods
- Adding to methods returning newly created values
- Skip for setter methods that return `&mut Self`

**Step 1: Add to most important public API methods**

Focus on:
- `new()` constructors
- Methods returning Options that indicate success/failure
- Builder pattern methods

**Step 2: For remaining, consider adding crate-level allow**

```rust
#![allow(clippy::must_use_candidate)]
```

**Step 3: Commit**

```bash
git commit -m "feat: add #[must_use] to important API methods"
```

---

## Phase 7: Struct Field Cleanup (OPTIONAL)

### Task 7.1: Review Struct with Many Bools

**Issue:** `more than 3 bools in a struct`

**Step 1: Find the struct**

```bash
cargo clippy 2>&1 | grep "more than 3 bools" -B 5
```

**Step 2: Consider using a bitflags crate or enum**

Or allow if the bools are semantically distinct.

**Step 3: Commit**

```bash
git commit -m "refactor: consolidate boolean flags where appropriate"
```

---

## Execution Order

**Recommended order (by impact and safety):**

1. **Phase 1** (Dead code) - Removes actual warnings, safe
2. **Phase 2** (Quick wins) - Easy clippy fixes, auto-fixable
3. **Phase 3** (Type casts) - Requires review but improves safety
4. **Phase 4** (Pipeline defaults) - Cosmetic, low risk
5. **Phase 5** (Documentation) - Improves maintainability
6. **Phase 6** (#[must_use]) - Large, consider crate-level allow
7. **Phase 7** (Struct cleanup) - Optional refactoring

**After all phases:**

Run full validation:
```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

---

## Summary

This plan addresses ~800+ clippy warnings and 5 compiler warnings through systematic cleanup:

| Phase | Category | Count | Priority |
|-------|----------|-------|----------|
| 1 | Dead code/unused | ~18 | HIGH |
| 2 | Quick clippy fixes | ~80 | MEDIUM |
| 3 | Type casts | ~80 | MEDIUM |
| 4 | Pipeline defaults | ~35 | LOW |
| 5 | Documentation | ~100 | LOW |
| 6 | #[must_use] | ~400 | OPTIONAL |
| 7 | Struct cleanup | ~5 | OPTIONAL |

Phases 1-5 should bring warnings to near-zero. Phases 6-7 are optional polish.
