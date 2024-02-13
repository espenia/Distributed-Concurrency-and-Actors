use std::error::Error;
use std::fs::File;

use csv::{Reader, ReaderBuilder, StringRecord};

pub fn open_csv(file_path: &str) -> Result<Reader<File>, std::io::Error> {
    let file = File::open(file_path)?;
    let reader = ReaderBuilder::new().has_headers(true).from_reader(file);
    Ok(reader)
}

pub fn leer_linea_csv_desde<F>(
    reader: &mut Reader<File>,
    constructor: fn(StringRecord) -> Result<F, Box<dyn Error>>,
    cursor: i32,
) -> Result<F, Box<dyn Error>> {
    let mut size = 0;
    for (i, result) in reader.records().into_iter().enumerate() {
        size += 1;
        if cursor as usize != i {
            continue;
        }
        let record = result?;
        let stock_producto = constructor(record)?;
        return Ok(stock_producto);
    }
    println!("sali del for con {}", size);
    // deberia romper si llega aca y tirar No se encontraron más registros en el csv
    let result = reader
        .records()
        .next()
        .ok_or("No se encontraron más registros en el csv")?;
    let record = result?;
    let stock_producto = constructor(record)?;
    return Ok(stock_producto);
}

pub fn leer_linea_csv<F>(
    reader: &mut Reader<File>,
    constructor: fn(StringRecord) -> Result<F, Box<dyn Error>>,
) -> Result<F, Box<dyn Error>> {
    let result = reader
        .records()
        .next()
        .ok_or("No se encontraron más registros en el csv")?;
    let record = result?;
    let stock_producto = constructor(record)?;
    Ok(stock_producto)
}

#[cfg(test)]
pub mod test_util {
    use csv::Writer;
    use std::fs::File;

    pub fn crear_archivo(test_file: &str) -> Writer<File> {
        csv::Writer::from_path(test_file).unwrap()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::common::orden::Orden;
    use crate::common::stock_producto::StockProducto;
    use test_util::crear_archivo;

    #[test]
    fn test_read_csv_line() {
        // Assuming a CSV file with two columns: "Value1,Value2"
        let test_file = "test.csv";
        let mut wtr = csv::Writer::from_path(test_file).unwrap();
        wtr.write_record(["id_producto", "cantidad", "latitud", "longitud"])
            .unwrap();
        wtr.write_record(["1", "5", "-32", "47"]).unwrap();
        wtr.flush().unwrap();

        let mut reader = open_csv(test_file).unwrap();
        let result = leer_linea_csv(&mut reader, Orden::from_record);

        // Clean up the test file
        std::fs::remove_file(test_file).unwrap();

        // Check if the result is as expected
        assert!(result.is_ok());

        let csv_row = result.unwrap();
        assert_eq!(csv_row.id_producto, 1);
        assert_eq!(csv_row.cantidad, 5);
    }

    #[test]
    fn test_devuelve_error_si_no_tiene_mas_ordenes() {
        // Assuming a CSV file with two columns: "Value1,Value2"
        let test_file = "test_devuelve_error_si_no_tiene_mas_ordenes.csv";
        let mut wtr = csv::Writer::from_path(test_file).unwrap();
        wtr.write_record(["id_producto", "cantidad", "latitud", "longitud"])
            .unwrap();
        wtr.write_record(["1", "5", "-32", "47"]).unwrap();
        wtr.flush().unwrap();

        let mut reader = open_csv(test_file).unwrap();
        let primera_orden = leer_linea_csv(&mut reader, Orden::from_record);
        let segunda_orden = leer_linea_csv(&mut reader, Orden::from_record);

        // Clean up the test file
        std::fs::remove_file(test_file).unwrap();

        assert!(primera_orden.is_ok());
        assert!(segunda_orden.is_err());
    }

    #[test]
    fn test_leer_linea_csv_desde_devuelve_error_si_no_tiene_mas_ordenes() {
        // Assuming a CSV file with two columns: "Value1,Value2"
        let test_file = "test_leer_linea_csv_desde_devuelve_error_si_no_tiene_mas_ordenes.csv";
        let mut wtr = csv::Writer::from_path(test_file).unwrap();
        wtr.write_record(["id_producto", "cantidad", "latitud", "longitud"])
            .unwrap();
        wtr.write_record(["1", "5", "-32", "47"]).unwrap();
        wtr.flush().unwrap();

        let mut reader = open_csv(test_file).unwrap();
        let primera_orden = leer_linea_csv_desde(&mut reader, Orden::from_record, 0);
        let segunda_orden = leer_linea_csv_desde(&mut reader, Orden::from_record, 1);

        // Clean up the test file
        std::fs::remove_file(test_file).unwrap();

        assert!(primera_orden.is_ok());
        assert!(segunda_orden.is_err());
    }

    // Leer stock
    #[test]
    fn test_leer_stock() {
        let test_file = "test_stock.csv";
        let mut wtr = crear_archivo(test_file);
        wtr.write_record(["id_producto", "stock"]).unwrap();
        wtr.write_record(["1", "5"]).unwrap();
        wtr.flush().unwrap();

        let mut reader = open_csv(test_file).unwrap();
        let result: Result<StockProducto, Box<dyn Error>> =
            leer_linea_csv(&mut reader, StockProducto::from_record);

        // Clean up the test file
        std::fs::remove_file(test_file).unwrap();

        // Check if the result is as expected
        assert!(result.is_ok());

        let stock_producto = result.unwrap();
        assert_eq!(stock_producto.id_producto, 1);
        assert_eq!(stock_producto.stock, 5);
    }

    #[test]
    fn test_dado_stock_producto_con_5_unidades_y_0_bloqueadas_al_bloquear_3_devuelve_ok() {
        let mut stock_producto = StockProducto {
            id_producto: 1,
            stock: 5,
            bloqueados: 0,
        };

        let resultado = stock_producto.bloquear(3);

        assert!(resultado.is_ok());
        assert_eq!(stock_producto.bloqueados, 3);
    }
}
