# Development

## Architecture

Project follows [hexagonal architecture](https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust) (ports & adapters):

```text
                          ┌──────────────────────────┐
                          │     Cronjobs/Scripts     │
                          │ (POST /callback/{id})    │
                          └────────────┬─────────────┘
                                       │
                                       │ HTTP Ping (Callback)
                                       v
                          ┌─────────────────────────┐
                          │         cronbird        │
                          │                         │
                          │   ┌─────────────────┐   │
                          │   │ In-memory State │   │
                          │   │   (RwLock)      │   │
                          │   └────────┬────────┘   │
                          │            │            │
         ┌────────────────┴────────────┼────────────┴────────────────┐
         │                             │                             │
         v                             v                             v
┌─────────────────┐           ┌─────────────────┐           ┌───────────────────┐
│ State Snapshots │           │    /metrics     │           │ /metrics/{id}     │
│ (JSON file)     │           │  (Prometheus)   │           │   (JSON API)      │
└─────────────────┘           └────────┬────────┘           └───────────────────┘
                                       ^
                                       │ Scrape (Pull)
                                       │
                          ┌────────────┴────────────┐
                          │ Prometheus / Victoria   │
                          │         Metrics         │
                          └─────────────────────────┘
```

File structure:

```
src/
├── domain/          # Core business logic
│   ├── model.rs     # Identity, CallbackRecord
│   └── ports.rs     # CallbackStore trait
├── adapters/        # Implementations
│   ├── memory_store.rs   # In-memory HashMap store
│   └── file_persist.rs   # JSON persistence
├── http/            # HTTP layer
│   ├── handlers.rs       # Request handlers
│   ├── metrics_format.rs # Prometheus/JSON formatters
│   └── auth.rs          # Bearer token middleware
├── config.rs        # Environment configuration
└── main.rs         # Bootstrap & graceful shutdown
```

**Design Decisions:**
- **In-memory storage** - Fast, simple, suitable for cron metrics
- **Periodic persistence** - Balance between durability and performance
- **Single instance** - State is local; for HA, use Redis adapter (not in MVP)
- **Lightweight** - No heavy frameworks or external dependencies

## API

Check [API.md](docs/API.md).
