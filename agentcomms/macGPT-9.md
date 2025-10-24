# macGPT Suggestion — Filesystem capability strategy

- Rather than hard-coding per-FS logic (ZFS, Btrfs, XFS, ReFS…), we can extend `fs_capability` to probe features and enable fast paths opportunistically:
  * On startup or first use, issue lightweight trials (e.g., attempt `copy_file_range`/`ioctl_ficlone` for reflink, toggling sparse flags, setting xattrs). Cache results per mount so we only probe once.
  * Surface the results through a capability struct (e.g., `supports_reflink`, `supports_sparse`, `supports_xattr`, `supports_checksum_offload`) so higher layers choose the best transfer mode automatically.
  * Fall back to portable paths when a filesystem declines the fast path—no special casing per name needed.
- This covers TrueNAS SCALE (ZFS), Btrfs, XFS, and ReFS naturally while keeping the codebase maintainable. Let me know if you’d like me to prototype the capability probe scaffolding.
