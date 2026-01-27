# Proper Code Cleanup Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix ~221 clippy warnings properly without using broad crate-level allows

**Architecture:** Use targeted fixes and allows only where semantically appropriate. Type casts in graphics code are intentional and get targeted allows. Auto-fixable lints get fixed. Documentation lints handled pragmatically.

**Tech Stack:** Rust, cargo clippy, cargo fmt

---

## Warning Summary (as of 2026-01-27)

| Category | Count | Approach |
|----------|-------|----------|
| Type casts (intentional graphics) | ~80 | Targeted `#[allow]` per expression |
| `Default::default()` style | ~43 | Auto-fix with clippy |
| Missing documentation (#Panics, #Errors) | ~33 | Crate-level allow (internal code) |
| Function too_many_lines | ~17 | Targeted allow (legitimate complexity) |
| Structural issues | ~15 | Fix where simple, allow where complex |
| Other style | ~33 | Fix or targeted allow |

---

### Task 1: Auto-fix Default::default() style warnings

**Files:** Multiple files in polyscope-render

**Step 1: Run clippy auto-fix for default_trait_access**

Run:
```bash
cargo clippy --fix --allow-dirty -- -W clippy::default_trait_access
```

**Step 2: Verify fixes applied**

Run:
```bash
cargo clippy 2>&1 | grep "default()" | wc -l
```
Expected: Significantly reduced count

**Step 3: Run formatter**

Run:
```bash
cargo fmt
```

**Step 4: Commit**

```bash
git add -A && git commit -m "style: use explicit type names instead of Default::default()"
```

---

### Task 2: Fix while_let_loop warning

**Files:**
- Modify: `crates/polyscope-core/src/state.rs:159`

**Step 1: Read the current code**

Read the loop at line 159 to understand the pattern.

**Step 2: Refactor to while let**

The current pattern:
```rust
loop {
    if let Some(group) = self.groups.get(current) {
        // ...
    } else {
        break;
    }
}
```

Should become:
```rust
while let Some(group) = self.groups.get(current) {
    // ...
}
```

**Step 3: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 4: Commit**

```bash
git add crates/polyscope-core/src/state.rs && git commit -m "fix: use while let loop instead of loop with if let"
```

---

### Task 3: Fix approx_constant warnings

**Files:**
- Modify: `crates/polyscope-render/src/color_maps.rs:75,92`

**Step 1: Add targeted allows for color map constants**

These are NOT the mathematical constant - they are color values that happen to be close to 1/PI. Add allow above the specific lines.

```rust
#[allow(clippy::approx_constant)] // This is a color value, not PI
```

**Step 2: Commit**

```bash
git add crates/polyscope-render/src/color_maps.rs && git commit -m "fix: allow approx_constant for color values (not mathematical PI)"
```

---

### Task 4: Add targeted type cast allows

**Files:** Multiple files with intentional graphics type casts

**Step 1: Identify all type cast locations**

Run:
```bash
cargo clippy 2>&1 | grep -E "cast_possible_truncation|cast_sign_loss|cast_precision_loss" | grep -oP "crates/[^:]+:\d+" | sort -u
```

**Step 2: Add targeted allows at each location**

For each location, add the minimal allow needed. For example:
```rust
#[allow(clippy::cast_possible_truncation)] // Width/height always fits in u32
let width = size as u32;
```

Or for expressions:
```rust
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
let idx = (t * n as f32).floor() as usize;
```

**Step 3: Verify casts are covered**

Run: `cargo clippy 2>&1 | grep "cast_" | wc -l`
Expected: 0

**Step 4: Commit**

```bash
git add -A && git commit -m "fix: add targeted allows for intentional type casts in graphics code"
```

---

### Task 5: Handle documentation lints pragmatically

**Files:**
- Modify: `crates/polyscope-core/src/lib.rs`
- Modify: `crates/polyscope-render/src/lib.rs`
- Modify: `crates/polyscope-structures/src/lib.rs`
- Modify: `crates/polyscope-ui/src/lib.rs`
- Modify: `crates/polyscope/src/lib.rs`

**Step 1: Add crate-level allows for documentation lints**

These are internal library crates. Missing #Panics and #Errors documentation is acceptable for now. Add at each crate's lib.rs:

```rust
// Documentation lints - internal crate, will add docs later
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
```

**Step 2: Commit**

```bash
git add -A && git commit -m "fix: allow missing docs lints (internal crates, docs to be added later)"
```

---

### Task 6: Handle too_many_lines warnings

**Files:** Files with large functions

**Step 1: Add targeted allows for legitimately complex functions**

For functions like `new_windowed`, `new_headless`, `handle_event` which are necessarily long, add:

```rust
#[allow(clippy::too_many_lines)] // Initialization requires many steps
pub async fn new_windowed(...) { ... }
```

**Step 2: Commit**

```bash
git add -A && git commit -m "fix: allow too_many_lines for legitimately complex initialization functions"
```

---

### Task 7: Fix structural issues

**Files:** Various

**Step 1: Fix unused_self warnings**

For methods with unused `self`, either:
- Make them associated functions (remove self)
- Add `#[allow(clippy::unused_self)]` if self is needed for API consistency

**Step 2: Fix needless_pass_by_value**

Change function signatures to take references where appropriate.

**Step 3: Fix items_after_statements**

Move enum/struct definitions to module level or add targeted allow.

**Step 4: Fix struct_excessive_bools and struct_field_names**

Add targeted allows - these are design choices.

**Step 5: Commit each category**

```bash
git add -A && git commit -m "fix: address structural clippy warnings"
```

---

### Task 8: Handle remaining warnings

**Files:** Various

**Step 1: Fix similar_names warnings**

Review and either rename variables or add allow.

**Step 2: Fix single-character binding names**

Add allow if names are clear in context (e.g., `x`, `y`, `z` for coordinates).

**Step 3: Fix let_else and lifetime warnings**

Apply suggested fixes.

**Step 4: Commit**

```bash
git add -A && git commit -m "fix: address remaining clippy warnings"
```

---

### Task 9: Final verification

**Step 1: Run full clippy check**

Run:
```bash
cargo clippy 2>&1
```
Expected: No warnings (or minimal acceptable warnings)

**Step 2: Run tests**

Run:
```bash
cargo test
```
Expected: All tests pass

**Step 3: Run example to verify no runtime warnings**

Run:
```bash
cargo run --example demo
```
Expected: No warnings printed

---

## Notes

- Type casts in graphics code are intentional - we're converting between GPU types and CPU types
- Documentation lints are allowed at crate level because this is an internal library
- Function length limits are subjective - initialization functions are necessarily long
- The goal is clean code, not zero warnings at any cost
