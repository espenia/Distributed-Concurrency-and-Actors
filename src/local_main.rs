use actix::io::SinkWrite;
use actix::{
    Actor, ActorFutureExt, Addr, AsyncContext, Context, Handler, Message, Recipient,
    ResponseActFuture, StreamHandler, WrapFuture,
};
use actix_rt::net::UdpSocket;
use actix_rt::{Arbiter, System};
use csv::Reader;
use futures::stream::SplitSink;
use futures::StreamExt;
use rand::{thread_rng, Rng};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufRead;
use std::net::SocketAddr;
use std::time::Duration;
use std::{env, io};
use tokio::time::sleep;
use tokio_util::bytes::{Bytes, BytesMut};
use tokio_util::codec::BytesCodec;
use tokio_util::udp::UdpFramed;

mod common;

use common::error_local::ErrorLocal;
use common::lector_csv::{leer_linea_csv, open_csv};
use common::local::{Local, Productos};
use common::mensaje_protocolo::TipoDeMensaje;
use common::orden::Orden;
use common::socket::id_to_addr_local;
use common::stock_producto::StockProducto;

type SinkItem = (Bytes, SocketAddr);
type UdpSink = SplitSink<UdpFramed<BytesCodec, UdpSocket>, SinkItem>;

/// Actor que convierte el socket udp en un stream y maneja los mensajes que recibe a través de él
struct UdpClientActor {
    sink: SinkWrite<SinkItem, UdpSink>,
    aceptar_ordenes: bool,
    recipient_local: Recipient<AgregarOrden>,
}

impl UdpClientActor {
    /// Inicia el actor a partir de un UdpSocket y un recipient capaz de recibir un mensaje del tipo
    /// AgregarOrden
    pub fn start(socket: UdpSocket, recipient: Recipient<AgregarOrden>) -> Addr<UdpClientActor> {
        let (sink, stream) = UdpFramed::new(socket, BytesCodec::new()).split();

        UdpClientActor::create(|ctx| {
            ctx.add_stream(stream.filter_map(
                |item: Result<(BytesMut, SocketAddr), io::Error>| async {
                    item.map(|(data, sender)| UdpPacket(data, sender)).ok()
                },
            ));

            UdpClientActor {
                sink: SinkWrite::new(sink, ctx),
                aceptar_ordenes: true,
                recipient_local: recipient,
            }
        })
    }
}

impl Actor for UdpClientActor {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "()")]
struct UdpPacket(BytesMut, SocketAddr);

/// Cada vez que un mensaje entra por el socket, el mensaje entra en la queue del Actor
/// El socket caido se simula con self.aceptar_ordenes en false
/// Deserializa la orden validando que tenga el formato correcto, si lo tiene le envia un msg al actor Local
impl StreamHandler<UdpPacket> for UdpClientActor {
    fn handle(&mut self, item: UdpPacket, _ctx: &mut Self::Context) {
        println!("[UDP] Recibí: ({:?}, {:?})", item.0, item.1);
        // TODO: ver de sacar este atomicbool
        if !self.aceptar_ordenes {
            println!("[UDP] Simulando local caido, no acepta orden");
            return;
        }
        match Orden::deserializar(item.0.iter().as_slice()) {
            Ok(orden) => {
                self.recipient_local
                    .try_send(AgregarOrden(orden, item.1))
                    .unwrap();
            }
            Err(_e) => {
                println!("[UDP] Mensaje no reconocido:");
                if self
                    .sink
                    .write(("MENSAJE NO RECONOCIDO".as_bytes().into(), item.1))
                    .is_ok()
                {
                } else {
                    eprintln!("[UDP - No se pudo enviar mensaje] MENSAJE NO RECONOCIDO");
                };
            }
        };
    }
}

impl actix::io::WriteHandler<io::Error> for UdpClientActor {}

#[derive(Message)]
#[rtype(result = "()")]
struct ResultadoAgregarOrden(Option<ErrorLocal>, SocketAddr);

/// Recibe el resultado de agregar una orden del actor Local y envia la respuesta
/// a la dirección del ecommerce que le envio la orden
impl Handler<ResultadoAgregarOrden> for UdpClientActor {
    type Result = ();

    fn handle(&mut self, msg: ResultadoAgregarOrden, _ctx: &mut Self::Context) -> Self::Result {
        match msg.0 {
            None => {
                println!("[UDP] Orden agregada");
                // TODO: puede ser que justo un ecommerce se caiga, en este caso simplemente no hacer nada
                if self
                    .sink
                    .write((
                        TipoDeMensaje::OrdenAceptada.value().as_bytes().into(),
                        msg.1,
                    ))
                    .is_ok()
                {
                } else {
                    // TODO: si pasa esto el ecommerce va a hacer timeout y enviar la misma orden a otro local, duplicandola
                    eprintln!("[UDP - Error] No se pudo enviar mensaje OrdenAceptada");
                }
            }
            Some(e) => {
                println!("[UDP - Error] {:?}", e);
                if self
                    .sink
                    .write((
                        TipoDeMensaje::from_error_local(&e)
                            .value()
                            .as_bytes()
                            .into(),
                        msg.1,
                    ))
                    .is_ok()
                {
                } else {
                    eprintln!("[UDP - Error] No se pudo enviar mensaje {:?}", e);
                }
            }
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct AceptarOrdenes(bool);

impl Handler<AceptarOrdenes> for UdpClientActor {
    type Result = ();

    fn handle(&mut self, msg: AceptarOrdenes, _ctx: &mut Self::Context) -> Self::Result {
        println!("[DEBUG] aceptar ordenes = {}", msg.0);
        self.aceptar_ordenes = msg.0;
    }
}

// Empieza ActorLocal
/// Es encargado de manejar el stock y ordenes del local, el resto de los actores
/// les envian mensajes con el tipo de acción a realizar: agregar una orden, entregarla/cancelarla
/// vender en local y les responde el resultado de la operación
struct ActorLocal {
    local: Local,
    recipient_recibir_ordenes: Recipient<ResultadoAgregarOrden>,
    recipient_vender_en_local: Recipient<ResultadoVenderEnLocal>,
    recipient_job_ordenes: Recipient<ResultadoEntregarOrden>,
}

impl Actor for ActorLocal {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "()")]
struct AgregarOrden(Orden, SocketAddr);

/// Agregar orden del ecommerce al local, devuelve resultado a UdpClientActor
impl Handler<AgregarOrden> for ActorLocal {
    type Result = ();

    fn handle(&mut self, msg: AgregarOrden, _ctx: &mut Self::Context) -> Self::Result {
        match self.local.agregar_orden(msg.0) {
            Ok(_) => {
                // TODO: DO NOT UNWRAP
                self.recipient_recibir_ordenes
                    .try_send(ResultadoAgregarOrden(None, msg.1))
                    .unwrap();
            }
            Err(e) => {
                // TODO: DO NOT UNWRAP
                self.recipient_recibir_ordenes
                    .try_send(ResultadoAgregarOrden(Some(e), msg.1))
                    .unwrap();
            }
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct VenderEnLocal(Orden);

/// Recibe orden y vende los productos del local, devuelve resultado a actor Vendedor
impl Handler<VenderEnLocal> for ActorLocal {
    type Result = ();
    fn handle(&mut self, msg: VenderEnLocal, _ctx: &mut Self::Context) -> Self::Result {
        match self.local.vender(msg.0) {
            Ok(_) => {
                println!("[Vendedor] se vende producto en local");
                self.recipient_vender_en_local
                    .try_send(ResultadoVenderEnLocal(None))
                    .unwrap();
            }
            Err(e) => {
                eprintln!("[Vendedor - Error] {}", e);
                self.recipient_vender_en_local
                    .try_send(ResultadoVenderEnLocal(Some(e)))
                    .unwrap();
            }
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct EntregarOrden;

/// Recibe un msg vacio, hace un ranom para determinar y una orden se entrega o cancela
/// y envia un msg a el JobOrdenes con el resultado
impl Handler<EntregarOrden> for ActorLocal {
    type Result = ();

    fn handle(&mut self, _msg: EntregarOrden, _ctx: &mut Self::Context) {
        if self.local.ordenes_en_progreso.is_empty() {
            println!("[Job] no hay ordenes en progreso");
            let _ = self
                .recipient_job_ordenes
                .try_send(ResultadoEntregarOrden(None));
            return;
        }
        let random: f64 = rand::random();
        let indice_maximo_ordenes = self.local.ordenes_en_progreso.len() - 1;
        let res_random = random > 0.5;
        let func = if res_random {
            Local::entregar_orden
        } else {
            Local::cancelar_orden
        };
        let respuesta = if res_random { "entregada" } else { "cancelada" };
        match func(&mut self.local, || {
            thread_rng().gen_range(0..=indice_maximo_ordenes)
        }) {
            Ok(_) => {
                println!("[Job] Orden {}", respuesta);
                // TODO: do not unwrap
                self.recipient_job_ordenes
                    .try_send(ResultadoEntregarOrden(None))
                    .unwrap();
            }
            Err(e) => {
                eprintln!("[Job - Error] {}", e);
                // TODO: do not unwrap
                self.recipient_job_ordenes
                    .try_send(ResultadoEntregarOrden(Some(e)))
                    .unwrap();
            }
        }
    }
}
// Termina ActorLocal

struct Vendedor {
    recipient_local: Recipient<VenderEnLocal>,
    ordenes_reader: Reader<File>,
}

impl Actor for Vendedor {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "()")]
struct VenderEnLocalVendedor;

/// Lee una orden y se la envia al actor Local para que entregue los productos
impl Handler<VenderEnLocalVendedor> for Vendedor {
    type Result = ();

    fn handle(&mut self, _msg: VenderEnLocalVendedor, _ctx: &mut Self::Context) {
        // Simulo que las entregas ocurren cada cierto tiempo
        let orden = leer_linea_csv(&mut self.ordenes_reader, Orden::from_record);
        match orden {
            Ok(orden) => {
                print!("[Vendedor] recibe pedido en local de {}", orden);
                // TODO: Do not Unwrap
                self.recipient_local.try_send(VenderEnLocal(orden)).unwrap()
            }
            Err(e) => {
                eprintln!("[Vendedor - Error] {}", e);
            }
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct ResultadoVenderEnLocal(Option<ErrorLocal>);

/// Recibe msg con el resutado de vender en local hace un sleep y se auto envia un nuevo msg
/// para vender otra orden. Simulando así un loop
impl Handler<ResultadoVenderEnLocal> for Vendedor {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, _msg: ResultadoVenderEnLocal, _ctx: &mut Self::Context) -> Self::Result {
        println!("[Vendedor] descansa un poco");
        Box::pin(sleep(Duration::from_millis(2000)).into_actor(self).map(
            move |_result, _me, ctx| {
                ctx.address().try_send(VenderEnLocalVendedor).unwrap();
            },
        ))
    }
}

struct JobOrdenes {
    recipient_local: Recipient<EntregarOrden>,
}

impl Actor for JobOrdenes {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "()")]
struct EntregarOrdenJobOrdenes;

impl Handler<EntregarOrdenJobOrdenes> for JobOrdenes {
    type Result = ();

    fn handle(&mut self, _msg: EntregarOrdenJobOrdenes, _ctx: &mut Self::Context) {
        // TODO: do not unwrap
        self.recipient_local.try_send(EntregarOrden).unwrap()
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct ResultadoEntregarOrden(Option<ErrorLocal>);

impl Handler<ResultadoEntregarOrden> for JobOrdenes {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, _msg: ResultadoEntregarOrden, _ctx: &mut Self::Context) -> Self::Result {
        println!("[Job] descansa un poco");
        Box::pin(sleep(Duration::from_millis(2000)).into_actor(self).map(
            move |_result, _me, ctx| {
                ctx.address().try_send(EntregarOrdenJobOrdenes).unwrap();
            },
        ))
    }
}

struct AceptadorOrdenes {
    recipient_recibir_ordenes: Recipient<AceptarOrdenes>,
}

impl Actor for AceptadorOrdenes {
    type Context = Context<Self>;
}

// Define a message for the actor
#[derive(Message)]
#[rtype(result = "()")]
struct ReadStdin;

/// Actor que lee del stdin, se debe ejecutar en un Arbiter distinto al resto para evitar bloquear
/// la ejecución
/// Acepta los siguientes valores
/// c: evita que UdpClientActor responda a los paquetes
/// l: vuelve a permitir la entrada de paquetes
impl Handler<ReadStdin> for AceptadorOrdenes {
    type Result = ();

    fn handle(&mut self, _msg: ReadStdin, _ctx: &mut Self::Context) {
        // Create a buffer to read lines from stdin
        let stdin = io::stdin();
        let reader = stdin.lock();

        // Iterate over lines and handle them
        for line in reader.lines() {
            match line {
                Ok(line) => {
                    if line == "c" {
                        println!("[UDP] Cerrando conexión");
                        // TODO: do not unwrap
                        let _ = self
                            .recipient_recibir_ordenes
                            .try_send(AceptarOrdenes(false));
                    }
                    if line == "l" {
                        println!("[UDP] Levantando conexión");
                        let _ = self
                            .recipient_recibir_ordenes
                            .try_send(AceptarOrdenes(true));
                    }
                }
                Err(error) => {
                    eprintln!("[UDP] Comando invalido: {}", error);
                }
            }
        }
    }
}

fn main() {
    let mut args = env::args().skip(1);
    let id = args
        .next()
        .expect("Falta parametro del id")
        .parse::<usize>()
        .expect("No es un numero");

    let dir_stock = format!(
        "{}{}{}{}",
        env!("CARGO_MANIFEST_DIR"),
        "/data/stock_local_",
        id.clone(),
        ".txt"
    );

    let local = instanciar_local(&dir_stock).expect("Error al instanciar local");

    let dir_ordenes = format!(
        "{}{}{}{}",
        env!("CARGO_MANIFEST_DIR"),
        "/data/ordenes_local_",
        id.clone(),
        ".txt"
    );

    let ordenes_reader = open_csv(&dir_ordenes).expect("Error al abrir csv");

    let system = System::new();

    let arbiter_1 = Arbiter::new();

    let future = async move {
        let address = id_to_addr_local(id).parse::<SocketAddr>().unwrap();
        let socket = UdpSocket::bind(&address).await.unwrap();

        let mut addr_udp_ext: Option<Addr<UdpClientActor>> = None;

        ActorLocal::create(|ctx| {
            let addr_local = ctx.address();

            let addr_udp = UdpClientActor::start(socket, addr_local.clone().recipient());
            let addr_vendedor = Vendedor {
                recipient_local: addr_local.clone().recipient(),
                ordenes_reader,
            }
            .start();
            let addr_job_ordenes = JobOrdenes {
                recipient_local: addr_local.clone().recipient(),
            }
            .start();

            addr_vendedor.do_send(VenderEnLocalVendedor);
            addr_job_ordenes.do_send(EntregarOrdenJobOrdenes);

            addr_udp_ext = Option::from(addr_udp.clone());

            ActorLocal {
                local,
                recipient_recibir_ordenes: addr_udp.recipient(),
                recipient_vender_en_local: addr_vendedor.recipient(),
                recipient_job_ordenes: addr_job_ordenes.recipient(),
            }
        });

        let arbiter_2 = Arbiter::new();
        arbiter_2.spawn(async move {
            let addr = AceptadorOrdenes {
                recipient_recibir_ordenes: addr_udp_ext.unwrap().recipient(),
            }
            .start();
            addr.do_send(ReadStdin);
        });
    };

    arbiter_1.spawn(future);

    system.run().unwrap();
}

fn instanciar_local(dir_archivo: &str) -> Result<Local, io::Error> {
    let mut reader = open_csv(dir_archivo)?;
    let mut productos: Productos = HashMap::new();
    while let Ok(producto) = leer_linea_csv(&mut reader, StockProducto::from_record) {
        productos.insert(producto.id_producto, producto);
    }

    Ok(Local::new(productos))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::lector_csv::test_util::crear_archivo;

    #[test]
    fn test_instanciar_local_leyendo_stock() {
        let dir_archivo_test = "test_local.csv";
        let mut wtr = crear_archivo(dir_archivo_test);
        wtr.write_record(["id_producto", "stock"]).unwrap();
        wtr.write_record(["1", "30"]).unwrap();
        wtr.flush().unwrap();

        let local = instanciar_local(dir_archivo_test);

        std::fs::remove_file(dir_archivo_test).unwrap();

        assert!(local.is_ok());

        let local = local.unwrap();
        assert_eq!(local.productos_en_stock.len(), 1);
    }
}
