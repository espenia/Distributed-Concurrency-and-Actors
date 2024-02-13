use std::error::Error;
use std::fmt;

/// Se wrapean los errores de local en un tipo custom
/// https://doc.rust-lang.org/rust-by-example/error/multiple_error_types/wrap_error.html
#[derive(Debug, PartialEq)]
pub enum ErrorLocal {
    StockInsuficiente,
    NoExisteProductoEnLocal,
    CantidadOrdenMayorQueBloqueados,
}

impl fmt::Display for ErrorLocal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ErrorLocal::StockInsuficiente => write!(f, "No hay stock suficiente en el local"),
            ErrorLocal::NoExisteProductoEnLocal => write!(f, "No exste el producto en el local"),
            ErrorLocal::CantidadOrdenMayorQueBloqueados => write!(
                f,
                "La cantidad de la orden supera a la cantidad de bloqueados"
            ),
        }
    }
}

impl Error for ErrorLocal {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match *self {
            ErrorLocal::StockInsuficiente => None,
            ErrorLocal::NoExisteProductoEnLocal => None,
            ErrorLocal::CantidadOrdenMayorQueBloqueados => None,
        }
    }
}
