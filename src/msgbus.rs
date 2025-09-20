use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub enum Msg {
    Pkt(PktMsg),
    CreateGame(CreateGameMsg),
    GameSchedule(GameScheduleMsg),
    NewTurn(NewTurnMsg),
    TurnHostStart(TurnHostStartMsg),
    OrdersSubmitted(OrdersSubmittedMsg),
    MapChanged(MapChangedMsg),
    ClientDisc(ClientDiscMsg),
    ModsChanged(ModsChangedMsg),
    EraChanged(EraChangedMsg),
    GameArchived(GameArchivedMsg),
    EmailConfigCreated(EmailConfigCreatedMsg),
    EmailConfigDeleted(EmailConfigDeletedMsg),
    DiscordConfigCreated(DiscordConfigCreatedMsg),
    DiscordConfigDeleted(DiscordConfigDeletedMsg),
    LastActivityUpdated(LastActivityUpdatedMsg)
}

#[derive(Clone)]
pub struct ClientDiscMsg {
    pub addr: std::net::SocketAddr,
}

#[derive(Clone)]
pub struct PktMsg {
    pub addr: std::net::SocketAddr,
    pub pkt: crate::packets::Body,
}
#[derive(Clone)]
pub struct GameScheduleMsg {
    pub game_id: i32,
    pub schedule: std::time::SystemTime,
}

#[derive(Clone)]
pub struct CreateGameMsg {
    pub game_id: i32,
}

#[derive(Clone)]
pub struct GameArchivedMsg {
    pub game_id: i32,
}

#[derive(Clone)]
pub struct OrdersSubmittedMsg {
    pub game_id: i32,
    pub nation_id: i32,
    pub turn_id: i32,
}

#[derive(Clone)]
pub struct TurnHostStartMsg {
    pub game_id: i32,
    pub turn_number: i32,
}

#[derive(Clone)]
pub struct MapChangedMsg {
    pub game_id: i32,
    pub map_id: i32,
}

#[derive(Clone)]
pub struct ModsChangedMsg {
    pub game_id: i32,
}

#[derive(Clone)]
pub struct EmailConfigCreatedMsg {
    pub id: i32,
}

#[derive(Clone)]
pub struct EmailConfigDeletedMsg {
    pub id: i32,
}

#[derive(Clone)]
pub struct DiscordConfigCreatedMsg {
    pub id: i32,
}
#[derive(Clone)]
pub struct DiscordConfigDeletedMsg {
    pub id: i32,
}

#[derive(Clone)]
pub struct EraChangedMsg {
    pub game_id: i32,
    pub new_era: i32,
}

#[derive(Clone)]
pub struct LastActivityUpdatedMsg {
    pub game_id: i32,
    pub last_activity: std::time::SystemTime
}

#[derive(Clone)]
pub struct NewTurnMsg {
    pub game_id: i32,
    pub turn_number: i32,
}

pub struct MsgBus {
    broadcasts: Arc<Mutex<Vec<crossbeam_channel::Sender<Msg>>>>,
    pub sender: crossbeam_channel::Sender<Msg>,
}

impl MsgBus {
    pub fn new() -> MsgBus {
        let (tx, rx) = crossbeam_channel::unbounded::<Msg>();
        let broadcasts: Arc<Mutex<Vec<crossbeam_channel::Sender<Msg>>>> =
            Arc::new(Mutex::new(vec![]));
        let cloned_broadcast = broadcasts.clone();
        std::thread::spawn(move || loop {
            let m = rx
                .recv()
                .expect("Msgbus failed to receive broadcast request (send channel hosed?");
            if let Ok(mut broadcasters) = cloned_broadcast.lock() {
                broadcasters.extract_if(.., |broadcaster| !broadcaster.send(m.clone()).is_ok()).for_each(drop);
            }
        });
        MsgBus {
            broadcasts,
            sender: tx,
        }
    }
    pub fn new_recv(&self) -> MsgBusRx {
        let (tx, rx) = crossbeam_channel::unbounded::<Msg>();
        if let Ok(mut tx_vec) = (*self.broadcasts).lock() {
            tx_vec.push(tx);
            drop(tx_vec);
            MsgBusRx {
                rx,
                broadcasts: self.broadcasts.clone(),
            }
        } else {
            panic!("Fail case not implemented");
        }
    }

    pub fn send(&self, msg: Msg) {
        self.sender.send(msg).unwrap();
    }
}

pub struct MsgBusRx {
    rx: crossbeam_channel::Receiver<Msg>,
    broadcasts: Arc<Mutex<Vec<crossbeam_channel::Sender<Msg>>>>,
}

impl Clone for MsgBusRx {
    fn clone(&self) -> Self {
        self.new_recv()
    }
}

impl MsgBusRx {
    pub fn new_recv(&self) -> MsgBusRx {
        let (tx, rx) = crossbeam_channel::unbounded::<Msg>();
        if let Ok(mut tx_vec) = (*self.broadcasts).lock() {
            tx_vec.push(tx);
            drop(tx_vec);
            MsgBusRx {
                rx,
                broadcasts: self.broadcasts.clone(),
            }
        } else {
            panic!("Fail case not implemented");
        }
    }
    pub fn recv(&self) -> Result<Msg, crossbeam_channel::RecvError> {
        self.rx.recv()
    }
    pub fn recv_timeout(
        &self,
        timeout: std::time::Duration,
    ) -> Result<Msg, crossbeam_channel::RecvTimeoutError> {
        if timeout == std::time::Duration::from_nanos(0) {
            self.rx
                .recv_timeout(std::time::Duration::from_secs(999_999_999))
        } else {
            self.rx.recv_timeout(timeout)
        }
    }
}

pub type MsgBusTx = crossbeam_channel::Sender<Msg>;
