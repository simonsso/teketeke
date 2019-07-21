use hyper::{Body, Response, Server, Request};
use hyper::rt::Future;
use hyper::service::service_fn_ok;
use hyper::service::service_fn;


use std::{thread, time};


fn hw() ->String{
    println!("HW start");
    let ten_sec = time::Duration::from_millis(10000);
    thread::sleep(ten_sec);
    println!("HW done");
    "Hello, world!".to_string()
}


fn microservice_handler(req: Request<Body>) -> Response<Body> {
    println!("HELLO {}", req.uri());
    let uri:String = req.uri().to_string();
    let uri:Vec<&str> = uri.split('/').collect();

    // push two empty to ensure match have some data to work with

    match(uri[0],uri[1],uri[2]){
        ("table",_,_) => {
            Response::new(Body::from(hw()))
        }
        (_,_,_)=>{
            Response::new(Body::from(hw()))
        }
    }
 //   Response::new(Body::from(hw()))
}

fn main() {
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
    // ## #[test]
    // ##fn handler_test(){
    // ##    let mut r:Request<()> = Default();
    // ##    assert!(r.uri() , "/");
    // ##    let ans = microservice_handler(r);
    // ##    assert!(*ans.body(), *"/");
    // ##}
}
