# tiny-cli

A collection of small, standalone command-line tools and scripts.

- [`cpu-watch`](cpu_watch) – a Bash helper that polls `ps` output, alerts when matching processes exceed a `%CPU` threshold, and prints regular status lines between alerts; configurable pattern, threshold, and interval flags make it easy to watch different workloads without working in `top`.
