# Moving files: `git mv` vs `mv`

To avoid losing history and preserve traceability in git.

Files already tracked by git are always moved with `git mv`.

It is forbidden to delete files (for example, `rm`, `Remove-Item`, etc.) and recreate them in the new destination.

Files that are not yet tracked are moved with the normal command (for example, `mv`, `Move-Item`, etc.).

This applies to everything that lives in any repository or new files that have not yet been added to git.
