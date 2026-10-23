# Bounded range scans

The skip list seeks to the lower bound using its upper links. SSTables binary-search the sparse index, read the possible predecessor block and stop before blocks starting at the exclusive upper bound. A heap merges bounded streams, chooses the newest sequence and then removes tombstones. The k: namespace ends at k;. The disk format is unchanged.

Run `cargo test --locked --test range_scan`. The test compares many bounds against a reference map and asserts fewer query block reads for a narrow interval. `Db::scan_with_block_reads` exposes counts; menu 10 prints them in its trace. Opening the database still verifies complete tables. Results are collected in memory.
