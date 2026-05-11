Development commands on Windows PowerShell:
- cargo run -p seating-cli -- validate --people people.csv --closeness closeness.csv --tables tables.csv
- cargo run -p seating-cli -- optimize --people people.csv --closeness closeness.csv --tables tables.csv --output seating.csv --seed 1234 --solutions 1
- cargo run -p seating-cli -- score --people people.csv --closeness closeness.csv --tables tables.csv --seating seating.csv
- cargo run -p seating-cli -- render --people people.csv --tables tables.csv --seating seating.csv --output seating-plan.svg
- cargo run -p seating-gui
- cargo fmt --all
- cargo clippy --workspace --all-targets -- -D warnings
- cargo test --workspace