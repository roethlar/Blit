# 10GbE session evidence (2026-07-04, MTU 9000 both ends)
iperf3 baseline: fwd 9.88 Gbit/s (1 stream) / 9.91 (4 streams); rev 9.91
TCP push 1GiB: 934ms wall, first payload 14.5ms
TCP pull 1GiB (clean): 887ms = 9.7 Gbit/s; client stream line: 9.78 Gbps
gRPC push 1GiB (clean): 1054ms = 8.2 Gbit/s; first payload 1.01s (design-4 full-manifest gate)
Reverse (skippy client): push 7.25 Gbps data-plane / first payload 3.9ms; pull 881ms = 9.75 Gbit/s
Concurrent 2x push: 8.30 + 3.21 Gbps, clean completion, no resize triggered (single stream saturates 10GbE)
Loopback pull byte-identical (cmp)
