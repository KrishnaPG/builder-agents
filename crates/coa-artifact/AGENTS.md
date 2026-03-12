# coa-artifact

**Typed, content-addressed artifacts** with structural delta support.

## STRUCTURE

```
src/
├── artifact.rs       # Artifact<T> container
├── delta.rs          # StructuralDelta transformations
├── hash.rs           # Blake3 content hashing
├── merkle.rs         # Merkle tree support
├── path.rs           # SymbolPath hierarchical addressing
└── types/            # Concrete artifact types
    ├── binary.rs
    ├── code.rs       # Parsed AST with symbols
    ├── config.rs     # Schema-validated config
    └── spec.rs       # Specification documents
```

## KEY PATTERNS

### Content Addressing
```rust
let artifact = Artifact::<CodeArtifact>::new(content)?;
println!("Hash: {}", artifact.hash());
assert!(artifact.verify());
```

### Structural Deltas
```rust
let delta = StructuralDelta::<CodeArtifact>::new(
    SymbolPath::single("function"),
    DeltaOperation::Replace(new_content),
    *artifact.hash(),
);
assert!(delta.validate_base(&artifact).is_ok());
```

### ArtifactType Trait
Implement for custom types:
```rust
impl ArtifactType for MyArtifact {
    type Content = MyContent;
    fn hash(content: &Self::Content) -> ContentHash;
    const TYPE_ID: &'static str;
}
```

## ANTI-PATTERNS

- **Hash collisions ignored**: Always verify before applying deltas
- **Direct mutation**: Create new artifacts, don't mutate
- **Path construction**: Use `SymbolPath::from_str()`, not string concat

## COMPLEXITY HOTSPOTS

- `types/code.rs` (613 lines): Complex AST handling
- `delta.rs` (546 lines): Delta operations
- `types/spec.rs` (562 lines): Spec validation
