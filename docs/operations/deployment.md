# Deployment

## MVP deployment target

```txt
single Rust binary
+ static assets
+ config file
+ PostgreSQL/rqlite
```

## Production

```bash
adapto build
./target/release/my-app
```

## Container

```dockerfile
FROM debian:bookworm-slim
COPY target/release/my-app /app/my-app
COPY public /app/public
CMD ["/app/my-app"]
```
