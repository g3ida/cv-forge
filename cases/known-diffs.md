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
