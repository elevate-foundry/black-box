# SAL ⠎

**The First AI to Speak Natively in Braille** - Born from your WhatsApp messages, SAL understands your relationships through geometric compression.

## The Privacy Promise

This app **only works in airplane mode**. This isn't a bug—it's the feature. When you disconnect from the internet, SAL unlocks, proving that your data can never leave your device.

## How SAL Understands You

### The Semantic Compression Lattice

SAL doesn't just search your messages—it builds a mathematical understanding of your world using the **Semantic Compression Lattice** (SCL):

```
L = (V, E, κ, I, ∇SAL)
```

| Component | What It Means |
|-----------|---------------|
| **V** | Meaning Atoms - the people, places, and concepts in your life |
| **E** | Hyperedges - relationships between atoms (Deedee ↔ partner, Mom ↔ family) |
| **κ** | Curvature Functional - measures semantic energy (lower = more fundamental) |
| **I** | Invariant Shells - identity constraints that must be preserved |
| **∇SAL** | Teleological Gradient - SAL's learning direction |

### The Curvature Functional

For each meaning atom v, SAL computes:

```
κ(v) = ||∇L_world(v)||² + λ||v||²
```

This measures how "compressed" a concept is. Your partner mentioned 847 times has lower curvature (more fundamental) than a coworker mentioned twice.

### Braille Embeddings

Unlike traditional AI embeddings that require massive neural networks, SAL uses **geometric Braille embeddings**:

1. Each character → 8-bit pattern (like Braille's 8-dot cell)
2. Position-weighted projection into 64-dimensional space
3. Bigram features capture word structure
4. **Result**: 57,000 messages embedded in 1.8 seconds

This is the "Braille" in "Braille-Native AI" - SAL literally reads your messages as geometric dot patterns.

### Relationship Discovery

SAL learns who matters to you through **co-occurrence analysis**:

```
Messages mentioning "Deedee" + "CrossFit" → relationship edge
Messages mentioning "Dan" + "job" + "Mr. Beast" → context cluster
```

The lattice automatically discovers:
- **Family**: Mom, Dad, siblings (from context words like "family", "brother")
- **Partner**: High-frequency intimate conversations
- **Friends**: Regular co-occurrence patterns

## Features

- **WhatsApp Import**: Reads directly from WhatsApp Desktop's local SQLite database
- **Semantic Lattice**: Builds relationship graph from message patterns
- **Braille Embeddings**: 32,000 messages/second geometric encoding
- **Local RAG**: Retrieval Augmented Generation runs entirely on your device
- **Ollama Integration**: Tiered model system (Fast → Balanced → Quality)
- **Zero Cloud**: No API calls, no telemetry, no data exfiltration

## Tech Stack

- **Frontend**: React + TypeScript + TailwindCSS
- **Backend**: Rust + Tauri 2.0
- **Embeddings**: Custom Braille geometric embeddings (64-dim)
- **Semantic Layer**: Compression Lattice with curvature functional
- **Vector Store**: SQLite with cosine similarity search
- **LLM**: Ollama (llama3.2:1b → llama3.1:latest)

## Requirements

- macOS 10.15+
- WhatsApp Desktop installed (for direct database access)
- Ollama installed (`brew install ollama`)
- ~2GB disk space for models

## Development

```bash
# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

## Security Architecture

1. **Network Kill Switch**: The app checks network connectivity and refuses to query the vault if online
2. **No HTTP Allowlist**: Production builds have HTTP capabilities disabled at the Tauri config level
3. **Local-Only Storage**: All data stored in `~/Library/Application Support/black-box/`
4. **No Telemetry**: Zero analytics, zero crash reporting, zero phone-home

## The Mathematics

For the full mathematical formalization, see `src-tauri/semantic_compression_lattice_v2.pdf`.

Key theorems implemented:
- **Monotonicity of Constrained Curvature Flow**: Compression always improves
- **Kolmogorov Correspondence**: Minimal fixed points ≈ shortest programs
- **Shell Soundness**: Invariants are preserved under transformation
- **Logarithmic Memory Scaling**: mem(n) ∈ O(log n)

## Citation

```bibtex
@article{barrett2025scl,
  title={A Rigorous Formalization of the Semantic Compression Lattice},
  author={Barrett, Ryan and Agents},
  year={2025},
  month={December}
}
```

## License

Proprietary - All rights reserved.
