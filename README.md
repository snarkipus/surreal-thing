# Watchers
Build:

`TEST_LOG=1 cargo watch -q -c -w src/ -x run | bunyan`

Test: 

`TEST_LOG=1 cargo watch -q -c -w tests/ -x "test --package surreal-simple --test endpoints -- crud_query_endpoints_work --exact --nocapture"`