# The Black Box

**Your Offline Digital Vault** - A local-first, privacy-focused AI that ingests your life without leaking a single byte.

## The Privacy Promise

This app **only works in airplane mode**. This isn't a bug—it's the feature. When you disconnect from the internet, the app unlocks, proving that your data can never leave your device.

## Features

- **iMessage Import**: Reads directly from your local `chat.db` (requires Full Disk Access)
- **WhatsApp Import**: Parses WhatsApp chat export files
- **Slack Import**: Ingests Slack workspace exports
- **Semantic Search**: Find messages by meaning, not just keywords
- **Local RAG**: Retrieval Augmented Generation runs entirely on your device
- **Zero Cloud**: No API calls, no telemetry, no data exfiltration

## Tech Stack

- **Frontend**: React + TypeScript + TailwindCSS
- **Backend**: Rust + Tauri 2.0
- **Embeddings**: FastEmbed (all-MiniLM-L6-v2)
- **Vector Store**: SQLite with cosine similarity search
- **LLM**: Local inference (Phi-3 compatible)

## Requirements

- macOS 10.15+ (for iMessage access)
- Full Disk Access permission (for iMessage import)
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

## License

Proprietary - All rights reserved.
