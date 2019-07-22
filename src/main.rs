use hyper::{Body, Response, Server, Request, StatusCode, Method};

use lazy_static::lazy_static;
use hyper::service::service_fn;

use std::sync::Arc;
// added from example
use std::io::{Error};
use futures::{future, Future, Stream, task};
use regex::Regex;

use std::{thread, time};

use futures_locks::RwLock;
use tokio::timer::Interval;
use futures::future::lazy;
use futures::task::spawn;
use future::{err, ok};

#[derive(Copy, Clone)]
enum States{
    ETA(u32),
    DONE,
    REMOVED,
    EMPTY,
}

struct Record{
    id: u32,
    state: States,
}

struct Datastore{
    vault: Vec<RwLock<Record>>,
}

fn DatastoreRwLock()->RwLock<Datastore>{
    let v:Vec<RwLock<Record>> = Vec::with_capacity(100);
    let d:Datastore = Datastore{vault : v};
    RwLock::new(d)
}


// Function for emulating execution time and explore locking anc blocking
fn hw() ->String{
    println!("HW start{:?}",thread::current().id() );
    println!("HW done {:?}",thread::current().id());
    "Hello, world!".to_string()
}

lazy_static!{
    // TODO verify the correctness of regexp in tests
    static ref RE_TABLE_NUM: Regex = Regex::new(r"^/table/(\d+)(/.*)?$").unwrap();
    static ref STORAGE:RwLock<Datastore> = DatastoreRwLock();
}


// Encapsulate response for hyper
fn microservice_handler(req: Request<Body>) -> Box<Future<Item=Response<Body>, Error=Error> + Send> {
    let uri:String = req.uri().to_string();
    let method = req.method().to_string();
    
    let (table,path):(Option<u32>,Option<String>) =  match RE_TABLE_NUM.captures(&uri){
        Some(m)=>{
            // this is checked to be an integer
            let tbl = m.get(1).unwrap().as_str().parse::<u32>().unwrap();
            match m.get(2){
                Some(argument) => {
                         (Some(tbl),Some(argument.as_str().to_string()))
                }
                None => {
                    (Some(tbl),None) 
                }
            }
        }
        None =>{
            (None,None)
        }
    };
    
    match (method.as_ref(),table,path){
        ("GET",Some(t),None) =>{
            // GET all items for table t
            let lock = STORAGE.read().map(|guard| { *guard });
            let mut v = spawn(lock).wait_future().unwrap().vault;
            match v.get(t as usize) {
                Some(x) => {
                    println!("Found Gold in {}",t);
                }
                None =>{
                    let r = Record{id:t, state: States::ETA(t) };
                    v.push(RwLock::new(r));
                }

            }
        }
        ("GET",None,None) =>{
            // Get all items
        }
        ("POST",Some(t),None) =>{
            // Add some items to table order
        }
        ("DELETE",Some(t),path) =>{
            // Remove something from table t
        }
        ("UPDATE",Some(t),path) =>{
            // Change some object for instance when it is deliverd to table
        }
        _ =>{
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

    let server = Server::bind(&addr).serve(||{
        service_fn(move |req|{
                microservice_handler(req)
            }
        )});

    let server = server.map_err(drop);
    hyper::rt::run(server);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!("Hello, world!" , hw());
    }
    #[test]
    #[should_panic]
    fn ne_it_works() {
        assert_eq!("Hello, WORLD!" , hw());
    }

    // TODO/Note add unit test on handler level, when there is time to add
    // a request struct and check the response struct. Have to figure out
    // how to achive this.
    //
    // For now this must be tested at system level by usage.
     #[test]
    fn handler_test(){
        let ans = microservice_handler_inner("/table/10".to_string(), "GET".to_string());
        assert_eq!(ans , "GET");

        let get = Request::new(Body::empty());
        let ans = microservice_handler(get);

        let dummy = Request::new(ans.body());
        assert!(ans.status().as_u16() == 200);
    }
    #[test]
    fn check_regexp(){
        let ans = RE_TABLE_NUM.captures("/table/100");

        match ans
        {
            Some(m) =>{
                assert_eq!( m.get(1).map_or("Unknown", |m| m.as_str()) , "100"  );
            }
            _ => {
                assert!(false);
            }
        }
        match RE_TABLE_NUM.captures("/table/100/open"){
            Some(m) =>{
                assert_eq!( m.get(1).map_or("Unknown", |m| m.as_str()) , "100"  );
                assert_eq!( m.get(2).map_or("Unknown", |m| m.as_str()) , "/open"  );
            }
            _ => {
                assert!(false);
            }
        }

    }
}
