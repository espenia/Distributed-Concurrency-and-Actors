use csv::StringRecord;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;

#[derive(Debug, PartialEq, PartialOrd, Serialize, Deserialize, Clone)]
pub struct Direccion {
    latitud: i32,
    longitud: i32,
}

impl Direccion {
    pub fn new(latitud: i32, longitud: i32) -> Self {
        Direccion { latitud, longitud }
    }

    pub fn distancia(&self, other: &Direccion) -> f64 {
        // Formula Haversine
        const R: f64 = 6371.0; // Radio de la tierra

        let d_lat = ((other.latitud - self.latitud) as f64).to_radians();
        let d_lon = ((other.longitud - self.longitud) as f64).to_radians();

        let a = (d_lat / 2.0).sin() * (d_lat / 2.0).sin()
            + (self.latitud as f64).to_radians().cos()
                * (other.latitud as f64).to_radians().cos()
                * (d_lon / 2.0).sin()
                * (d_lon / 2.0).sin();

        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

        R * c
    }
}

#[derive(Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Orden {
    pub id_producto: usize,
    pub cantidad: usize,
    pub direccion: Direccion,
}

impl Orden {
    /// Solo para testing
    pub fn new(id_producto: usize, cantidad: usize, latitud: i32, longitud: i32) -> Orden {
        Orden {
            id_producto,
            cantidad,
            direccion: Direccion::new(latitud, longitud),
        }
    }

    /// Crea Orden a partir de un record de csv
    pub fn from_record(record: StringRecord) -> Result<Self, Box<dyn Error>> {
        let id_producto = record
            .get(0)
            .ok_or("Orden no tiene id_producto")?
            .parse::<usize>()?;
        let cantidad = record
            .get(1)
            .ok_or("Orden no tiene cantidad")?
            .parse::<usize>()?;
        let latitud = record
            .get(2)
            .ok_or("Orden no tiene latitud")?
            .parse::<i32>()?;
        let longitud = record
            .get(3)
            .ok_or("Orden no tiene longitud")?
            .parse::<i32>()?;

        Ok(Orden {
            id_producto,
            cantidad,
            direccion: Direccion::new(latitud, longitud),
        })
    }

    /// Crea orden a partir de un array de bytes en json
    pub fn deserializar(data: &[u8]) -> Result<Self, serde_json::Error> {
        let orden: Orden = serde_json::from_slice(data)?;
        Ok(orden)
    }
}

impl fmt::Display for Orden {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{},{}", self.id_producto, self.cantidad)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_deserializacion_orden() {
        let orden = Orden {
            id_producto: 1,
            cantidad: 5,
            direccion: Direccion::new(32, 43),
        };
        let json = json!(orden).to_string();
        let orden_serializada = json.as_bytes();

        let orden_deserializada = Orden::deserializar(&orden_serializada).unwrap();

        assert_eq!(orden_deserializada.cantidad, orden.cantidad);
        assert_eq!(orden_deserializada.id_producto, orden.id_producto);
    }

    #[test]
    fn test_deserializar_recibe_1comma256n_y_devuelve_orden_con_id_1_ycantidad_256() {
        let orden = Orden {
            id_producto: 1,
            cantidad: 256,
            direccion: Direccion::new(12, 43),
        };
        let json = json!(orden).to_string();
        let orden_serializada = json.as_bytes();

        let orden = Orden::deserializar(orden_serializada).unwrap();

        assert_eq!(
            orden,
            Orden {
                id_producto: 1,
                cantidad: 256,
                direccion: Direccion::new(12, 43)
            }
        );
    }

    #[test]
    fn test_deserializar_recibe_1comma123456n_y_devuelve_orden_con_id_1_ycantidad_123456() {
        let orden = Orden {
            id_producto: 1,
            cantidad: 123456,
            direccion: Direccion::new(33, 19),
        };
        let json = json!(orden).to_string();
        let orden_serializada = json.as_bytes();

        let orden = Orden::deserializar(orden_serializada).unwrap();

        assert_eq!(
            orden,
            Orden {
                id_producto: 1,
                cantidad: 123456,
                direccion: Direccion::new(33, 19)
            }
        );
    }

    #[test]
    fn test_distancia_al_mismo_punto_es_cero() {
        let direccion = Direccion::new(10, 20);
        assert_eq!(direccion.distancia(&direccion), 0.0);
    }

    #[test]
    fn test_dsitancia_entre_dos_puntos() {
        let direccion = Direccion::new(45, 45);
        let direccion_2 = Direccion::new(50, 50);
        assert!((direccion.distancia(&direccion_2) - 671.27f64) < 1.0f64);
    }
}
