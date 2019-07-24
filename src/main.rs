#[macro_use]
extern crate serde_derive;
extern crate hyper;
extern crate serde_json;

use futures::{future, Future, Stream};
use hyper::{Body, Request, Response, Server, StatusCode};

use hyper::service::service_fn;
use lazy_static::lazy_static;

use regex::Regex;

use futures::task::spawn;
use futures_locks::RwLock;
use hyper::error::Error;

#[derive(Copy, Deserialize, Clone, Serialize)]
enum States {
    PENDING,
    DELIVERD,
    EMPTY,
}
#[derive(Deserialize, Clone, Serialize)]
struct Record {
    itemname: String,
    id: usize,
    state: States,
    qty: i32,
    eta: u64,
}

struct Datastore {
    vault: Vec<RwLock<Vec<Record>>>,
}

#[derive(Deserialize, Clone, Serialize)]
struct TableRequestVec {
    tab: Vec<TableRequest>,
}

#[derive(Deserialize, Clone, Serialize)]
struct TableRequest {
    itemname: String,
    qty: i32,
    eta: u64,
}

fn datastore_rw_lock(num: usize) -> RwLock<Datastore> {
    let mut v: Vec<RwLock<Vec<Record>>> = Vec::with_capacity(100);
    for _ in 0..num {
        v.push(RwLock::new(Vec::new()))
    }
    let d: Datastore = Datastore { vault: v };
    RwLock::new(d)
}

lazy_static! {
    // TODO verify the correctness of regexp in tests
    static ref RE_TABLE_NUM: Regex = Regex::new(r"^/table/(\d+)(/(.*))?$").unwrap();
    static ref STORAGE:RwLock<Datastore> =datastore_rw_lock(100);  //TODO init with 100 tables this should be done on demand instead
    static ref ITEMNUM:RwLock<usize> =RwLock::new(0);             // Global uniq order num
}

fn get_global_num() -> usize{
    let mut retval = 0;
    let lock = ITEMNUM.write().map(|mut cnt|{
        *cnt+=1;
        retval=*cnt;
    });
    match spawn(lock).wait_future() {
        Ok(_x) =>  {  }
        Err(_) => {  }
    }
    retval
}

// Encapsulate response for hyper
fn microservice_handler(
    req: Request<Body>,
) -> Box<Future<Item = Response<Body>, Error = Error> + Send> {
    let uri: String = req.uri().to_string();
    let method = req.method().to_string();

    // Parse request URL with stored Regexp
    let (table, path): (Option<usize>, Option<String>) = match RE_TABLE_NUM.captures(&uri) {
        Some(m) => {
            // this is checked to be an integer
            let tbl = m.get(1).unwrap().as_str().parse::<usize>().unwrap();
            match m.get(3) {
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
            match table_get_all(table) {
                ApiResult::Ok(s) => {
                    return Box::new(future::ok(
                        Response::builder().status(200).body(Body::from(s)).unwrap(),
                    ));
                }
                ApiResult::Err(code, s) => {
                    return Box::new(future::ok(
                        Response::builder()
                            .status(code)
                            .body(Body::from(s))
                            .unwrap(),
                    ));
                }
            }
        }
        ("GET", None, None) => {
            // Get all items
            let comma: String = ",".to_string();
            let lock = STORAGE.read();
            let v = &spawn(lock).wait_future().unwrap().vault;

            //Reusing get from table code
            let mut bodychunks: Vec<String> = Vec::new();
            bodychunks.push("{\"tables\":{".to_string());
            for i in 0..v.len() {
                match table_get_all(i) {
                    ApiResult::Ok(s) => {
                        let headerstring = format!("\"{}\":",i);
                        bodychunks.push(headerstring);
                        bodychunks.push(s);
                        bodychunks.push(comma.clone())
                    }
                    ApiResult::Err(code, msg) => {
                        //TODO This should not occour
                        println!("Enexpected error fetcing all data {} {} {}", i, code, msg);
                    }
                }
            }
            if bodychunks.last() == Some(&comma) {
                bodychunks.pop();
            }
            bodychunks.push("}}".to_string());
            let stream = futures::stream::iter_ok::<_, ::std::io::Error>(bodychunks);
            let body = Body::wrap_stream(stream);
            let resp = Response::builder().status(200).body(body).unwrap();
            return Box::new(future::ok(resp));
        }
        ("POST", None, None) => {
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
                Some(_x) => {
                    return table_add_items(req.into_body(), table);
                }
                None => {
                    let err = "I am a tea pot Error: this table is not allocate - build a bigger restaurant";
                    return Box::new(future::ok(
                        Response::builder()
                            .status(418)
                            .body(Body::from(err))
                            .unwrap(),
                    ));
                }
            }
        }
        ("DELETE", Some(table), Some(path)) => {
            // Remove something from table t
            //Todo find a way to identify items in table tab... maybe with id
            let table = table as usize;
            match table_remove_item(table,path) {
                ApiResult::Ok(s) => {
                    return Box::new(future::ok(
                        Response::builder().status(200).body(Body::from(s)).unwrap(),
                    ));
                }
                ApiResult::Err(code, s) => {
                    return Box::new(future::ok(
                        Response::builder()
                            .status(code)
                            .body(Body::from(s))
                            .unwrap(),
                    ));
                }
            }
        }
        ("UPDATE", Some(_t), Some(_path)) => {
            // Change some object for instance when it is deliverd to table
        }
        _ => {
            // Unsupported operation
        }
    };
    // Fall throu default response
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


fn table_get_all(table: usize) -> ApiResult<String> {
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

fn table_add_items( body: Body, table: usize,
) -> Box<Future<Item = Response<Body>, Error = Error> + Send> {
    let resp = body.concat2().map(move |chunks| {
        let res = serde_json::from_slice::<TableRequestVec>(chunks.as_ref())
            .map(|t| table_store_new_items(table, t.tab))
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

fn table_remove_item(table:usize,path:String)-> ApiResult<String> {
    let removethis = match path.parse::<usize>(){
        Ok(x) => x,
        Err(_x) => return ApiResult::Err(503,"Illegal table number".to_string())
    };
    println!();
    let outerlock = STORAGE.read().map(|outer| {
    // range check done is outside
    let innerlock = (*outer).vault[table as usize].write().map(|mut inner| {
            // *inner is now the handle for table vector
            
            match (*inner).iter().position( |c| (*c).id == removethis) {
                Some(x) =>{
                     (*inner).remove(x);
                },
                None => {
                },
            }
        });
        spawn(innerlock).wait_future()
    });
    match spawn(outerlock).wait_future() {
        Ok(_) =>  { 0 }
        Err(_) => { 0 }
    };
    ApiResult::Ok("".to_string())
}

fn table_store_new_items(table: usize, v: Vec<TableRequest>) -> usize {
    let mut target: Vec<Record> = Vec::with_capacity(v.len());
    for i in v {
        target.push(Record {
            itemname: i.itemname,
            id: get_global_num(),
            qty: i.qty,
            state: States::PENDING,
            eta: i.eta,
        })
    }
    let retval = target.len();
    // Get lock for data store
    let outerlock = STORAGE.read().map(|outer| {
        // range check done is outside
        let innerlock = (*outer).vault[table as usize].write().map(|mut inner| {
            (*inner).append(&mut target);
        });
        spawn(innerlock).wait_future()
    });
    match spawn(outerlock).wait_future() {
        Ok(_) => {retval}
        Err(_) => { 0 }
    }
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
        let table: usize = 1;
        let ans = table_add_items(body, table);
        let r = spawn(ans).wait_future().unwrap();
        assert!(r.status() == 422);

        let order = r#"{"tab":[{"itemname": "Edamame","qty" : 100 ,"eta":100 },{"itemname": "Nama biru","qty" : 5 ,"eta":200} ]}"#;
        let chunks = vec![order];
        let stream = futures::stream::iter_ok::<_, ::std::io::Error>(chunks);
        let body = Body::wrap_stream(stream);
        let table: usize = 1;
        let ans = table_add_items(body, table);
        let r = spawn(ans).wait_future().unwrap();
        assert!(r.status() == 200);


    }  
    #[test]
    fn check_store_values(){
        let mut v: Vec<TableRequest> = Vec::new();
        v.push(TableRequest{itemname: "Something".to_string(),qty : 1, eta:100 });

        let table_get_all_res=match table_get_all(10){
            ApiResult::Ok(x) => x,
            _ =>{panic!()}
        };
        let before:Vec<Record> = serde_json::from_slice(table_get_all_res.as_bytes()).unwrap();
        let storednum =  table_store_new_items(10, v);
        assert_eq!(1, storednum);
        let table_get_all_res=match table_get_all(10){
            ApiResult::Ok(x) => x,
            _ =>{panic!()}
        };
        let after:Vec<Record> = serde_json::from_slice(table_get_all_res.as_bytes()).unwrap();

        // Should be one more entry
        assert!(after.len()-before.len() == 1);
        table_remove_item(10, "1".to_string());

        let table_get_all_res=match table_get_all(10){
            ApiResult::Ok(x) => x,
            _ =>{panic!()}
        };
        let _after:Vec<Record> = serde_json::from_slice(table_get_all_res.as_bytes()).unwrap();

        //TODO Should be back where we started but item ide can not be guessed as they are world uniq now
        //assert_eq!(after.len(),before.len());
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
