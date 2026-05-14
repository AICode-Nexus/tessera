# tessera-storage

JSONL trace writer and SQLite index for Tessera.

JSONL is the durable event truth. SQLite is a rebuildable query index over trace events.

This crate must not call providers, render UI, make model requests, evaluate policy, or persist secret values.
