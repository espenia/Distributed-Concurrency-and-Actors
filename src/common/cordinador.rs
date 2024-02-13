use std::sync::{Arc, Mutex};
use std::{net::TcpStream, thread};

use crate::common::conexion_tcp::ConexionTcp;
use std_semaphore::Semaphore;

use crate::common::mensaje_protocolo::{Conexion, MensajeProtocolo, TipoDeMensaje};

pub struct Cordinador {
    pub permiso: Semaphore,
    pub cursor: i32,
}

impl Cordinador {
    pub fn new(id: u32) -> Cordinador {
        // TODO reemplazar con el algoritmo de lider
        Cordinador {
            permiso: Semaphore::new(1),
            cursor: 0,
        }
    }
    pub fn get_permso(&self) {
        self.permiso.acquire();
    }

    pub fn release_permiso(&self) {
        self.permiso.release();
    }
}

pub fn setup_conexion(stream: TcpStream) -> ConexionTcp {
    let addr = stream.peer_addr().unwrap();
    let mut conexion = ConexionTcp::new(stream, addr);
    println!("Conexion recibida de: {}", addr);
    match conexion.esperar_mensaje() {
        Ok(msg) => {
            if msg.get_tipo_de_mensaje().value() == TipoDeMensaje::Conexion.value() {
                let conexion_msg = Conexion::deserializar(msg.get_contenido()).unwrap();
                conexion.set_detalles_conexion(conexion_msg);
            } else {
                println!(
                    "Mensaje no reconocido: {}",
                    msg.get_tipo_de_mensaje().value()
                );
            }
        }
        Err(e) => {
            println!("Error al recibir mensaje: {}", e);
        }
    }
    conexion
}

pub(crate) fn connect_to_leader(direccion: String, id: u32) -> ConexionTcp {
    println!("Conectando a {}", direccion);
    let stream = TcpStream::connect(direccion).unwrap();
    let addr = stream.peer_addr().unwrap();
    let conexion = ConexionTcp::new(stream, addr);
    conexion
        .enviar_mensaje(
            &MensajeProtocolo::new_conexion(Conexion {
                nombre: format!("eccomerce_{}", id),
            })
            .unwrap(),
        )
        .unwrap();
    conexion
}

pub fn coordinar(stream: TcpStream, cordinador: Arc<Mutex<Cordinador>>) {
    // TODO agregar a la lista de conexiones y liberar todos juntos
    let _ = thread::spawn(move || coordinar_conexion(stream.try_clone().unwrap(), cordinador));
}

// espero mensaje para mandar ordenes
// si es el mensaje no es lo que espero corto la
// mando mensaje de permiso
// espero mensaje de termine de mandar ordenes
// libero el permiso
// si me envian algo que no es lo que espero tambien libero el permiso

fn coordinar_conexion(stream: TcpStream, cordinador: Arc<Mutex<Cordinador>>) {
    let conexion = setup_conexion(stream);
    println!("Cliente agregado a la lista");
    loop {
        match conexion.esperar_mensaje() {
            Ok(msg) => match msg.get_tipo_de_mensaje() {
                TipoDeMensaje::QuieroMandarOrdenes => {
                    println!(
                        "Recibi un pedido de querer mandar ordenes de parte de [{}]",
                        conexion.get_detalles_conexion().as_ref().unwrap().nombre
                    );
                    let mut cordinador_lock = cordinador.try_lock().unwrap();
                    cordinador_lock.get_permso();
                    println!(
                        "Le di permiso a [{}] para mandar ordenes",
                        conexion.get_detalles_conexion().as_ref().unwrap().nombre
                    );
                    conexion
                        .enviar_mensaje(&MensajeProtocolo::new_puedo_mandar_ordenes(
                            cordinador_lock.cursor,
                        ))
                        .unwrap();
                    match conexion.esperar_mensaje() {
                        Ok(msg_final) => match msg_final.get_tipo_de_mensaje() {
                            TipoDeMensaje::TermineDeMandarOrdenes => {
                                cordinador_lock.cursor += 1;
                                println!(
                                    "Recibi un pedido de terminar de mandar ordenes de parte de [{}]",
                                    conexion.get_detalles_conexion().as_ref().unwrap().nombre
                                );
                                cordinador_lock.release_permiso();
                                drop(cordinador_lock);
                                println!(
                                    "Le quite el permiso a [{}] para mandar ordenes",
                                    conexion.get_detalles_conexion().as_ref().unwrap().nombre
                                );
                            }
                            TipoDeMensaje::Desconexion => {
                                println!(
                                    "Recibi una desconexion de parte de [{}]",
                                    conexion.get_detalles_conexion().as_ref().unwrap().nombre
                                );
                                cordinador_lock.release_permiso();
                                drop(cordinador_lock);
                                println!(
                                    "Le quite el permiso a [{}] para mandar ordenes",
                                    conexion.get_detalles_conexion().as_ref().unwrap().nombre
                                );
                                break;
                            }
                            _ => {
                                println!(
                                    "Mensaje no reconocido: {}",
                                    msg.get_tipo_de_mensaje().value()
                                );
                                cordinador.try_lock().unwrap().release_permiso();
                                println!(
                                    "Le quite el permiso a [{}] para mandar ordenes",
                                    conexion.get_detalles_conexion().as_ref().unwrap().nombre
                                );
                            }
                        },
                        Err(e) => {
                            println!("Error al recibir mensaje: {}", e);
                            cordinador.try_lock().unwrap().release_permiso();
                            break;
                        }
                    }
                }
                _ => {
                    println!(
                        "Mensaje no reconocido: {}",
                        msg.get_tipo_de_mensaje().value()
                    )
                }
            },
            Err(_) => println!("Error al recibir mensaje"),
        }
    }
}
