# cpu-watch

Small Bash script that polls process CPU usage and prints an alert when matched processes cross a threshold.

## Motivation

Watching `top` can be hard to follow; snapshots may be easier to inspect _and_ easier to analyse afterwards.

## Usage

```bash
./cpu_watch [options]
```

Stop with `Ctrl+C`.

### Typical workflow

Send the output to a file. Often there are unknown processes.
Send the whole file to an LLM agent and ask it to analyse and summarise.

## Options

- `-p, --pattern REGEX` regular expression matched against command/path fields from `ps` output (default: `.*`)
- `-t, --threshold PERCENT` alert threshold for `%CPU` (default: `40`)
- `-i, --interval SECONDS` poll interval in seconds (default: `10`)
- `--program-names` enable process name replacement from sidecar mapping (default: enabled)
- `--no-program-names` disable sidecar process name replacement
- `-h, --help` show help

## Examples

```bash
# print help
./cpu_watch -h

# run with defaults
./cpu_watch

# example with all parameters (patterns, threshold, interval in seconds)
./cpu_watch -p 'Firefox|Safari' -t 60 -i 5

# look for macOS file indexing activity
./cpu_watch --pattern 'mds|mdworker|mds_stores'

# show all activity over 20% every minute
./cpu_watch -t 20 -i 60
```

## Output

- Prints `ALERT` with matching rows when any process is at or above the threshold.
- Alert rows include: `pid`, `cpu`, `comm`.
- When `--program-names` is enabled and a mapping matches, `comm` is replaced with
  `program*` (asterisk suffix means mapped).
- If sidecar mapping is disabled (`--no-program-names`), missing, or unmapped, the
  original process name/path is shown.
- Otherwise prints an `ok` status line for each polling interval.

## Optional Sidecar Mapping

You can add a file named `cpu_watch.names.tsv` in the working directory to map
process names (`comm`) to friendlier program names.

Format per line:

```tsv
process_name<TAB>program_name
```

Example:

```tsv
# comment lines are ignored
mdworker	Spotlight Indexer
mds	Spotlight Metadata Server
Google Chrome Helper	Google Chrome
```
