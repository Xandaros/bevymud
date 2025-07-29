use async_channel::{Receiver, Sender, TryRecvError};
use async_net::{TcpListener, TcpStream};
use bevy::{
    asset::{AsyncReadExt, AsyncWriteExt},
    prelude::*,
    tasks::{IoTaskPool, Task, futures_lite::StreamExt},
};
use libmudtelnet::{Parser as TelnetParser, telnet::op_option};
use libmudtelnet::{
    bytes::{Bytes, BytesMut},
    compatibility::CompatibilityTable,
    events::{TelnetEvents, TelnetNegotiation},
    telnet::op_command,
};

pub struct TelnetPlugin;

impl Plugin for TelnetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup);
        app.add_systems(PreUpdate, (connection_handler, data_handler));
        app.add_systems(PostUpdate, data_sender);
        app.add_event::<NewConnection>();
        app.add_event::<MessageReceived>();
        app.add_event::<SendMessageAction>();
    }
}

#[derive(Resource)]
struct Channel {
    receiver: Receiver<TcpStream>,
}

#[derive(Component)]
pub struct Connection {
    _reader_task: Task<()>,
    _event_handler: Task<()>,
    data_receiver: Receiver<Bytes>,
    pub telnet_event_sender: Sender<TelnetEvents>,
    telnet_event_receiver: Receiver<TelnetEvent>,
    pub parser: TelnetParser,
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("_reader_task", &self._reader_task)
            .field("_event_handler", &self._event_handler)
            .field("data_receiver", &self.data_receiver)
            .field("telnet_event_sender", &self.telnet_event_sender)
            .field("telnet_event_receiver", &self.telnet_event_receiver)
            .finish()
    }
}

#[derive(Event, Clone)]
pub struct MessageReceived {
    pub connection: Entity,
    pub data: Bytes,
}

impl MessageReceived {
    pub fn to_text(&self) -> String {
        let mut ret = String::from_utf8_lossy(&self.data).into_owned();
        if ret.ends_with("\r\n") {
            ret.truncate(ret.len().saturating_sub(2));
        } else if ret.ends_with("\r") || ret.ends_with("\n") {
            ret.truncate(ret.len().saturating_sub(1));
        }
        ret
    }
}

#[derive(Event, Clone)]
pub struct SendMessageAction {
    pub connection: Entity,
    pub data: TelnetEvents,
}

pub trait EventWriterTelnetEx {
    fn send_message(&mut self, conn: Entity, events: TelnetEvents);

    fn print(&mut self, conn: Entity, text: &str) {
        self.send_message(
            conn,
            TelnetEvents::DataSend(libmudtelnet::Parser::escape_iac(format!("{text}"))),
        );
    }

    fn println(&mut self, conn: Entity, text: &str) {
        self.send_message(
            conn,
            TelnetEvents::DataSend(libmudtelnet::Parser::escape_iac(format!("{text}\r\n"))),
        );
    }

    fn ga(&mut self, conn: Entity) {
        self.send_message(
            conn,
            TelnetEvents::DataSend(Bytes::copy_from_slice(&[
                libmudtelnet::telnet::op_command::IAC,
                libmudtelnet::telnet::op_command::GA,
            ])),
        );
    }

    fn echo(&mut self, conn: Entity, echo: bool) {
        let command = if echo {
            op_command::WONT
        } else {
            op_command::WILL
        };

        self.send_message(
            conn,
            TelnetEvents::Negotiation(TelnetNegotiation {
                command,
                option: op_option::ECHO,
            }),
        );
    }
}

impl<'w> EventWriterTelnetEx for EventWriter<'w, SendMessageAction> {
    fn send_message(&mut self, conn: Entity, events: TelnetEvents) {
        self.write(SendMessageAction {
            connection: conn,
            data: events,
        });
    }
}

enum TelnetEvent {
    MessageReceived(Bytes),
}

#[derive(Event)]
pub struct NewConnection {
    pub entity: Entity,
}

/// Listens to incoming connections and sends the stream to `sender`
async fn connection_listener(sender: Sender<TcpStream>) {
    let listener = TcpListener::bind("0.0.0.0:2222")
        .await
        .expect("Could not open socket");
    let mut incoming = listener.incoming();

    while let Some(conn) = incoming.next().await {
        if let Ok(stream) = conn {
            sender.send(stream).await.expect("Channel closed");
        }
    }
}

fn startup(mut commands: Commands) {
    let (tx, rx) = async_channel::unbounded();

    IoTaskPool::get().spawn(connection_listener(tx)).detach();

    commands.insert_resource(Channel { receiver: rx });
}

/// Reads data from `stream` and sends the read bytes to `sender`
async fn connection_reader(mut stream: TcpStream, sender: Sender<Bytes>) {
    let mut buf = [0; 1024];

    loop {
        if let Ok(n) = stream.read(&mut buf).await {
            if n == 0 {
                // Connection closed
                sender.close();
                return;
            } else {
                let data = BytesMut::from(&buf[0..n]);
                if sender.send(data.freeze()).await.is_err() {
                    // Channel closed?
                    let _ = stream.write(b"Internal Error: TCP Channel closed.").await;
                    let _ = stream.close().await;
                    return;
                }
            }
        }
    }
}

async fn telnet_event_handler(
    event_rx: Receiver<TelnetEvents>,
    mut socket: TcpStream,
    event_tx: Sender<TelnetEvent>,
) {
    while let Ok(event) = event_rx.recv().await {
        match event {
            TelnetEvents::IAC(_) => debug!("IAC"),
            TelnetEvents::Negotiation(negotioation) => debug!("Negotiation: {:?}", negotioation),
            TelnetEvents::Subnegotiation(_) => debug!("Subnegotiation"),
            TelnetEvents::DataReceive(data) => {
                trace!("Data received: {:?}", data);
                event_tx
                    .send(TelnetEvent::MessageReceived(data))
                    .await
                    .expect("todo");
            }
            TelnetEvents::DataSend(data) => {
                trace!("Sending data");
                socket.write_all(&data).await.expect("todo");
            }
            TelnetEvents::DecompressImmediate(_) => debug!("Decompress"),
        }
    }
}

fn connection_handler(
    mut commands: Commands,
    channel: Res<Channel>,
    mut new_connection_event: EventWriter<NewConnection>,
) {
    while let Ok(stream) = channel.receiver.try_recv() {
        let (tcp_sender, tcp_receiver) = async_channel::unbounded(); // Raw received data

        // Outgoing telnet events
        let (telnet_out_sender, telnet_out_receiver) = async_channel::unbounded();
        // Incoming telnet events
        let (telnet_in_sender, telnet_in_receiver) = async_channel::unbounded();

        let reader_task = {
            let stream = stream.clone();
            IoTaskPool::get().spawn(async move { connection_reader(stream, tcp_sender).await })
        };

        let event_handler = {
            let stream = stream.clone();
            IoTaskPool::get().spawn(async move {
                telnet_event_handler(telnet_out_receiver, stream, telnet_in_sender).await
            })
        };

        let parser = TelnetParser::with_support({
            let mut table = CompatibilityTable::new();
            table.support(op_option::ECHO);
            table
        });

        let entity = commands.spawn(Connection {
            _reader_task: reader_task,
            _event_handler: event_handler,
            data_receiver: tcp_receiver,
            telnet_event_sender: telnet_out_sender,
            telnet_event_receiver: telnet_in_receiver,
            parser,
        });

        new_connection_event.write(NewConnection {
            entity: entity.id(),
        });
    }
}

/// Receives raw socket data, parses it, and sends possible send events back out
/// Also retrieves incoming telnet events and emits corresponding events
fn data_handler(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Connection)>,
    mut message_event: EventWriter<MessageReceived>,
) {
    for (entity, mut connection) in &mut query {
        match connection.data_receiver.try_recv() {
            Ok(data) => {
                let events = connection.parser.receive(&data);
                for event in events {
                    if connection.telnet_event_sender.try_send(event).is_err() {
                        error!("Could not send telnet event");
                        todo!()
                    }
                }
            }
            Err(TryRecvError::Closed) => {
                // Connection closed
                if let Ok(mut ent) = commands.get_entity(entity) {
                    ent.despawn();
                }
            }
            Err(TryRecvError::Empty) => {
                // No data
            }
        }

        match connection.telnet_event_receiver.try_recv() {
            Ok(TelnetEvent::MessageReceived(data)) => {
                let event = MessageReceived {
                    connection: entity,
                    data,
                };
                message_event.write(event.clone());
                commands.trigger_targets(event, entity);
            }
            Err(TryRecvError::Closed) => {
                // Connection closed
                if let Ok(mut ent) = commands.get_entity(entity) {
                    ent.despawn();
                }
            }
            Err(TryRecvError::Empty) => {}
        }
    }
}

fn data_sender(mut events: EventReader<SendMessageAction>, mut query: Query<&mut Connection>) {
    for event in events.read() {
        if let Ok(mut conn) = query.get_mut(event.connection) {
            if let TelnetEvents::Negotiation(TelnetNegotiation { command, option }) = event.data {
                let data = match command {
                    op_command::WILL => conn.parser._will(option),
                    op_command::WONT => conn.parser._wont(option),
                    op_command::DO => conn.parser._do(option),
                    op_command::DONT => conn.parser._dont(option),
                    _ => None,
                };
                if let Some(data) = data {
                    let _ = conn.telnet_event_sender.try_send(data);
                }
            } else {
                let _ = conn.telnet_event_sender.try_send(event.data.clone());
            }
        }
    }
}
