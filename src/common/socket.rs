use crate::common::conexion_tcp::ConexionTcp;
use crate::common::mensaje_protocolo::{Conexion, MandarOrdenes, MensajeProtocolo, TipoDeMensaje};
use std::cmp;
use std::fmt::{Debug, Display, Formatter};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::{error::Error, fmt};

use crate::common::cordinador::{connect_to_leader, coordinar, Cordinador};

const ECOMMERCE_PUERTO_BASE: u32 = 1024;
const ECOMMERCE_ADDR_BASE: &str = "127.0.0.1";

pub struct ErrorSocket {
    pub error: TipodeError,
}

#[derive(Debug, Clone)]
pub enum TipodeError {
    ErrorLectura,
    ErrorTipoDeMensaje,
}

impl Display for ErrorSocket {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Error type:{0}", self.error.clone() as u8)
    }
}

impl Debug for ErrorSocket {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Error type:{0}", self.error.clone() as u8)
    }
}

impl Error for ErrorSocket {}

pub struct Socket {
    direccion: String,
    puerto: u32,
    listener: Option<TcpListener>,
    id: u32,
    leader: Option<ConexionTcp>,
    cordinador: Arc<Mutex<Cordinador>>,
}

impl Socket {
    pub fn new(direccion: String, puerto: u32, id: u32) -> Socket {
        let leader;
        let listener;
        let cordinador = Arc::new(Mutex::new(Cordinador::new(id)));
        if id != 1 {
            listener = None;
            leader = Some(connect_to_leader(
                format!("{}:{}", String::from(ECOMMERCE_ADDR_BASE), puerto + 1),
                id,
            ));
        } else {
            leader = None;
            listener = Some(TcpListener::bind(format!("{}:{}", direccion, puerto + id)).unwrap());
        }
        Socket {
            direccion: direccion.clone(),
            puerto,
            listener,
            id,
            leader,
            cordinador,
        }
    }

    pub fn get_permiso(&mut self) {
        self.cordinador.clone().try_lock().unwrap().get_permso();
    }

    pub fn release_permiso(&mut self) {
        self.cordinador
            .clone()
            .try_lock()
            .unwrap()
            .release_permiso();
    }

    pub fn get_cordinator_leader_value(id: u32) -> bool {
        id == 1
    }

    pub fn esperar_conexiones(&mut self, id: u32) {
        println!("Esperando conexiones!");
        let cordinador = Arc::new(Mutex::new(Cordinador::new(id)));
        let mut incoming = self.listener.as_ref().unwrap().incoming();
        loop {
            let clone_cordinador = cordinador.clone();
            let opt_stream = incoming.next().unwrap();
            match opt_stream {
                Ok(stream) => {
                    coordinar(stream, clone_cordinador);
                }
                Err(e) => {
                    println!("Error al conectar: {}", e)
                }
            }
        }
    }

    pub fn quiero_enviar_ordenes(&mut self) -> i32 {
        println!("Esperando mandar ordenes!");

        self.leader
            .as_ref()
            .unwrap()
            .enviar_mensaje(&MensajeProtocolo::new_quiero_mandar_ordenes())
            .unwrap();
        let respuesta = match self.leader.as_ref().unwrap().esperar_mensaje() {
            Ok(msg) => msg,
            Err(e) => {
                println!("Error al recibir mensaje: {}", e);
                return -1;
            }
        };
        let cursor = serde_json::from_slice::<MandarOrdenes>(respuesta.get_contenido())
            .unwrap()
            .cursor;
        self.get_permiso(); //TODO esta al dope
        cursor
    }

    pub fn ordenes_enviadas(&mut self) {
        self.leader
            .as_ref()
            .unwrap()
            .enviar_mensaje(&MensajeProtocolo::new_termino_de_mandar_ordenes())
            .unwrap();
        self.release_permiso();
    }

    pub fn desconexion(&mut self) {
        self.leader
            .as_ref()
            .unwrap()
            .enviar_mensaje(&MensajeProtocolo::new_desconexion())
            .unwrap();
        self.release_permiso();
    }
}

pub fn id_to_addr_local(id: usize) -> String {
    "127.0.0.1:1234".to_owned() + &*id.to_string()
}
