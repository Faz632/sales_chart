use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::sync::Arc;
use chrono::NaiveDate;
use csv::{ReaderBuilder, StringRecord};
use plotters::prelude::*;
use rayon::prelude::*;

type DateKey = i32;

fn date_to_key(date: &NaiveDate) -> DateKey {
    date.num_days_from_ce()
}

fn key_to_date(key: DateKey) -> NaiveDate {
    NaiveDate::from_num_days_from_ce(key)
}

fn validate_csv_structure(headers: &StringRecord) -> Result<(), Box<dyn Error>> {
    if headers.len() != 3 {
        return Err("Invalid column length".into());
    }

    let expected_headers = ["month", "product", "sales_amount"];
    for &expected in &expected_headers {
        if !headers.iter().any(|h| h.to_lowercase() == expected) {
            return Err(format!("Missing column: {}", expected).into());
        }
    }

    Ok(())
}

fn process_sales_data(rdr: &mut csv::Reader<File>) -> Result<(HashMap<DateKey, f64>, HashMap<String, f64>), Box<dyn Error>> {
    let headers = rdr.headers()?.clone();
    validate_csv_structure(&headers)?;

    let month_index = headers.iter().position(|h| h.to_lowercase() == "month").unwrap();
    let product_index = headers.iter().position(|h| h.to_lowercase() == "product").unwrap();
    let sales_index = headers.iter().position(|h| h.to_lowercase() == "sales_amount").unwrap();

    let records: Vec<StringRecord> = rdr.records().collect::<Result<_, _>>()?;

    let (sales_by_month, sales_by_product): (HashMap<DateKey, f64>, HashMap<String, f64>) = records
        .par_iter()
        .try_fold(
            || (HashMap::new(), HashMap::new()),
            |(mut sales_by_month, mut sales_by_product), record| {
                if record.len() != 3 {
                    return Err("Invalid column length in data row".into());
                }

                let date_str = &record[month_index];
                let month = NaiveDate::parse_from_str(&format!("{}-01", date_str), "%Y-%m-%d")
                    .map_err(|e| format!("Invalid date format in \"{}\": {}", date_str, e))?;
                let product = record[product_index].to_string();
                let sales: f64 = record[sales_index]
                    .parse()
                    .map_err(|e| format!("Invalid sales number in \"{}\": {}", record[sales_index].to_string(), e))?;

                *sales_by_month.entry(date_to_key(&month)).or_insert(0.0) += sales;
                *sales_by_product.entry(product).or_insert(0.0) += sales;

                Ok((sales_by_month, sales_by_product))
            },
        )
        .try_reduce(
            || (HashMap::new(), HashMap::new()),
            |(mut acc_month, mut acc_product), (month, product)| {
                for (k, v) in month {
                    *acc_month.entry(k).or_insert(0.0) += v;
                }
                for (k, v) in product {
                    *acc_product.entry(k).or_insert(0.0) += v;
                }
                Ok((acc_month, acc_product))
            },
        )?;

    Ok((sales_by_month, sales_by_product))
}

fn prepare_data_for_plotting(sales_by_month: HashMap<DateKey, f64>, sales_by_product: HashMap<String, f64>) 
    -> (Vec<(NaiveDate, f64)>, Vec<(String, f64)>) {
    let mut monthly_data: Vec<(NaiveDate, f64)> = sales_by_month
        .into_par_iter()
        .map(|(k, v)| (key_to_date(k), v))
        .collect();
    monthly_data.par_sort_unstable_by_key(|&(date, _)| date);

    let mut product_data: Vec<(String, f64)> = sales_by_product.into_iter().collect();
    product_data.par_sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    (monthly_data, product_data)
}

fn create_line_chart(monthly_data: &[(NaiveDate, f64)]) -> Result<(), Box<dyn Error>> {
    let root = BitMapBackend::new("line_chart.png", (800, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .caption("Monthly Sales Trend", ("sans-serif", 30).into_font())
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(
            monthly_data.first().unwrap().0..monthly_data.last().unwrap().0,
            0f64..monthly_data.iter().map(|(_, v)| *v).fold(0f64, f64::max),
        )?;

    chart.configure_mesh().draw()?;

    chart
        .draw_series(LineSeries::new(
            monthly_data.iter().map(|(x, y)| (*x, *y)),
            &RED,
        ))?
        .label("Total Sales")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

    chart.configure_series_labels().draw()?;

    root.present()?;
    println!("Line chart saved as line_chart.png");
    Ok(())
}

fn create_bar_chart(product_data: &[(String, f64)]) -> Result<(), Box<dyn Error>> {
    let root = BitMapBackend::new("bar_chart.png", (800, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let mut chart = ChartBuilder::on(&root)
        .caption("Sales by Product", ("sans-serif", 30).into_font())
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(
            0..product_data.len(),
            0f64..product_data.iter().map(|(_, v)| *v).fold(0f64, f64::max),
        )?;

    chart.configure_mesh().draw()?;

    chart.draw_series(
        product_data.iter().enumerate().map(|(i, (_product, sales))| {
            let color = Palette99::pick(i).mix(0.9);
            let mut bar = Rectangle::new([(i, 0.0), (i + 1, *sales)], color.filled());
            bar.set_margin(0, 0, 5, 5);
            bar
        }),
    )?;

    chart.draw_series(
        product_data.iter().enumerate().map(|(i, (product, sales))| {
            EmptyElement::at((i, *sales))
                + Text::new(
                    format!("{}: ${:.2}", product, sales),
                    (0, 15),
                    ("sans-serif", 15).into_font(),
                )
        }),
    )?;

    root.present()?;
    println!("Bar chart saved as bar_chart.png");
    Ok(())
}

fn create_pie_chart(product_data: &[(String, f64)]) -> Result<(), Box<dyn Error>> {
    let root = BitMapBackend::new("pie_chart.png", (800, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    let total_sales: f64 = product_data.iter().map(|(_, sales)| sales).sum();
    let drawing_area = root.centered_at((400, 300));
    let size = 300;

    let mut chart = ChartBuilder::on(&drawing_area)
        .caption("Sales by Product", ("sans-serif", 30).into_font())
        .build_cartesian_2d(-1.0..1.0, -1.0..1.0)?;

    chart.configure_mesh().disable_mesh().draw()?;

    let mut start_angle = 0.0;
    for (idx, (product, sales)) in product_data.iter().enumerate() {
        let angle = sales / total_sales * 360.0;
        let color = Palette99::pick(idx);

        chart.draw_series(std::iter::once(Sector::new(
            (0, 0),
            size,
            start_angle.deg(),
            (start_angle + angle).deg(),
            color.filled(),
        )))?;

        let mid_angle = start_angle + angle / 2.0;
        let (x, y) = (mid_angle.cos(), mid_angle.sin());
        
        chart.draw_series(std::iter::once(Text::new(
            format!("{}: ${:.2} ({:.1}%)", product, sales, sales / total_sales * 100.0),
            (x * size as f64 * 0.7, y * size as f64 * 0.7),
            ("sans-serif", 15).into_font(),
        )))?;

        start_angle += angle;
    }

    root.present()?;
    println!("Pie chart saved as pie_chart.png");
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let file = File::open("large_sales_data.csv")?;
    let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(file);

    match process_sales_data(&mut rdr) {
        Ok((sales_by_month, sales_by_product)) => {
            let (monthly_data, product_data) = prepare_data_for_plotting(sales_by_month, sales_by_product);
            create_line_chart(&monthly_data)?;
            create_bar_chart(&product_data)?;
            create_pie_chart(&product_data)?;
            println!("All charts created successfully!");
        }
        Err(e) => {
            eprintln!("Error processing sales data: {}", e);
            return Err(e);
        }
    }

    Ok(())
}