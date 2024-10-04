use async_channel::{Receiver, Sender, TryRecvError};
use async_net::{TcpListener, TcpStream};
use bevy::{
    asset::{AsyncReadExt, AsyncWriteExt},
    prelude::*,
    tasks::{futures_lite::StreamExt, IoTaskPool, Task},
};
use libmudtelnet::{
    bytes::{Bytes, BytesMut},
    compatibility::CompatibilityTable,
    events::{TelnetEvents, TelnetIAC},
    telnet::op_command,
};
use libmudtelnet::{telnet::op_option, Parser as TelnetParser};

pub struct TelnetPlugin;

impl Plugin for TelnetPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, startup);
        app.add_systems(PreUpdate, (connection_handler, data_handler));
        app.add_systems(PostUpdate, data_sender);
        app.add_event::<NewConnection>();
        app.add_event::<MessageReceived>();
        app.add_event::<SendMessage>();
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

#[derive(Event)]
pub struct MessageReceived {
    pub connection: Entity,
    pub data: Bytes,
}

#[derive(Event)]
pub struct SendMessage {
    pub connection: Entity,
    pub data: TelnetEvents,
}

pub trait EventWriterTelnetEx {
    fn print(&mut self, conn: Entity, text: &str);

    fn println(&mut self, conn: Entity, text: &str) {
        self.print(conn, text);
        self.print(conn, "\r\n");
    }

    fn ga(&mut self, conn: Entity);
}

impl<'w> EventWriterTelnetEx for EventWriter<'w, SendMessage> {
    fn print(&mut self, conn: Entity, text: &str) {
        self.send(SendMessage {
            connection: conn,
            data: TelnetEvents::DataSend(BytesMut::from(text).freeze()),
        });
    }

    fn ga(&mut self, conn: Entity) {
        self.send(SendMessage {
            connection: conn,
            data: TelnetEvents::IAC(TelnetIAC {
                command: op_command::GA,
            }),
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
    let listener = TcpListener::bind("127.0.0.1:2222")
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
            TelnetEvents::IAC(_) => println!("IAC"),
            TelnetEvents::Negotiation(negotioation) => println!("Negotiation: {:?}", negotioation),
            TelnetEvents::Subnegotiation(_) => println!("Subnegotiation"),
            TelnetEvents::DataReceive(data) => {
                println!("Data received: {:?}", data);
                event_tx
                    .send(TelnetEvent::MessageReceived(data))
                    .await
                    .expect("todo");
            }
            TelnetEvents::DataSend(data) => {
                println!("Sending data");
                socket.write_all(&data).await.expect("todo");
            }
            TelnetEvents::DecompressImmediate(_) => println!("Decompress"),
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

        new_connection_event.send(NewConnection {
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
                        println!("Could not send telnet event");
                        todo!()
                    }
                }
            }
            Err(TryRecvError::Closed) => {
                // Connection closed
                if let Some(mut ent) = commands.get_entity(entity) {
                    ent.despawn();
                }
            }
            Err(TryRecvError::Empty) => {
                // No data
            }
        }

        match connection.telnet_event_receiver.try_recv() {
            Ok(TelnetEvent::MessageReceived(data)) => {
                message_event.send(MessageReceived {
                    connection: entity,
                    data,
                });
            }
            Err(TryRecvError::Closed) => {
                // Connection closed
                if let Some(mut ent) = commands.get_entity(entity) {
                    ent.despawn();
                }
            }
            Err(TryRecvError::Empty) => {}
        }
    }
}

fn data_sender(mut events: EventReader<SendMessage>, query: Query<&Connection>) {
    for event in events.read() {
        if let Ok(ent) = query.get(event.connection) {
            let _ = ent.telnet_event_sender.try_send(event.data.clone());
        }
    }
}
