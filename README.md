# Sales Chart Visualization

This Rust application processes sales data from a CSV file and generates visualizations of sales trends over time and by product. The visualizations are saved as PNG images.

## Features

- Validates CSV file structure.
- Processes sales data to calculate total sales by month and by product.
- Generates a line chart for monthly sales trends.
- Creates a bar chart for sales by product.
- Saves charts as `sales_chart.png`.

## Prerequisites

- Rust (installed via [rustup](https://www.rust-lang.org/tools/install))
- `cargo` (Rust package manager and build tool)

## Installation

1. **Clone the repository** (or create a new project):
   ```sh
   cargo new sales_chart
   cd sales_chart
   ```

2. Add dependencies to `Cargo.toml`:
  ```toml  
  [dependencies]
  plotters = "0.4"
  chrono = "0.4"
  csv = "1.1"
  ```

3. Replace `src/main.rs` with the provided code.

## Usage

1. **Prepare your CSV file**: Ensure the CSV file is named `sales_data.csv` and located in the project directory. The CSV should have the following columns: `month`, `product`, and `sales_amount`.

   Example CSV:
   ```csv
   month,product,sales_amount
   2023-01,Product A,100.50
   2023-01,Product B,200.75
   2023-02,Product A,150.25
   2023-02,Product B,180.00
  ```

2. Build the project:
  ```sh
  cargo build
  ```

3. Run the project:
  ```sh
  cargo run
  ```

4. View the output: Check the sales_chart.png file in your project directory for the generated charts.

## Testing

The project includes unit tests to ensure the correctness of data processing and chart generation. To run the tests:
```sh
cargo test
```

## Unit Test

The unit tests include a function to test the CSV processing functionality. It creates a test CSV file with sample data and verifies that the data is processed correctly.




