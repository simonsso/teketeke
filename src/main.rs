//extern crate futures;
//extern crate hyper;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use futures::{future, Future, Stream};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
// use hyper::{Body, Error, Method, Request, Response, Server, StatusCode};

use hyper::service::service_fn;
use lazy_static::lazy_static;

// added from example
//use futures::{future, /*task,*/ Future, Stream};
use regex::Regex;

// use std::{thread, time};

// use future::{err, ok};
// use futures::future::lazy;
use futures::task::spawn;
use futures_locks::RwLock;
//use tokio::timer::Interval;
use hyper::error::Error;

#[derive(Copy, Clone)]
enum States {
    ETA(u32),
    DONE,
    REMOVED,
    EMPTY,
}

struct Record {
    id: u32,
    state: States,
}

struct Datastore {
    vault: Vec<RwLock<Record>>,
}

// #[derive(Serialize)]
// struct TableRequest{
//     itemname:String,
//     qty:i32,
// }

#[derive(Deserialize, Clone, Serialize)]
struct TableRequestVec {
    tab: Vec<TableRequest>,
}

#[derive(Deserialize, Clone, Serialize)]
#[serde(tag = "order", content = "parameters", rename_all = "lowercase")]
enum TableRequest {
    order { itemname: String, qty: i32 },
}

impl std::fmt::Debug for TableRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TableRequest::order { itemname, qty } => write!(f, "{} {}", itemname, qty),
            _ => write! {f,""},
        }
    }
}
fn DatastoreRwLock() -> RwLock<Datastore> {
    let v: Vec<RwLock<Record>> = Vec::with_capacity(100);
    let d: Datastore = Datastore { vault: v };
    RwLock::new(d)
}

lazy_static! {
    // TODO verify the correctness of regexp in tests
    static ref RE_TABLE_NUM: Regex = Regex::new(r"^/table/(\d+)(/.*)?$").unwrap();
    static ref STORAGE:RwLock<Datastore> = DatastoreRwLock();
}

// Encapsulate response for hyper
fn microservice_handler(
    req: Request<Body>,
) -> Box<Future<Item = Response<Body>, Error = Error> + Send> {
    let uri: String = req.uri().to_string();
    let method = req.method().to_string();

    let (table, path): (Option<u32>, Option<String>) = match RE_TABLE_NUM.captures(&uri) {
        Some(m) => {
            // this is checked to be an integer
            let tbl = m.get(1).unwrap().as_str().parse::<u32>().unwrap();
            match m.get(2) {
                Some(argument) => (Some(tbl), Some(argument.as_str().to_string())),
                None => (Some(tbl), None),
            }
        }
        None => (None, None),
    };

    match (method.as_ref(), table, path) {
        ("GET", Some(t), None) => {
            // GET all items for table t
            let lock = STORAGE.read();
            let v = &spawn(lock).wait_future().unwrap().vault;
            match v.get(t as usize) {
                Some(_x) => {
                    println!("Found Gold in {}", t);
                }
                None => {}
            }
        }
        ("GET", None, None) => {
            // Get all items
        }
        ("POST", None, None) => {
            println!("Hello post empty post here");
            let resp = Response::builder()
                .status(200)
                .body(req.into_body())
                .unwrap();
            return Box::new(future::ok(resp));
        }
        ("POST", Some(t), None) => {
            println!("Hello post {}  here", t);

            let resp = req.into_body().concat2().map(|chunks| {
                println!("DATA: {:?}", chunks.as_ref());
                let res = serde_json::from_slice::<TableRequestVec>(chunks.as_ref())
                    // .map(handle_request)
                    .map(|t| t)
                    .and_then(|resp| serde_json::to_string(&resp));
                println!("Ok Somethuing {:?}", res);
                match res {
                    Ok(body) => {
                        println!("Ok Somethuing {:?}", body);
                        Response::new(body.into())
                    }
                    Err(err) => Response::builder()
                        .status(StatusCode::UNPROCESSABLE_ENTITY)
                        .body(err.to_string().into())
                        .unwrap(),
                }
            });

            // println!("Body future wait start");
            //match spawn(body).wait_future(){
            //    _ => println!("Body future wait")
            //}
            //Box::new(body);

            //println!("{:?}",body);

            // Add some items to table order
            let r = Record {
                id: t,
                state: States::ETA(t),
            };
            let lock = STORAGE.write().map(|mut guard| {
                (*guard).vault.push(RwLock::new(r));
            });
            let v = spawn(lock).wait_future();

            println!("bye bye {}", t);
            return Box::new(resp);
        }
        ("DELETE", Some(t), path) => {
            // Remove something from table t
        }
        ("UPDATE", Some(t), path) => {
            // Change some object for instance when it is deliverd to table
        }
        _ => {
            // Unsupported operation
        }
    };

    let ans = "TODO Chnage me";
    let resp = Response::builder()
        .status(200)
        .body(Body::from(ans))
        .unwrap();
    Box::new(future::ok(resp))
}

fn main() {
    println!("Starting server port");
    let addr = ([127, 0, 0, 1], 8888).into();

    let server = Server::bind(&addr).serve(|| service_fn(move |req| microservice_handler(req)));

    let server = server.map_err(drop);
    hyper::rt::run(server);
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO/Note add unit test on handler level, when there is time to add
    // a request struct and check the response struct. Have to figure out
    // how to achive this.
    //
    // For now this must be tested at system level by usage.
    #[test]
    fn check_regexp() {
        let ans = RE_TABLE_NUM.captures("/table/100");

        match ans {
            Some(m) => {
                assert_eq!(m.get(1).map_or("Unknown", |m| m.as_str()), "100");
            }
            _ => {
                assert!(false);
            }
        }
        match RE_TABLE_NUM.captures("/table/100/open") {
            Some(m) => {
                assert_eq!(m.get(1).map_or("Unknown", |m| m.as_str()), "100");
                assert_eq!(m.get(2).map_or("Unknown", |m| m.as_str()), "/open");
            }
            _ => {
                assert!(false);
            }
        }
    }
}
