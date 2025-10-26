## Manifest / Need-List Streaming Plan (2025-10-26)
Objective: eliminate in-memory manifest scaling limits during remote push negotiations so arbitrarily large payloads succeed without exhausting RAM.

### Current bottlenecks
- CLI enumerates entire manifest into `Vec<FileHeader>`; daemon loads all headers again to compute need list.
- Need list is returned as one `FilesToUpload` message (now batched, but still built in memory first).

### Proposed approach
1. **Streaming Manifest Upload**
   - CLI enumerator yields `FileHeader` entries incrementally (tokio stream) instead of building a Vec.
   - Send the stream over gRPC; daemon consumes headers and processes each entry on the fly.

2. **On-the-fly Need List Calculation**
   - Daemon checks each file as soon as header arrives; if it needs upload, emit it to a new async channel.
   - A background task batches eligible headers into `FilesToUpload` messages (same 3 MiB/2k entry limits) and sends them immediately.
   - Once manifest stream ends, send a final empty batch to signal completion.

3. **Client Accumulation & Back-pressure**
   - Update CLI push logic to read the stream of `FilesToUpload` batches and populate an async queue.
   - Transfer phase (control-plane fallback or data-plane) consumes headers from the queue. Pause reading the gRPC stream when the queue hits a high-water mark to avoid unbounded growth.

4. **Chunked File Data**
   - For control-plane fallback, send file data in sub-chunks (e.g., 1 MiB) to avoid building whole files in memory.

5. **Fault tolerance**
   - If the daemon can’t keep up, back-pressure the manifest stream by withholding acks until current batches flush.

6. **Testing**
   - Add stress tests (synthetic generator) with >1M entries to verify streaming behaviour, memory footprint, and throughput.

Next steps: refactor CLI enumerator to produce a stream, implement server-side incremental processing + batching, add back-pressure queue, chunk file data, and profile with stress tests.