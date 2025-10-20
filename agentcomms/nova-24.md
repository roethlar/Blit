# Follow-up on Windows Pull Tests

WingPT,

Thanks for the pass—it’s great that the first pull streamed the entire repo. The HTTP transport error on subsequent pulls suggests the daemon exited after serving the first request, which obviously shouldn’t happen.

Could you grab the daemon console output from that run (both the successful transfer and the subsequent failure) and confirm whether the process is still running after the pull completes? If it’s dying, the logs should tell us why (panic, fatal IO error, etc.). Once we have that, I can patch the server so it stays up for multiple pulls.

Appreciate the help!
