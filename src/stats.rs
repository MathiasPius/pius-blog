use std::collections::HashMap;
use std::time::{Duration, Instant};
use actix::prelude::*;
use actix_web_actors::ws;
use actix_web::{HttpRequest, HttpResponse, Error, web::{Payload, Data}};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

/*
fn system_stats(_tera: Data<Tera>) -> Result<HttpResponse, BlogError>
{
    let sys = systemstat::System::new();
    let mem = sys.memory().map(|mem| format!("{}/{}", saturating_sub_bytes(mem.total, mem.free), mem.total))?;
    let cpu = sys.load_average().map(|cpu| format!("{}, {}, {}", cpu.one, cpu.five, cpu.fifteen))?;

    Ok(HttpResponse::Ok().content_type("text/html").body(format!("{}<br />{}", mem, cpu)))
}
*/

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

#[derive(Message)]
pub struct Update(pub String);

#[derive(Message)]
pub struct StartCollecting;

pub struct Connect {
    pub addr: Recipient<Update>
}

impl Message for Connect {
    type Result = usize;
}

#[derive(Message)]
pub struct Disconnect {
    pub id: usize
}

#[derive(Default)]
pub struct StatisticsServer {
    sessions: HashMap<usize, Recipient<Update>>,
    counter: usize
}

impl Actor for StatisticsServer {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.run_interval(Duration::from_secs(1), move |_, ctx|
            ctx.address().do_send(StartCollecting {})
        );
    }
}

impl Handler<StartCollecting> for StatisticsServer {
    type Result = ();

    fn handle(&mut self, _: StartCollecting, _: &mut Context<Self>) {
        for session in self.sessions.values() {
            session.do_send(Update("hello world!".into())).unwrap();            
        }
    }
}

impl Handler<Connect> for StatisticsServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        println!("Someone connected!");
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

impl Handler<Update> for StatsSession {
    type Result = ();

    fn handle(&mut self, msg: Update, ctx: &mut Self::Context) {
        ctx.text(msg.0)
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
            ws::Message::Text(_) => println!("Unexpected text"), 
            ws::Message::Binary(_) => println!("Unexpected binary"),
            ws::Message::Close(_) => {
                ctx.stop();
            }
            ws::Message::Nop => (),
        }
    }
}