use std::collections::HashMap;
use std::time::{Duration, Instant};
use actix::prelude::*;
use actix_web_actors::ws;
use actix_web::{HttpRequest, HttpResponse, Error, web::{Payload, Data}};
use systemstat::{System, Platform, saturating_sub_bytes};
use circular_queue::CircularQueue;
use crate::error::BlogError;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

/// Entry point for our route
pub fn system_stats(req: HttpRequest, stream: Payload, srv: Data<Addr<StatisticsServer>>) 
    -> Result<HttpResponse, Error> 
{
    ws::start(
        StatsSession {
            id: 0,
            hb: Instant::now(),
            addr: srv.get_ref().clone(),
        },
        &req,
        stream,
    )
}

#[derive(Serialize, Clone, Copy)]
pub struct MemoryMeasurement(u64);
#[derive(Serialize, Clone, Copy)]
pub struct CPUMeasurement(f32);

#[derive(Serialize)]
pub struct Update {
    max_memory: MemoryMeasurement,
    memory_used: MemoryMeasurement,
    load_average: CPUMeasurement
}

#[derive(Serialize)]
pub struct InitialValues {
    max_memory: MemoryMeasurement,
    memory_used: Vec<MemoryMeasurement>,
    load_average: Vec<CPUMeasurement>
}

#[derive(Message, Serialize)]
pub enum PushMessage {
    Update(Update)
}

pub struct GetInitialValues;

impl Message for GetInitialValues {
    type Result = Result<InitialValues, ()>;
}

pub struct CollectData;

impl Message for CollectData {
    type Result = Result<(), BlogError>;
}

pub struct Connect {
    pub addr: Recipient<PushMessage>
}

impl Message for Connect {
    type Result = usize;
}

#[derive(Message)]
pub struct Disconnect {
    pub id: usize
}

pub struct StatisticsServer {
    sessions: HashMap<usize, Recipient<PushMessage>>,
    system: System,
    max_memory: MemoryMeasurement,
    memory: CircularQueue<MemoryMeasurement>,
    cpu: CircularQueue<CPUMeasurement>,
    counter: usize
}

impl Default for StatisticsServer {
    fn default() -> Self {
        StatisticsServer {
            sessions: HashMap::default(),
            system: System::new(),
            max_memory: MemoryMeasurement(0),
            memory: CircularQueue::with_capacity(100),
            cpu: CircularQueue::with_capacity(100),
            counter: 0
        }
    }
}

impl Actor for StatisticsServer {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(Duration::from_secs(1), move |_, ctx|
            ctx.address().do_send(CollectData {})
        );
    }
}

impl Handler<CollectData> for StatisticsServer {
    type Result = Result<(), BlogError>;

    fn handle(&mut self, _: CollectData, _: &mut Context<Self>) -> Self::Result {
        let mem = self.system.memory()?;
            
        let memory_used = MemoryMeasurement(
            saturating_sub_bytes(mem.total, mem.free).as_u64()
        );

        let load_average = CPUMeasurement(
            self.system.load_average().map(|cpu| cpu.one)?
        );

        self.cpu.push(load_average);
        self.memory.push(memory_used);
        self.max_memory = MemoryMeasurement(mem.total.as_u64());

        for session in self.sessions.values() {
            session.do_send(PushMessage::Update(Update {
                max_memory: self.max_memory,
                memory_used,
                load_average
            })).unwrap();            
        }

        Ok(())
    }
}

impl Handler<GetInitialValues> for StatisticsServer {
    type Result = Result<InitialValues, ()>;

    fn handle(&mut self, _: GetInitialValues, _: &mut Context<Self>) -> Self::Result {
        Ok(InitialValues {
            max_memory: self.max_memory,
            memory_used: self.memory.iter().cloned().collect(),
            load_average: self.cpu.iter().cloned().collect()
        })
    }
}

impl Handler<Connect> for StatisticsServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        let id = self.counter;
        self.counter = self.counter + 1;
        self.sessions.insert(id, msg.addr);
        id
    }
}

impl Handler<Disconnect> for StatisticsServer {
    type Result = ();
    
    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        self.sessions.remove(&msg.id);
    }
}

pub struct StatsSession {
    id: usize,
    hb: Instant,
    addr: Addr<StatisticsServer>
}

impl StatsSession {
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                println!("Websocket Client heartbeat failed, disconnecting!");
                act.addr.do_send(Disconnect { id: act.id });
                ctx.stop();
                return;
            }

            ctx.ping("");
        });
    }
}


impl Actor for StatsSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);

        let addr = ctx.address();
        self.addr
            .send(Connect {
                addr: addr.recipient(),
            })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(res) => act.id = res,
                    _ => ctx.stop(),
                }
                fut::ok(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        self.addr.do_send(Disconnect { id: self.id });
        Running::Stop
    }
}

impl Handler<PushMessage> for StatsSession {
    type Result = ();

    fn handle(&mut self, msg: PushMessage, ctx: &mut Self::Context) {
        ctx.text(serde_json::to_string(&msg).unwrap())
    }
}

impl StreamHandler<ws::Message, ws::ProtocolError> for StatsSession {
    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
        match msg {
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.hb = Instant::now();
            }
            ws::Message::Nop => (),
            _ => ctx.stop()
        }
    }
}