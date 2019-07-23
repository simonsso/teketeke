//extern crate futures;
//extern crate hyper;
#[macro_use]
extern crate serde_derive;
extern crate hyper;
extern crate serde_json;

use futures::{future, Future, Stream};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use std::borrow::Borrow;
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

#[derive(Copy, Deserialize, Clone, Serialize)]
enum States {
    ETA(u32),
    DELIVERD,
    EMPTY,
}
#[derive(Deserialize, Clone, Serialize)]
struct Record {
    itemname: String,
    id: u32,
    state: States,
    qty: i32,
}

struct Datastore {
    vault: Vec<RwLock<Vec<Record>>>,
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
fn DatastoreRwLock(num: usize) -> RwLock<Datastore> {
    let mut v: Vec<RwLock<Vec<Record>>> = Vec::with_capacity(100);
    for _ in 0..num {
        v.push(RwLock::new(Vec::new()))
    }
    let d: Datastore = Datastore { vault: v };
    RwLock::new(d)
}

lazy_static! {
    // TODO verify the correctness of regexp in tests
    static ref RE_TABLE_NUM: Regex = Regex::new(r"^/table/(\d+)(/.*)?$").unwrap();
    static ref STORAGE:RwLock<Datastore> = DatastoreRwLock(10);
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
        ("GET", Some(table), None) => {
            // GET all items for table t
            let table = table as usize;
            println!("Hello GET {}  here", table);
            match get_all(table) {
                ApiResult::Ok(s) => {
                    let resp = Response::builder().status(200).body(Body::from(s)).unwrap();
                    return Box::new(future::ok(resp));
                }
                ApiResult::Err(code, s) => {
                    let resp = Response::builder()
                        .status(code)
                        .body(Body::from(s))
                        .unwrap();
                    return Box::new(future::ok(resp));
                }
            }
        }
        ("GET", None, None) => {
            // Get all items
            println!("GET");
            let comma: String = ",".to_string();
            let lock = STORAGE.read();
            let v = &spawn(lock).wait_future().unwrap().vault;

            let mut bodychunks: Vec<String> = Vec::new();
            bodychunks.push("[".to_string());
            for i in 0..v.len() {
                match get_all(i) {
                    ApiResult::Ok(s) => {
                        bodychunks.push(s);
                        bodychunks.push(comma.clone())
                    }
                    ApiResult::Err(code, msg) => {
                        println!("Enexpected error fetcing all data {} {} {}", i, code, msg);
                    }
                }
            }
            if bodychunks.last() == Some(&comma) {
                bodychunks.pop();
            }
            bodychunks.push("]".to_string());
            let stream = futures::stream::iter_ok::<_, ::std::io::Error>(bodychunks);
            let body = Body::wrap_stream(stream);
            let resp = Response::builder().status(200).body(body).unwrap();
            return Box::new(future::ok(resp));
        }
        ("POST", None, None) => {
            println!("Hello post empty post here");
            let resp = Response::builder()
                .status(501)
                .body(req.into_body())
                .unwrap();
            return Box::new(future::ok(resp));
        }
        ("POST", Some(table), None) => {
            let lock = STORAGE.read();
            let v = &spawn(lock).wait_future().unwrap().vault;
            match v.get(table as usize) {
                Some(x) => {
                    return table_add_items(req.into_body(), table);
                }
                None => {
                    let err = "I am a tea pot Error: this table is not allocate - build a bigger restaurant";
                    let resp = Response::builder()
                        .status(418)
                        .body(Body::from(err))
                        .unwrap();
                    return Box::new(future::ok(resp));
                }
            }
        }
        ("DELETE", Some(t), path) => {
            // Remove something from table t

            //Todo find a way to identify items in table tab... maybe with id
        }
        ("UPDATE", Some(t), path) => {
            // Change some object for instance when it is deliverd to table
        }
        _ => {
            // Unsupported operation
        }
    };

    let ans = "Not implemented";
    let resp = Response::builder()
        .status(501)
        .body(Body::from(ans))
        .unwrap();
    Box::new(future::ok(resp))
}

enum ApiResult<T> {
    Ok(T),
    Err(u16, String),
}

fn get_all(table: usize) -> ApiResult<String> {
    let lock = STORAGE.read();
    let v = &spawn(lock).wait_future().unwrap().vault;
    match v.get(table) {
        Some(x) => {
            // let vec_lock:RwLock<Vec<Record>> = *x;
            let read_lock = (*x).read();

            let x1 = spawn(read_lock).wait_future().unwrap();
            //sic!
            let table_vec: Vec<Record> = x1.to_vec();

            let bodytext: String = serde_json::to_string(&table_vec).unwrap();
            ApiResult::Ok(bodytext)
        }
        None => ApiResult::Err(
            418,
            "I am a tea pot Error: this table is not allocate - build a bigger restaurant"
                .to_string(),
        ),
    }
}

fn table_add_items(
    body: Body,
    table: u32,
) -> Box<Future<Item = Response<Body>, Error = Error> + Send> {
    let resp = body.concat2().map(move |chunks| {
        let res = serde_json::from_slice::<TableRequestVec>(chunks.as_ref())
            .map(|t| slurp_vector(table, t.tab))
            .and_then(|resp| serde_json::to_string(&resp));
        match res {
            Ok(body) => Response::new(body.into()),
            Err(err) => Response::builder()
                .status(StatusCode::UNPROCESSABLE_ENTITY)
                .body(err.to_string().into())
                .unwrap(),
        }
    });
    Box::new(resp)
}

fn slurp_vector(table: u32, v: Vec<TableRequest>) -> u32 {
    let mut target: Vec<Record> = Vec::with_capacity(v.len());
    println!("{}", v.len());
    for i in v {
        let timetocook = 60 * 5;
        match i {
            TableRequest::order { itemname, qty } => target.push(Record {
                itemname: itemname,
                id: 0,
                qty: qty,
                state: States::ETA(timetocook),
            }),
        }
    }

    // Get lock for data store
    let outerlock = STORAGE.read().map(|outer| {
        // range check done is outside
        let innerlock = (*outer).vault[table as usize].write().map(|mut inner| {
            (*inner).append(&mut target);
        });
        spawn(innerlock).wait_future()
    });
    spawn(outerlock).wait_future();
    0
}

fn main() {
    println!("Starting server port at http://localhost:8888/");
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
    fn check_post_ans() {
        let chunks = vec!["hello", " ", "world"];
        let stream = futures::stream::iter_ok::<_, ::std::io::Error>(chunks);
        let body = Body::wrap_stream(stream);
        let table: u32 = 1000;
        let ans = table_add_items(body, table);
        let r = spawn(ans).wait_future().unwrap();
        assert!(r.status() == 422);

        let order = r#"{"tab":[{"order": "order", "parameters": { "itemname": "Edamame","qty" : 100 }},{"order": "order", "parameters": { "itemname": "Nama biru","qty" : 5 } }]}"#;
        let chunks = vec![order];
        let stream = futures::stream::iter_ok::<_, ::std::io::Error>(chunks);
        let body = Body::wrap_stream(stream);
        let table: u32 = 1;
        let ans = table_add_items(body, table);
        let r = spawn(ans).wait_future().unwrap();
        assert!(r.status() == 200);
    }
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
