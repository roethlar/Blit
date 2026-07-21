# ldt-4 rig-W adaptive evidence

Status: **REVIEW_REQUIRED**

Validated 96 arms in 6 cells and 8 paired repetitions.
No worker target was selected or graded; observed floor, peak, final, operations, reasons, and raw samples are retained.
All interval fields are endpoint-local. No timestamp from q was subtracted from a netwatch-01 timestamp.

| Cell | source-init median ms | destination-init median ms | ratio | performance | decision pairs |
|---|---:|---:|---:|---|---:|
| q_to_windows_large | 878 | 1051 | 1.19703872437357630979498861 | REVIEW_REQUIRED | 0 |
| windows_to_q_large | 895 | 879 | 1.018202502844141069397042093 | WITHIN_1.10 | 0 |
| windows_to_q_small | 648.5 | 641.5 | 1.010911925175370226032735776 | WITHIN_1.10 | 8 |
| q_to_windows_small | 945.5 | 1015 | 1.073506081438392384981491274 | WITHIN_1.10 | 6 |
| q_to_windows_mixed | 715 | 809 | 1.131468531468531468531468531 | REVIEW_REQUIRED | 0 |
| windows_to_q_mixed | 578.5 | 559 | 1.034883720930232558139534884 | WITHIN_1.10 | 0 |

Any peak/final/operation/reason-distribution/reason-sequence difference is exported as REVIEW_REQUIRED; no undocumented decision threshold is applied.
A structurally valid zero-sample arm whose membership remained open through the first tuner tick is explicitly REVIEW_REQUIRED.
A median initiator-layout performance ratio above 1.10 is REVIEW_REQUIRED under the durable parent invariance bound.
