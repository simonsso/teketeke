use hyper::{Body, Response, Server, Request};
use hyper::rt::Future;
use lazy_static::lazy_static;
use hyper::service::service_fn_ok;
//TODO expect to need this: use hyper::service::service_fn;

use regex::Regex;


use std::{thread, time};

// Function for emulating execution time and explore locking anc blocking
fn hw() ->String{
    println!("HW start");
    let ten_sec = time::Duration::from_millis(10000);
    thread::sleep(ten_sec);
    println!("HW done");
    "Hello, world!".to_string()
}

lazy_static!{
    // TODO verify the correctness of regexp in tests
    static ref RE_TABLE_NUM: Regex = Regex::new(r"^/table/[\d+](/.*)$").unwrap();
}


// Encapsulate response for hyper
fn microservice_handler(req: Request<Body>) -> Response<Body> {
        let ans:String = microservice_handler_inner(req.uri().to_string(), req.method().to_string());
        Response::new(Body::from(ans))
}

// Change argument for unit_tests

fn microservice_handler_inner(uri:String,method:String) ->String{
    println!("{}:{}",method,uri);
    hw()
}

fn main() {
    
    println!("Would you like to play a game?");
    let addr = ([127, 0, 0, 1], 8888).into();

    let handler = ||{service_fn_ok(|req| microservice_handler(req ))};
    let server = Server::bind(&addr)
        .serve(handler)
        .map_err(|e| eprintln!("server error: {}", e));

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
        let ans = microservice_handler_inner("/table/10", "GET");
        //assert!(ans.body() , "Hello World");

        let ans = microservice_handler_inner("/table","GET");
        //assert!(*ans.body(), *"/");
    }
    #[test]
    fn check_regexp(){
        let ans = RE_TABLE_NUM.captures("/table/100/open");

        match ans
        {
            Some(m) =>{
                println!("Match 1{}",m.get(1).unwrap());
                println!("Match 1{}",m.get(1).unwrap());
            }
            _ => {
                assert!(false);
            }
        }
    }
}
