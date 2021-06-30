use std::net::SocketAddr;

use iced::{Application, Clipboard, Command, Text};

fn main() {
    TCPDemo::run(iced::Settings::default());
}

#[derive(Debug)]
enum Message_E {
    ListenerStatus(tcp::ListenerStatus_E),
    ReceiverStatus(tcp::ReceiverStatus_E),
    None,
}

struct TCPDemo {
    // Holds new connections
    new_connections: Vec<(SocketAddr, tokio::net::TcpStream)>,

    // Holds the IDs of the already established connections
    //     Lets assume only 1 peer per SocketAddr for now so there's no collision in the results of Recipe::hash()
    connections: Vec<SocketAddr>,
}

impl Application for TCPDemo {
    type Flags = ();
    type Executor = iced::executor::Default;
    type Message = Message_E;

    fn new(_flags: ()) -> (TCPDemo, Command<Message_E>) {
        (
            TCPDemo {
                new_connections: Vec::new(),
                connections: Vec::new(),
            },
            Command::none(),
        )
    }

    fn update(&mut self, message: Self::Message, _clipboard: &mut Clipboard) -> Command<Message_E> {
        match message {
            Message_E::ListenerStatus(status) => match status {
                tcp::ListenerStatus_E::Accepted { addr, stream } => {
                    self.new_connections.push((addr, stream));
                }
                _ => (),
            },
            Message_E::ReceiverStatus(status) => match status {
                tcp::ReceiverStatus_E::Ready(addr) => self.connections.push(addr),
                tcp::ReceiverStatus_E::Message(_) => {
                    // Process message
                }
            },
            Message_E::None => (),
        }

        Command::none()
    }

    fn view(&mut self) -> iced::Element<'_, Self::Message> {
        Text::new("TCP Demo").into()
    }

    fn title(&self) -> String {
        "TCP Demo".to_string()
    }

    fn subscription(&self) -> iced::Subscription<Message_E> {
        let connection_listener_it = std::iter::once(
            iced::Subscription::from_recipe(tcp::TcpListenerRecipe {
                addr: SocketAddr::from(([127, 0, 0, 1], 8000)),
            })
            .map(Message_E::ListenerStatus),
        );
        let curr_connections_it = self.connections.iter().copied().map(|addr| {
            iced::Subscription::from_recipe(tcp::TcpReceiverStubRecipe { addr })
                .map(|_| Message_E::None)
        });

        // Cannot mutate self so I cannot "take" the TcpStreams from Application
        //     and pass it to the TcpReceiverRecipe.
        let new_connections_it = self.new_connections.drain().map(|(addr, stream)| {
            iced::Subscription::from_recipe(tcp::TcpReceiverRecipe { addr, stream })
                .map(Message_E::ReceiverStatus)
        });

        iced_futures::subscription::Subscription::batch(
            connection_listener_it
                .chain(curr_connections_it)
                .chain(new_connections_it)
                .collect::<Vec<iced::Subscription<Message_E>>>(),
        )
    }
}

mod tcp {
    use iced_futures::futures;
    use std::{
        hash::{Hash, Hasher},
        net::SocketAddr,
    };
    use tokio::net::{TcpListener, TcpStream};

    pub struct TcpListenerRecipe {
        pub addr: SocketAddr,
    }

    pub struct TcpReceiverRecipe {
        pub addr: SocketAddr,
        pub stream: TcpStream,
    }
    // Stub recipe so the Recipe subscription ID matches the corresponding subscription created via TcpReceiverRecipe
    pub struct TcpReceiverStubRecipe {
        pub addr: SocketAddr,
    }

    #[derive(Debug)]
    pub enum ListenerStatus_E {
        Accepted { stream: TcpStream, addr: SocketAddr },
        Error(std::io::Error),
    }

    #[derive(Debug)]
    pub enum ReceiverStatus_E {
        Ready(SocketAddr),
        Message(Vec<u8>),
    }

    impl<H, I> iced_native::subscription::Recipe<H, I> for TcpListenerRecipe
    where
        H: Hasher,
    {
        type Output = ListenerStatus_E;

        fn hash(&self, state: &mut H) {
            self.addr.hash(state);
        }

        fn stream(
            self: Box<Self>,
            _input: iced_futures::BoxStream<I>,
        ) -> iced_futures::BoxStream<Self::Output> {
            use futures::stream::StreamExt;

            let std_listener = std::net::TcpListener::bind(self.addr).expect("Failed to bind");
            std_listener
                .set_nonblocking(true)
                .expect("Failed to set nonblocking");
            let listener = TcpListener::from_std(std_listener).unwrap();

            futures::stream::unfold(listener, |listener| async move {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        Some((ListenerStatus_E::Accepted { stream, addr }, listener))
                    }
                    Err(err) => Some((ListenerStatus_E::Error(err), listener)),
                }
            })
            .boxed()
        }
    }

    impl<H, I> iced_native::subscription::Recipe<H, I> for TcpReceiverRecipe
    where
        H: Hasher,
    {
        type Output = ReceiverStatus_E;

        fn hash(&self, state: &mut H) {
            self.addr.hash(state);
        }

        fn stream(
            self: Box<Self>,
            _input: iced_futures::BoxStream<I>,
        ) -> iced_futures::BoxStream<Self::Output> {
            // I need mutable access to the TcpStream here

            todo!()
        }
    }

    impl<H, I> iced_native::subscription::Recipe<H, I> for TcpReceiverStubRecipe
    where
        H: Hasher,
    {
        type Output = ();

        fn hash(&self, state: &mut H) {
            self.addr.hash(state);
        }

        fn stream(
            self: Box<Self>,
            input: iced_futures::BoxStream<I>,
        ) -> iced_futures::BoxStream<Self::Output> {
            use futures::StreamExt;

            futures::stream::empty().boxed()
        }
    }
}
