test *filter:
    cargo test {{ if filter == "" { "" } else { filter } }} -- --test-threads=1
