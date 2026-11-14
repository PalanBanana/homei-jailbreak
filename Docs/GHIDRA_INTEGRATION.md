# Ghidra integration (read-only)

The lab reads an **existing** local Ghidra analysis of the exact build and summarises it. It does not run Ghidra, does not open a project database for writing, does not reimport a binary and does not modify firmware.

## What is read

| Input | Used for |
|---|---|
| The project directory | whether a `.gpr` project and its `.rep` repository exist (presence only) |
| `analysis/function-counts-by-fileset-entry.json` | the function count |
| `analysis/kernel-fileset-coverage.json` | fileset entries and how many are covered |
| `analysis/kernel-symbolication-metrics.json` | the kernelcache hash the analysis refers to, and the symbol match rate reported by `ipsw`, kept as integer basis points |
| Ghidra log files | counts of classified import and analysis issues, listed per log file |

Every field of the summary is either read from one of those files or reported as missing. `import_status` says whether the project was imported and analysed, and says so with classified log lines rather than hiding them. For the current export the status is "imported and analysed, with classified log lines". The log lines are real: the per-KEXT diagnostic imports logged exceptions, and they stay visible in the app.

## What is never written

No absolute path, no file name from the local machine and no device identifier appears in the summary. The summary type has no path field, so one cannot be serialised by accident. The privacy tests scan the exported state for path markers.

## What the numbers mean

The exported summary for `kernelcache.release.iPhone16,2` reports 230,298 functions, 264 of 264 fileset entries covered and a symbol match rate of 75.56 percent. These are **static observations** with medium confidence. A function or symbol name is a label, not a statement about a vulnerability. Findings about specific mechanisms live in the research tracker and keep their own evidence levels.

## Reuse

Existing databases are preserved. The lab never reimports a binary that is already in the project. If the analysis directory is absent, the summary says the project is not present instead of inventing numbers.
