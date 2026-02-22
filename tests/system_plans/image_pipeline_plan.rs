//! Scenario Plan: Image / Binary Asset Pipeline with Artifact<Binary>
//!
//! Blueprint anchors:
//! - Artifact<Binary> for non-text assets – 01-intro.md §3.1.
//! - No Direct IO + StructuralDelta model – 01-intro.md §3.1; 04-agent-model.md §5.1.
//! - Security pipeline and policy stages – 03-security.md (overall).
//!
//! High-level story:
//! - A team wants to optimize and standardize image assets for a web app:
//!   resizing, converting formats, and updating references in code/config.
//! - Agents must treat images as binary artifacts, not raw files, and all
//!   transformations must go through StructuralDelta<Binary> and associated
//!   metadata updates.
//!
//! Representative user input:
//! - Intent:
//!   "Optimize all hero images under `assets/hero/*.png` for web, convert
//!    them to WebP, update references in code and config, and generate a
//!    report of size reductions."
//! - Context:
//!   - Binary artifacts representing images.
//!   - Code and config artifacts referencing those images.
//!   - Mode: Factory or Sketchpad, depending on strictness.
//!   - Security pipeline configured to scan binaries.
//!
//! Expected system behavior:
//! 1. Intent parsing:
//!    - Goal: Optimize assets (non-code).
//!    - Targets: Binary artifacts + their references.
//!    - Acceptance: all hero images optimized and references updated without
//!      broken links.
//! 2. Decomposition:
//!    - Task A: Discover all hero image artifacts and their references in
//!      code/config artifacts.
//!    - Task B: For each image, compute an optimized variant and produce a
//!      StructuralDelta<Binary> describing transformation and metadata
//!      changes (e.g., resolution, format).
//!    - Task C: Generate StructuralDelta<Code/Config> to update references
//!      from old images to new ones.
//!    - Task D: Run image-specific checks (dimensions, size constraints).
//!    - Task E: Generate a report artifact summarizing changes and savings.
//! 3. Graph construction:
//!    - Ensure no direct file writes; all updates happen through deltas and
//!      application layer.
//!    - Enforce single-writer per image and reference.
//!    - Validate that all SymbolRefs / references to images are updated and
//!      still resolve after transformations.
//! 4. Execution:
//!    - Apply binary and code/config deltas in sandbox.
//!    - Run link-check / consistency tests.
//!    - Run security scans on new binaries.
//! 5. Output:
//!    - New Binary artifacts for optimized images.
//!    - Updated Code/Config artifacts with correct references.
//!    - Report artifact capturing before/after sizes and integrity status.
//!
//! Test plan dimensions:
//! - Correctness:
//!   - For a synthetic mini-site, all image references remain valid and
//!     renderable after optimization.
//! - Safety:
//!   - No direct file IO; all changes go through Binary artifacts.
//!   - Security scanning tasks execute and must pass.
//! - Cross-artifact integrity:
//!   - References in code/config are updated consistently.
//! - Regression hooks:
//!   - Fixed set of synthetic images with known sizes and a golden report
//!     to detect regressions in binary artifact handling and reference
//!     updates.

