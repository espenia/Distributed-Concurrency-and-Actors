use crate::common::error_local::ErrorLocal;
use csv::StringRecord;
use std::error::Error;

#[derive(Debug)]
pub struct StockProducto {
    pub id_producto: usize,
    pub stock: usize,
    pub bloqueados: usize,
}

impl StockProducto {
    pub fn new(id_producto: usize, stock: usize) -> StockProducto {
        StockProducto {
            id_producto,
            stock,
            bloqueados: 0,
        }
    }

    pub fn new_con_bloqueados(
        id_producto: usize,
        stock: usize,
        bloqueados: usize,
    ) -> StockProducto {
        StockProducto {
            id_producto,
            stock,
            bloqueados,
        }
    }

    pub fn from_record(record: StringRecord) -> Result<Self, Box<dyn Error>> {
        let id_producto = record
            .get(0)
            .ok_or("registro no tiene atributo id_producto")?
            .parse::<usize>()?;
        let stock = record
            .get(1)
            .ok_or("registro no tiene atributo stock")?
            .parse::<usize>()?;

        Ok(StockProducto {
            id_producto,
            stock,
            bloqueados: 0,
        })
    }

    /// aumenta la cantidad de bloqueados segun cantidad_a_bloquear
    pub fn bloquear(&mut self, cantidad_a_bloquear: usize) -> Result<(), ErrorLocal> {
        if self.stock - self.bloqueados >= cantidad_a_bloquear {
            self.bloqueados += cantidad_a_bloquear;
            return Ok(());
        }
        Err(ErrorLocal::StockInsuficiente)
    }

    /// reduce stock segun cantidad
    pub fn vender(&mut self, cantidad: usize) -> Result<(), ErrorLocal> {
        if self.stock - self.bloqueados >= cantidad {
            self.stock -= cantidad;
            return Ok(());
        }
        Err(ErrorLocal::StockInsuficiente)
    }

    /// reduce cantidad de bloqueados y stock segun cantidad
    pub fn entregar(&mut self, cantidad: usize) -> Result<(), ErrorLocal> {
        if self.bloqueados < cantidad {
            return Err(ErrorLocal::CantidadOrdenMayorQueBloqueados);
        }
        if self.stock < cantidad {
            return Err(ErrorLocal::StockInsuficiente);
        }
        self.stock -= cantidad;
        self.bloqueados -= cantidad;
        Ok(())
    }

    /// reduce cantidad de bloqueados segun cantidad
    pub fn cancelar(&mut self, cantidad: usize) -> Result<(), ErrorLocal> {
        if self.bloqueados < cantidad {
            return Err(ErrorLocal::CantidadOrdenMayorQueBloqueados);
        }
        if self.stock < cantidad {
            return Err(ErrorLocal::StockInsuficiente);
        }
        self.bloqueados -= cantidad;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vender_10_stock_producto_con_stock_20_y_bloqueados_0_devuelve_ok() {
        let mut stock_producto = StockProducto::new(1, 20);

        let resultado = stock_producto.vender(10);

        assert!(resultado.is_ok());
        assert_eq!(stock_producto.bloqueados, 0);
        assert_eq!(stock_producto.stock, 10);
    }

    #[test]
    fn test_vender_10_stock_producto_con_stock_20_y_bloqueados_19_devuelve_error() {
        let mut stock_producto = StockProducto::new_con_bloqueados(1, 20, 19);

        let resultado = stock_producto.vender(10);

        assert!(resultado.is_err());
        assert_eq!(stock_producto.bloqueados, 19);
        assert_eq!(stock_producto.stock, 20);
    }

    #[test]
    fn test_entregar_10_unidades_producto_id_1_con_stock_20_y_bloqueados_10_reduce_stock_y_bloqueados_en_10(
    ) {
        let mut stock_producto = StockProducto::new_con_bloqueados(1, 20, 10);

        let resultado = stock_producto.entregar(10);

        assert!(resultado.is_ok());
        assert_eq!(stock_producto.stock, 10);
        assert_eq!(stock_producto.bloqueados, 0);
    }

    #[test]
    fn test_cancelar_10_unidades_producto_id_1_con_stock_20_y_bloqueados_10_reduce_bloqueados_en_10(
    ) {
        let mut stock_producto = StockProducto::new_con_bloqueados(1, 20, 10);

        let resultado = stock_producto.cancelar(10);

        assert!(resultado.is_ok());
        assert_eq!(stock_producto.stock, 20);
        assert_eq!(stock_producto.bloqueados, 0);
    }

    #[test]
    fn test_cancelar_30_unidades_producto_id_1_con_stock_30_y_bloqueados_10_devuelve_error_se_cancelan_mas_que_la_cantidad_bloqueados(
    ) {
        let mut stock_producto = StockProducto::new_con_bloqueados(1, 20, 10);

        let resultado = stock_producto.cancelar(30);

        assert!(resultado.is_err());
        assert_eq!(
            resultado.unwrap_err(),
            ErrorLocal::CantidadOrdenMayorQueBloqueados
        );
    }
}
