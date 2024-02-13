use crate::common::error_local::ErrorLocal;
use crate::common::orden::Orden;
use crate::common::stock_producto::StockProducto;
use std::collections::HashMap;

pub type Productos = HashMap<usize, StockProducto>;

pub struct Local {
    pub productos_en_stock: Productos,
    pub ordenes_en_progreso: Vec<Orden>,
}

impl Local {
    /// Agrega la orden al local y bloquea la cantidad del producto especificado
    pub fn agregar_orden(&mut self, orden: Orden) -> Result<(), ErrorLocal> {
        let producto = self
            .productos_en_stock
            .get_mut(&orden.id_producto)
            .ok_or(ErrorLocal::NoExisteProductoEnLocal)?;
        producto.bloquear(orden.cantidad)?;
        self.ordenes_en_progreso.push(orden);
        Ok(())
    }

    /// Descuenta el stock si tiene la cantidad indicada por la orden
    pub fn vender(&mut self, orden: Orden) -> Result<(), ErrorLocal> {
        let producto = self
            .productos_en_stock
            .get_mut(&orden.id_producto)
            .ok_or(ErrorLocal::NoExisteProductoEnLocal)?;
        producto.vender(orden.cantidad)?;
        Ok(())
    }

    pub fn new(productos: Productos) -> Local {
        Local {
            productos_en_stock: productos,
            ordenes_en_progreso: vec![],
        }
    }

    /// Elige orden de forma aleatoria, reduce stock y bloqueados del producto
    /// segun el id_producto y la cantidad indicada en la orden
    pub fn entregar_orden(&mut self, rng: impl Fn() -> usize) -> Result<(), ErrorLocal> {
        let indice_random = rng();
        if self.ordenes_en_progreso.get(indice_random).is_some() {
            let orden = self.ordenes_en_progreso.remove(indice_random);
            let producto = self
                .productos_en_stock
                .get_mut(&orden.id_producto)
                .ok_or(ErrorLocal::NoExisteProductoEnLocal)?;
            // Transaccionalidad: si falla este método entonces removí la orden pero no desconté los productos de la orden.
            producto.entregar(orden.cantidad)?
        } else {
            eprintln!("[Job - Error] numero random esta por encima del largo del array");
        }

        Ok(())
    }

    /// Elige orden de forma aleatoria, reduce stock y bloqueados del producto
    /// segun el id_producto y la cantidad indicada en la orden
    pub fn cancelar_orden(&mut self, rng: impl Fn() -> usize) -> Result<(), ErrorLocal> {
        let indice_random = rng();
        if self.ordenes_en_progreso.get(indice_random).is_some() {
            let orden = self.ordenes_en_progreso.remove(indice_random);
            let producto = self
                .productos_en_stock
                .get_mut(&orden.id_producto)
                .ok_or(ErrorLocal::NoExisteProductoEnLocal)?;
            // Transaccionalidad: si falla este método entonces removí la orden pero no libere los productos bloqueados.
            producto.cancelar(orden.cantidad)?
        } else {
            eprintln!("[Job - Error] numero random esta por encima del largo del array");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FactoryProductos {
        id_actual: usize,
    }

    impl FactoryProductos {
        fn crear_producto(&mut self, stock: usize, bloqueados: usize) -> StockProducto {
            let id_producto = self.id_actual;
            self.id_actual += 1;
            return StockProducto::new_con_bloqueados(id_producto, stock, bloqueados);
        }
    }

    fn crear_local(productos: Productos) -> Local {
        Local::new(productos)
    }

    #[test]
    fn test_instanciar_local() {
        let productos: Productos = HashMap::new();
        let local = Local::new(productos);

        assert_eq!(local.productos_en_stock.len(), 0);
    }

    #[test]
    fn test_agregar_orden_de_3_productos_id_1_a_local_con_10_productos_id_1_devuelve_ok() {
        let mut productos: Productos = HashMap::new();
        productos.insert(1, StockProducto::new(1, 10));
        let mut local = crear_local(productos);

        let orden = Orden::new(1, 3, 33, 22);

        assert!(local.agregar_orden(orden).is_ok());
        assert_eq!(local.productos_en_stock.get(&1usize).unwrap().bloqueados, 3);
        assert_eq!(local.productos_en_stock.get(&1usize).unwrap().stock, 10);
    }

    #[test]
    fn test_agregar_orden_de_3_productos_id_1_a_local_sin_suficiente_stock_devuelve_error_stock_insuficiente(
    ) {
        let mut productos: Productos = HashMap::new();
        let id_producto = 1;
        productos.insert(id_producto, StockProducto::new(id_producto, 1));
        let mut local = crear_local(productos);

        let orden = Orden::new(1, 3, 33, 22);
        let resultado = local.agregar_orden(orden);
        assert!(resultado.is_err());
        assert_eq!(resultado.unwrap_err(), ErrorLocal::StockInsuficiente);
    }

    #[test]
    fn test_agregar_orden_de_producto_con_id_inexistente_devuelve_error_no_existe_producto_en_local(
    ) {
        let mut productos: Productos = HashMap::new();
        let id_producto = 1;
        productos.insert(id_producto, StockProducto::new(id_producto, 1));
        let mut local = crear_local(productos);

        let orden = Orden::new(2398462734, 3, 33, 22);
        let resultado = local.agregar_orden(orden);
        assert!(resultado.is_err());
        assert_eq!(resultado.unwrap_err(), ErrorLocal::NoExisteProductoEnLocal);
    }

    #[test]
    fn test_agregar_orden_incrementa_la_cantidad_de_ordenes_en_progreso_en_1() {
        let mut productos: Productos = HashMap::new();
        let id_producto = 1;
        productos.insert(id_producto, StockProducto::new(id_producto, 10));
        let mut local = crear_local(productos);

        let orden = Orden::new(1, 3, 33, 22);

        assert!(local.agregar_orden(orden).is_ok());
        assert_eq!(local.ordenes_en_progreso.len(), 1);
        assert_eq!(local.ordenes_en_progreso.last().unwrap().id_producto, 1);
        assert_eq!(local.ordenes_en_progreso.last().unwrap().cantidad, 3);
    }

    #[test]
    fn test_local_vender_orden_producto_id_1_cantidad_10_cuando_stock_20_reduce_en_10_el_stock() {
        let mut productos: Productos = HashMap::new();
        let id_producto = 1;
        productos.insert(id_producto, StockProducto::new(id_producto, 20));
        let mut local = crear_local(productos);

        let orden = Orden::new(1, 10, 33, 22);

        assert!(local.vender(orden).is_ok());
        assert_eq!(local.productos_en_stock.get(&1usize).unwrap().stock, 10);
        assert_eq!(local.productos_en_stock.get(&1usize).unwrap().bloqueados, 0);
    }

    #[test]
    fn test_local_vender_orden_producto_id_1_cantidad_10_cuando_stock_5_devuelve_error() {
        let mut productos: Productos = HashMap::new();
        let id_producto = 1;
        productos.insert(id_producto, StockProducto::new(id_producto, 5));
        let mut local = crear_local(productos);

        let orden = Orden::new(1, 10, 33, 22);

        assert!(local.vender(orden).is_err());
        assert_eq!(local.productos_en_stock.get(&1usize).unwrap().stock, 5);
        assert_eq!(local.productos_en_stock.get(&1usize).unwrap().bloqueados, 0);
    }

    #[test]
    fn test_local_vender_orden_producto_id_1_cantidad_10_cuando_stock_15_y_bloqueados_5_reduce_en_10_el_stock(
    ) {
        let mut productos: Productos = HashMap::new();
        let id_producto = 1;
        productos.insert(
            id_producto,
            StockProducto::new_con_bloqueados(id_producto, 15, 5),
        );
        let mut local = crear_local(productos);

        let orden = Orden::new(1, 10, 33, 22);

        assert!(local.vender(orden).is_ok());
        assert_eq!(local.productos_en_stock.get(&1usize).unwrap().stock, 5);
        assert_eq!(local.productos_en_stock.get(&1usize).unwrap().bloqueados, 5);
    }

    #[test]
    fn test_dado_local_con_1_orden_y_10_stock_producto_id_1_cuando_entrega_orden_5_productos_id_1_entonces_reduce_stock_en_5_y_bloqueados_en_5(
    ) {
        let mut productos: Productos = HashMap::new();
        let id_producto = 1;
        productos.insert(
            id_producto,
            StockProducto::new_con_bloqueados(id_producto, 15, 0),
        );
        let mut local = crear_local(productos);
        let orden = Orden::new(id_producto, 5, 33, 22);
        let _ = local.agregar_orden(orden);

        let resultado = local.entregar_orden(|| 0);

        assert!(resultado.is_ok());
        assert_eq!(local.ordenes_en_progreso.len(), 0);
        assert_eq!(
            local.productos_en_stock.get(&id_producto).unwrap().stock,
            10
        );
        assert_eq!(
            local
                .productos_en_stock
                .get(&id_producto)
                .unwrap()
                .bloqueados,
            0
        );
    }

    #[test]
    fn test_dado_local_con_2_ordenes_cuando_se_entrega_segunda_orden_queda_la_primera_orden_sin_entregar(
    ) {
        let mut productos: Productos = HashMap::new();
        let id_producto = 1;
        productos.insert(
            id_producto,
            StockProducto::new_con_bloqueados(id_producto, 15, 0),
        );
        let mut local = crear_local(productos);

        let orden = Orden::new(id_producto, 5, 33, 22);
        let orden_2 = Orden::new(id_producto, 10, 33, 22);
        let _ = local.agregar_orden(orden);
        let _ = local.agregar_orden(orden_2);

        let resultado = local.entregar_orden(|| 1);

        assert!(resultado.is_ok());
        assert_eq!(local.ordenes_en_progreso.len(), 1);
        assert_eq!(local.productos_en_stock.get(&id_producto).unwrap().stock, 5);
        assert_eq!(
            local
                .productos_en_stock
                .get(&id_producto)
                .unwrap()
                .bloqueados,
            5
        );
    }

    #[test]
    fn test_dado_local_con_1_orden_de_10_productos_id_1_cuando_se_cancela_entonces_no_quedan_ordenes_y_bloqueados_se_reduce_en_10(
    ) {
        let mut productos: Productos = HashMap::new();
        let id_producto = 1;
        productos.insert(
            id_producto,
            StockProducto::new_con_bloqueados(id_producto, 15, 0),
        );
        let mut local = crear_local(productos);
        let orden = Orden::new(id_producto, 10, 33, 22);
        let _ = local.agregar_orden(orden);

        let resultado = local.cancelar_orden(|| 0);

        assert!(resultado.is_ok());
        assert_eq!(local.ordenes_en_progreso.len(), 0);
        assert_eq!(
            local.productos_en_stock.get(&id_producto).unwrap().stock,
            15
        );
        assert_eq!(
            local
                .productos_en_stock
                .get(&id_producto)
                .unwrap()
                .bloqueados,
            0
        );
    }

    #[test]
    fn test_dado_local_con_2_ordenes_cuando_se_cancela_la_segunda_orden_por_10_queda_la_primera_orden_y_bloqueados_se_reduce_en_10(
    ) {
        let mut factory_productos = FactoryProductos { id_actual: 0 };
        let producto = factory_productos.crear_producto(15, 0);
        let id_producto = producto.id_producto;
        let mut productos: Productos = HashMap::new();
        productos.insert(producto.id_producto, producto);
        let mut local = crear_local(productos);

        let orden = Orden::new(id_producto, 5, 33, 22);
        let orden_2 = Orden::new(id_producto, 10, 33, 22);
        let _ = local.agregar_orden(orden);
        let _ = local.agregar_orden(orden_2);

        let resultado = local.cancelar_orden(|| 1);

        assert!(resultado.is_ok());
        assert_eq!(local.ordenes_en_progreso.len(), 1);
        assert_eq!(
            local.productos_en_stock.get(&id_producto).unwrap().stock,
            15
        );
        assert_eq!(
            local
                .productos_en_stock
                .get(&id_producto)
                .unwrap()
                .bloqueados,
            5
        );
    }
}
