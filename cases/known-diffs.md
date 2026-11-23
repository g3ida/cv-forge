# Cases changed by hand

Only these recorded cases were changed after recording, with the user's approval
(DECISIONS.md ADR-015, 2026-11-21). In each, a path is `''`, which pathlib reads as `.` (the
current folder): cv-builder crashed with a Python traceback and exit status 1; cv-forge prints
a one-line error with the same exit status. stdout and status are unchanged.

| Case | Arguments | cv-builder (last traceback line) | cv-forge |
| --- | --- | --- | --- |
| trap-014-validate | `validate ''` | `IsADirectoryError: [Errno 21] Is a directory: '.'` | `.: is a directory` |
| trap-024-build | `build ''` | `IsADirectoryError: [Errno 21] Is a directory: '.'` | `error: .: is a directory` |
| trap-034-build | `build '' -o out/cv.pdf --accent '#0B7A75' --page-size Letter` | `IsADirectoryError: [Errno 21] Is a directory: '.'` | `error: .: is a directory` |
| trap-074-build | `build '' --max-pages 2` | `IsADirectoryError: [Errno 21] Is a directory: '.'` | `error: .: is a directory` |
| trap-092-build | `build '' --html debug.html` | `IsADirectoryError: [Errno 21] Is a directory: '.'` | `error: .: is a directory` |
| trap-112-build | `build '' -t clear` | `IsADirectoryError: [Errno 21] Is a directory: '.'` | `error: .: is a directory` |
| trap-132-schema | `schema -o ''` | `IsADirectoryError: [Errno 21] Is a directory: '.'` | `error: .: is a directory` |
| trap-142-build | `build '' -t classic -o out.pdf` | `IsADirectoryError: [Errno 21] Is a directory: '.'` | `error: .: is a directory` |
| trap-162-build | `build /home/gaida/Documents/projects/personal_2/cv-forge/examples/android-hmi-architect.json -t classic -o ''` | `IsADirectoryError: [Errno 21] Is a directory: '.'` | `error: .: is a directory` |
| trap-172-build | `build '' -t clear -o out.pdf` | `IsADirectoryError: [Errno 21] Is a directory: '.'` | `error: .: is a directory` |
| trap-182-build | `build /home/gaida/Documents/projects/personal_2/cv-forge/examples/android-hmi-architect.json -t clear -o ''` | `IsADirectoryError: [Errno 21] Is a directory: '.'` | `error: .: is a directory` |
| trap-192-build | `build '' -t compact -o out.pdf` | `IsADirectoryError: [Errno 21] Is a directory: '.'` | `error: .: is a directory` |
| trap-202-build | `build /home/gaida/Documents/projects/personal_2/cv-forge/examples/android-hmi-architect.json -t compact -o ''` | `IsADirectoryError: [Errno 21] Is a directory: '.'` | `error: .: is a directory` |
| trap-212-build | `build '' -t modern -o out.pdf` | `IsADirectoryError: [Errno 21] Is a directory: '.'` | `error: .: is a directory` |
| trap-222-build | `build /home/gaida/Documents/projects/personal_2/cv-forge/examples/android-hmi-architect.json -t modern -o ''` | `IsADirectoryError: [Errno 21] Is a directory: '.'` | `error: .: is a directory` |
| trap-232-build | `build '' -t sidebar -o out.pdf` | `IsADirectoryError: [Errno 21] Is a directory: '.'` | `error: .: is a directory` |
| trap-242-build | `build /home/gaida/Documents/projects/personal_2/cv-forge/examples/android-hmi-architect.json -t sidebar -o ''` | `IsADirectoryError: [Errno 21] Is a directory: '.'` | `error: .: is a directory` |
| trap-252-build | `build '' -t timeline -o out.pdf` | `IsADirectoryError: [Errno 21] Is a directory: '.'` | `error: .: is a directory` |

## Eleven templates (ADR-024, 2026-11-23)

cv-builder bundles eleven templates since career-ledger 20429fc, cv-forge too; the recorded
cases come from the snapshot with six. These cases differ only in that list: `example-008-templates`
(stdout: the five new templates listed) and, in stderr, `unknown template '...' (available: ...)`
naming eleven: `build-unknown-template`, `trap-149-build`, `trap-150-build`, `trap-151-build`,
`trap-153-build`, `trap-154-build`, `trap-155-build`, `trap-156-build`, `trap-157-build`,
`trap-158-build`.
