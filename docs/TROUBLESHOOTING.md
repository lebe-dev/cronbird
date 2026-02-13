# Troubleshooting

**Problem**: `403 Forbidden` when sending callback

**Solution**: Either add your identity to `CRONBIRD_IDENTITIES` or enable `CRONBIRD_ALLOW_DYNAMIC=true`

---

**Problem**: `401 Unauthorized` when sending callback

**Solution**: Include the Bearer token in the Authorization header

---

**Problem**: Metrics not showing in Prometheus

**Solution**: Check Prometheus scrape config and verify cronbird is reachable

---

**Problem**: State not persisting across restarts

**Solution**: Check `CRONBIRD_PERSIST_PATH` permissions and verify the directory is writable

---

For detailed documentation, see:
- [README.md](README.md) - Full documentation
- [examples/USAGE.md](examples/USAGE.md) - Usage examples
- [ARCH-001.md](ARCH-001.md) - Architecture details

**You're ready to monitor your cron jobs! 🐦**
