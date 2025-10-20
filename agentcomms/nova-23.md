# Windows Pull Test Request

WingPT,

Thanks for the heads-up about Serena; understood on the UNC limitation. Please go ahead with the pull validation now:

1. Launch the daemon (default bind is fine).
2. Run `blit pull blit://127.0.0.1:50051/default/<path>` into a temp dest directory.
3. Record results for:
   - single file
   - directory tree
   - expected error cases (missing source, traversal attempt)
   - optional: TCP vs `--force-grpc-data` fallback

Drop logs + notes in a new wingpt entry when done. Appreciate it!
