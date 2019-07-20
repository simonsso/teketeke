use hyper::{Body, Response, Server, Request};
use hyper::rt::Future;
use hyper::service::service_fn_ok;
use hyper::service::service_fn;



fn hw() ->String{
    "Hello, world!".to_string()
}

fn microservice_handler(req: Request<Body>) -> Response<Body> {
    Response::new(Body::from(hw()))
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
}
