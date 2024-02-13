use std::net::{SocketAddr, UdpSocket};
use std::thread::sleep;
use std::time::Duration;
use std::{env, fmt, thread};

mod common;
use common::lector_csv::{leer_linea_csv_desde, open_csv};
use common::mensaje_protocolo::TipoDeMensaje;
use common::orden::{Direccion, Orden};
use common::socket::{id_to_addr_local, Socket};

const ECOMMERCE_PUERTO_BASE: u32 = 1024;
const ECOMMERCE_ADDR_BASE: &str = "127.0.0.1";
const ECOMMERCE_UDP_ADDR_BASE: &str = "127.0.0.1:555";

#[derive(Debug, PartialEq)]
pub enum ErrorEcommerce {
    SocketTimeOut,
}

impl fmt::Display for ErrorEcommerce {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ErrorEcommerce::SocketTimeOut => write!(f, "Timeout en socket recv"),
        }
    }
}

#[derive(Debug)]
struct InfoLocal {
    id: usize,
    direccion: Direccion,
}

fn main() {
    let mut args = env::args().skip(1);
    let id = args
        .next()
        .expect("Falta parametro del id")
        .parse::<u32>()
        .expect("No es un numero");

    let puerto = ECOMMERCE_PUERTO_BASE;
    let mut socket = Socket::new(String::from(ECOMMERCE_ADDR_BASE), puerto, id);
    println!("[Ecommerce] id {} con puerto {} creado", id, puerto + id);

    thread::spawn(move || {
        if id == 1 {
            socket.esperar_conexiones(id);
        } else {
            leer_orden_y_enviarsela_al_local(&mut socket, id);
        }
    })
    .join()
    .unwrap();
}

fn leer_orden_y_enviarsela_al_local(socket_ecommerce: &mut Socket, id: u32) {
    let socket = UdpSocket::bind(format!("{}{}", ECOMMERCE_UDP_ADDR_BASE, id)).unwrap();
    let locales: Vec<InfoLocal> = (1..3)
        .map(|id| InfoLocal {
            id,
            direccion: Direccion::new((id * 2 + 1) as i32, (id * 4 + 1) as i32),
        })
        .collect();

    println!("Locales: {:?}", locales);
    println!("[Ecommerce] Abriendo archivo ordenes");
    let path = format!(
        "{}{}",
        env!("CARGO_MANIFEST_DIR"),
        "/data/ordenes_ecommerce.txt"
    );

    loop {
        let cursor = socket_ecommerce.quiero_enviar_ordenes();
        let mut ordenes_reader = open_csv(&path).unwrap();
        println!("[Ecommerce] Empezando a leer ordenes");
        println!("[Ecommerce] Leo una orden con cursor {}", cursor);
        //TODO cambiar el estado o bloquear orden actual o enviar cursor
        let orden = match leer_linea_csv_desde(&mut ordenes_reader, Orden::from_record, cursor) {
            Ok(orden) => orden,
            Err(err) => {
                eprintln!("Error: {}", err);
                if err.to_string() == "No se encontraron más registros en el csv" {
                    socket_ecommerce.desconexion();
                    break;
                }
                panic!("Error al leer ordenes");
            }
        };
        drop(ordenes_reader);
        // termine de leer libero el mutex
        socket_ecommerce.ordenes_enviadas();

        let mut locales_visitados = vec![];

        let orden_serializada = serde_json::to_string(&orden).unwrap();
        let mut buffer = [0; 100];
        let mut orden_aceptada = false;

        while !orden_aceptada && (locales_visitados.len() < locales.len()) {
            let local_seleccionado =
                seleccionar_local_mas_cercano(&orden, &locales, &locales_visitados);
            println!(
                "[Ecommerce] Envio orden {} a local {} con addr {}",
                orden_serializada,
                local_seleccionado,
                id_to_addr_local(local_seleccionado)
            );

            if let Ok((size, from)) = enviar_orden(
                &socket,
                &orden_serializada,
                &mut buffer,
                &local_seleccionado,
            ) {
                let buffer_sin_ceros = &mut buffer[..size];
                let mensaje = String::from_utf8(Vec::from(buffer_sin_ceros)).unwrap();

                println!("[Ecommerce] recibí {} de {}", mensaje, from);
                if TipoDeMensaje::OrdenAceptada.value() == mensaje {
                    orden_aceptada = true;
                } else {
                    locales_visitados.push(local_seleccionado);
                }
            } else {
                locales_visitados.push(local_seleccionado);
                continue;
            }
        }

        if !orden_aceptada {
            println!("[Ecommerce] Ningun local pudo aceptar la orden, se desestima")
        }
        sleep(Duration::from_millis(1000));
    }
}

fn enviar_orden(
    socket: &UdpSocket,
    orden_serializada: &String,
    buffer: &mut [u8; 100],
    local_seleccionado: &usize,
) -> Result<(usize, SocketAddr), ErrorEcommerce> {
    socket
        .send_to(
            orden_serializada.as_bytes(),
            id_to_addr_local(*local_seleccionado),
        )
        .unwrap();

    socket
        .set_read_timeout(Some(Duration::from_millis(500)))
        .unwrap();
    let (size, from) = match socket.recv_from(buffer) {
        Ok((size, from)) => (size, from),
        Err(err) => {
            println!("[Ecommerce - Error] timeout recv: {}", err);
            return Err(ErrorEcommerce::SocketTimeOut);
        }
    };

    Ok((size, from))
}

fn seleccionar_local_mas_cercano(
    orden: &Orden,
    locales: &[InfoLocal],
    locales_visitados: &[usize],
) -> usize {
    let mut distancia_minima = f64::MAX;
    let mut id_seleccionado = 0;

    for local in locales.iter() {
        if locales_visitados.contains(&local.id) {
            continue;
        }
        let distancia = orden.direccion.distancia(&local.direccion);
        if distancia < distancia_minima {
            distancia_minima = distancia;
            id_seleccionado = local.id;
        }
    }

    id_seleccionado
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dado_local_latitud_5_longitud_5_y_local_latitud_10_longitud_10_cuando_calculo_local_mas_cercano_a_orden_direccion_3_3_obtengo_primer_local(
    ) {
        let locales: Vec<InfoLocal> = (1..3)
            .map(|id| InfoLocal {
                id,
                direccion: Direccion::new((id * 5) as i32, (id * 5) as i32),
            })
            .collect();
        let orden = Orden::new(1, 5, 3, 3);
        let locales_visitados = vec![];
        let local_seleccionado =
            seleccionar_local_mas_cercano(&orden, &locales, &locales_visitados);

        assert_eq!(local_seleccionado, locales[0].id);
    }
}
