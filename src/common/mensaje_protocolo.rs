use serde::{Deserialize, Serialize};

use crate::common::error_local::ErrorLocal;
use crate::common::orden::{Direccion, Orden};

use crate::common::socket::TipodeError;

#[derive(Clone)]
pub struct MensajeProtocolo {
    tipo_de_mensaje: TipoDeMensaje,
    tamanio: u8,
    contenido: Vec<u8>,
}
#[derive(Clone)]
pub enum TipoDeMensaje {
    Conexion,
    Orden,
    StockInsuficiente,
    OrdenAceptada,
    QuieroMandarOrdenes,
    PuedoMandarOrdenes,
    TermineDeMandarOrdenes,
    Desconexion,
}

impl TipoDeMensaje {
    pub fn value(&self) -> &'static str {
        match *self {
            TipoDeMensaje::StockInsuficiente => "StockInsuficiente",
            TipoDeMensaje::OrdenAceptada => "OrdenAceptada",
            TipoDeMensaje::Orden => "Orden",
            TipoDeMensaje::Conexion => "Conexion",
            TipoDeMensaje::QuieroMandarOrdenes => "QuieroMandarOrdenes",
            TipoDeMensaje::PuedoMandarOrdenes => "PuedoMandarOrdenes",
            TipoDeMensaje::TermineDeMandarOrdenes => "TermineDeMandarOrdenes",
            TipoDeMensaje::Desconexion => "Desconexion",
        }
    }

    pub fn new_tipo_de_mensaje(tipo_de_mensaje: u8) -> Result<TipoDeMensaje, TipodeError> {
        match tipo_de_mensaje {
            0 => Ok(TipoDeMensaje::Conexion),
            1 => Ok(TipoDeMensaje::Orden),
            2 => Ok(TipoDeMensaje::StockInsuficiente),
            3 => Ok(TipoDeMensaje::OrdenAceptada),
            4 => Ok(TipoDeMensaje::QuieroMandarOrdenes),
            5 => Ok(TipoDeMensaje::PuedoMandarOrdenes),
            6 => Ok(TipoDeMensaje::TermineDeMandarOrdenes),
            7 => Ok(TipoDeMensaje::Desconexion),
            _ => Err(TipodeError::ErrorTipoDeMensaje),
        }
    }

    pub fn from_error_local(error_local: &ErrorLocal) -> TipoDeMensaje {
        match error_local {
            ErrorLocal::StockInsuficiente => TipoDeMensaje::StockInsuficiente,
            ErrorLocal::NoExisteProductoEnLocal => TipoDeMensaje::StockInsuficiente,
            ErrorLocal::CantidadOrdenMayorQueBloqueados => TipoDeMensaje::StockInsuficiente,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Conexion {
    pub nombre: String,
}

impl Conexion {
    pub fn deserializar(data: &[u8]) -> Result<Self, serde_json::Error> {
        let conexion: Conexion = serde_json::from_slice(data)?;
        Ok(conexion)
    }
}

#[derive(Serialize, Deserialize)]
pub struct MandarOrdenes {
    pub cursor: i32,
}

impl MensajeProtocolo {
    pub fn new(tipo_de_mensaje: TipoDeMensaje, contenido: Vec<u8>) -> Self {
        let tamanio = contenido.len() as u8;
        MensajeProtocolo {
            tipo_de_mensaje,
            tamanio,
            contenido,
        }
    }
    pub fn new_conexion(data: Conexion) -> Result<Self, serde_json::Error> {
        let contenido = serde_json::to_vec(&data)?;
        Ok(MensajeProtocolo::new(TipoDeMensaje::Conexion, contenido))
    }

    pub fn new_orden(data: Orden) -> Result<Self, serde_json::Error> {
        let contenido = serde_json::to_vec(&data)?;
        Ok(MensajeProtocolo::new(TipoDeMensaje::Orden, contenido))
    }
    pub fn new_desconexion() -> Self {
        MensajeProtocolo::new(TipoDeMensaje::Desconexion, Vec::new())
    }
    pub fn new_termino_de_mandar_ordenes() -> Self {
        MensajeProtocolo::new(TipoDeMensaje::TermineDeMandarOrdenes, Vec::new())
    }

    pub fn new_orden_aceptada() -> Self {
        MensajeProtocolo::new(TipoDeMensaje::OrdenAceptada, Vec::new())
    }

    pub fn new_stock_insuficiente() -> Self {
        MensajeProtocolo::new(TipoDeMensaje::StockInsuficiente, Vec::new())
    }

    pub fn new_quiero_mandar_ordenes() -> Self {
        MensajeProtocolo::new(TipoDeMensaje::QuieroMandarOrdenes, Vec::new())
    }

    pub fn new_puedo_mandar_ordenes(cursor: i32) -> Self {
        let contenido = serde_json::to_vec(&MandarOrdenes { cursor }).unwrap();
        MensajeProtocolo::new(TipoDeMensaje::PuedoMandarOrdenes, contenido)
    }

    pub fn get_tipo_de_mensaje(&self) -> TipoDeMensaje {
        self.tipo_de_mensaje.clone()
    }

    pub fn get_tamanio(&self) -> u8 {
        self.tamanio
    }

    pub fn get_contenido(&self) -> &Vec<u8> {
        &self.contenido
    }
}

#[cfg(test)]
mod tests {

    use crate::common::orden::Direccion;

    use super::*;

    #[test]
    fn test_deserializacion_orden() {
        let conexion = Conexion {
            nombre: "nombre".to_string(),
        };
        let conexion_serializada = serde_json::to_vec(&conexion).unwrap();

        let conexion_deserializada = Conexion::deserializar(&conexion_serializada).unwrap();
        assert_eq!(conexion_deserializada.nombre, conexion.nombre);
    }

    #[test]
    fn test_serializacion_y_deserializacion_orden() {
        let orden = Orden {
            id_producto: 1,
            cantidad: 5,
            direccion: Direccion::new(0, 0),
        };
        let serialized = serde_json::to_string(&orden).unwrap();

        let deserialized: Orden = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.id_producto, orden.id_producto);
    }

    #[test]
    fn test_serializacion_y_deserializacion_de_orden_usando_slice() {
        let orden = Orden {
            id_producto: 1,
            cantidad: 5,
            direccion: Direccion::new(0, 0),
        };
        let serialized = serde_json::to_string(&orden).unwrap();

        let bytes = serialized.as_bytes();

        let deserialized: Orden = serde_json::from_slice(bytes).unwrap();
        assert_eq!(deserialized.id_producto, orden.id_producto);
    }
}
