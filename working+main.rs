use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use chrono::NaiveDate;
use csv::ReaderBuilder;
use plotters::prelude::*;

fn process_sales_data(rdr: &mut csv::Reader<File>) -> Result<(HashMap<NaiveDate, f64>, HashMap<String, f64>), Box<dyn Error>> {
    let mut sales_by_month: HashMap<NaiveDate, f64> = HashMap::new();
    let mut sales_by_product: HashMap<String, f64> = HashMap::new();

    for result in rdr.records() {
        let record = result?;
        if record.len() < 3 {
            eprintln!("Warning: Incomplete data encountered: {:?}", record);
            continue; // Skip incomplete records
        }

        let date_str = &record[0];
        let month = NaiveDate::parse_from_str(&format!("{}-01", date_str), "%Y-%m-%d")
            .map_err(|e| format!("Invalid date format in \"{}\": {}", date_str, e))?;
        let product = record[1].to_string();
        let sales: f64 = record[2]
            .parse()
            .map_err(|e| format!("Invalid sales number in \"{}\": {}", record[2].to_string(), e))?;

        *sales_by_month.entry(month).or_insert(0.0) += sales;
        *sales_by_product.entry(product).or_insert(0.0) += sales;
    }

    Ok((sales_by_month, sales_by_product))
}

fn prepare_data_for_plotting(sales_by_month: HashMap<NaiveDate, f64>, sales_by_product: HashMap<String, f64>) 
    -> (Vec<(NaiveDate, f64)>, Vec<(String, f64)>) {
    let mut monthly_data: Vec<(NaiveDate, f64)> = sales_by_month.into_iter().collect();
    monthly_data.sort_by_key(|&(date, _)| date);

    let mut product_data: Vec<(String, f64)> = sales_by_product.into_iter().collect();
    product_data.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    (monthly_data, product_data)
}

fn create_sales_chart(monthly_data: &[(NaiveDate, f64)], product_data: &[(String, f64)]) -> Result<(), Box<dyn Error>> {
    let root = BitMapBackend::new("sales_chart.png", (1024, 768)).into_drawing_area();
    root.fill(&WHITE)?;

    let (upper, lower) = root.split_vertically(384);

    // Monthly sales trend chart
    let mut chart = ChartBuilder::on(&upper)
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

    // Product sales chart
    let mut chart = ChartBuilder::on(&lower)
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
                    (0, 0),
                    ("sans-serif", 15).into_font(),
                )
        }),
    )?;

    root.present()?;

    println!("Chart saved as sales_chart.png");
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let file = File::open("sales_data.csv")?;
    let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(file);

    let (sales_by_month, sales_by_product) = process_sales_data(&mut rdr)?;
    let (monthly_data, product_data) = prepare_data_for_plotting(sales_by_month, sales_by_product);
    create_sales_chart(&monthly_data, &product_data)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use csv::ReaderBuilder;
    use std::io::Write;

    fn create_test_csv() -> Result<File, Box<dyn Error>> {
        let mut file = File::create("test_sales_data.csv")?;
        writeln!(file, "date,product,sales")?;
        writeln!(file, "2023-01,Product A,100.50")?;
        writeln!(file, "2023-01,Product B,200.75")?;
        writeln!(file, "2023-02,Product A,150.25")?;
        writeln!(file, "2023-02,Product B,180.00")?;
        file.sync_all()?;
        File::open("test_sales_data.csv")
    }

    #[test]
    fn test_process_sales_data() -> Result<(), Box<dyn Error>> {
        let file = create_test_csv()?;
        let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(file);
        
        let (sales_by_month, sales_by_product) = process_sales_data(&mut rdr)?;

        assert_eq!(sales_by_month.len(), 2);
        assert_eq!(sales_by_product.len(), 2);

        let jan_2023 = NaiveDate::from_ymd_opt(2023, 1, 1).unwrap();
        let feb_2023 = NaiveDate::from_ymd_opt(2023, 2, 1).unwrap();

        assert_eq!(sales_by_month.get(&jan_2023), Some(&301.25));
        assert_eq!(sales_by_month.get(&feb_2023), Some(&330.25));

        assert_eq!(sales_by_product.get("Product A"), Some(&250.75));
        assert_eq!(sales_by_product.get("Product B"), Some(&380.75));

        Ok(())
    }

    #[test]
    fn test_prepare_data_for_plotting() {
        let mut sales_by_month = HashMap::new();
        let mut sales_by_product = HashMap::new();

        sales_by_month.insert(NaiveDate::from_ymd_opt(2023, 1, 1).unwrap(), 100.0);
        sales_by_month.insert(NaiveDate::from_ymd_opt(2023, 2, 1).unwrap(), 200.0);

        sales_by_product.insert("Product A".to_string(), 150.0);
        sales_by_product.insert("Product B".to_string(), 150.0);

        let (monthly_data, product_data) = prepare_data_for_plotting(sales_by_month, sales_by_product);

        assert_eq!(monthly_data.len(), 2);
        assert_eq!(product_data.len(), 2);

        assert_eq!(monthly_data[0].0, NaiveDate::from_ymd_opt(2023, 1, 1).unwrap());
        assert_eq!(monthly_data[1].0, NaiveDate::from_ymd_opt(2023, 2, 1).unwrap());

        assert_eq!(monthly_data[0].1, 100.0);
        assert_eq!(monthly_data[1].1, 200.0);

        assert_eq!(product_data[0].1, 150.0);
        assert_eq!(product_data[1].1, 150.0);
    }
}
