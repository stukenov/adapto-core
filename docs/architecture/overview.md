# Архитектура

## High-level архитектура

```txt
app/*.adapto
   |
adapto_parser
   |
adapto_compiler
   |
Route Manifest + Component IR + Rust codegen
   |
adapto_ssr
   |
Initial HTML over HTTP
   |
Browser + tiny client runtime
   |
WebSocket events
   |
adapto_live session actor
   |
State update + dirty tracking
   |
Patch protocol
   |
DOM update in browser
```
