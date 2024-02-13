use crate::common::mensaje_protocolo::{Conexion, MensajeProtocolo, TipoDeMensaje};
use crate::common::socket::{ErrorSocket, TipodeError};
use std::cmp;
use std::error::Error;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, Mutex};

pub struct ConexionTcp {
    stream_lectura: Arc<Mutex<TcpStream>>,
    stream_escritrua: Arc<Mutex<TcpStream>>,
    addr: SocketAddr,
    detalles_conexion: Option<Conexion>,
}

impl ConexionTcp {
    pub fn new(stream: TcpStream, addr: SocketAddr) -> Self {
        let stream_escritrua = Arc::new(Mutex::new(stream.try_clone().unwrap()));
        ConexionTcp {
            stream_lectura: Arc::new(Mutex::new(stream)),
            stream_escritrua,
            addr,
            detalles_conexion: None,
        }
    }

    pub fn set_detalles_conexion(&mut self, conexion: Conexion) {
        self.detalles_conexion = Some(conexion);
    }

    pub fn get_detalles_conexion(&self) -> &Option<Conexion> {
        &self.detalles_conexion
    }

    pub fn enviar_mensaje(&self, mensaje: &MensajeProtocolo) -> Result<(), Box<dyn Error>> {
        let mut stream = self.stream_escritrua.lock().unwrap();
        let mut buffer = Vec::new();
        buffer.push(mensaje.get_tipo_de_mensaje() as u8);
        buffer.push(mensaje.get_tamanio());
        buffer.append(&mut mensaje.get_contenido().clone());
        match stream.write_all(&buffer) {
            Ok(_) => {
                println!("Mensaje enviado a [{}]", self.addr);
                Ok(())
            }
            Err(e) => {
                println!("Error al enviar mensaje a [{}], [{}]", self.addr, e);
                Err(Box::new(e))
            }
        }
    }

    /// A partir de esos n bytes puedo decodificar el tipo de packet recibido.
    pub fn read_all(stream: &mut TcpStream) -> Result<MensajeProtocolo, Box<dyn Error>> {
        let mut size_buf = [0_u8; 2];
        let msg_size: u32;
        let mensaje: TipoDeMensaje;
        let mut result: Vec<u8> = Vec::new();
        match stream.read_exact(&mut size_buf) {
            Ok(_) => {
                if size_buf[0] == 0 && size_buf[1] == 0 {
                    println!("Local respondió algo que el ecommerce no entendió");
                    panic!("");
                }
                match TipoDeMensaje::new_tipo_de_mensaje(size_buf[0]) {
                    Ok(tipo) => {
                        mensaje = tipo;
                        msg_size = size_buf[1] as u32;
                    }
                    Err(e) => {
                        return Err(Box::new(ErrorSocket { error: e }));
                    }
                }
            }
            Err(ref _e) => {
                return Err(Box::new(ErrorSocket {
                    error: TipodeError::ErrorLectura,
                }));
            }
        }

        // Leer del socket la cantidad de bytes que indica el tamanio del mensaje
        let mut bytes_read: u32 = 0;
        while bytes_read < msg_size {
            let max_limit = cmp::min(msg_size - bytes_read, 1024);
            let mut buf = vec![0; max_limit as usize].into_boxed_slice();
            match stream.read(&mut buf) {
                Ok(size) => {
                    let mut received = Vec::from(&buf[0..size]);
                    result.append(&mut received);
                    bytes_read += size as u32
                }
                Err(_) => {
                    return Err(Box::new(ErrorSocket {
                        error: TipodeError::ErrorLectura,
                    }))
                }
            }
        }
        Ok(MensajeProtocolo::new(mensaje, result))
    }

    pub fn esperar_mensaje(&self) -> Result<MensajeProtocolo, Box<dyn Error>> {
        let mut stream = self.stream_lectura.lock().unwrap();
        Self::read_all(&mut stream)
    }
}
