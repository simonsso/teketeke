#[macro_use]
extern crate serde_derive;
extern crate hyper;
extern crate hyper_staticfile;
extern crate serde_json;

use std::error::Error;

use futures::{future, Future, Stream};
use hyper::{Body, Request, Response, Server, StatusCode};

use hyper::error::Error as hyper_errors;
use hyper::service::service_fn;
use lazy_static::lazy_static;

use regex::Regex;

use futures::task::spawn;
use futures_locks::RwLock;
use hyper_staticfile::FileChunkStream;
use tokio::fs::File;

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
    static ref RE_TABLE:     Regex = Regex::new(r"^/table/?").unwrap();
    static ref RE_DOWNLOAD_FILE:Regex = Regex::new(r"^/(index.html|button.js)$").unwrap();
    static ref STORAGE:RwLock<Datastore> =datastore_rw_lock(101);   //init with tables upto 100
                                                                   // TODO this should be done on demand instead
    static ref ITEMNUM:RwLock<usize> =RwLock::new(0);             // Global uniq order num
}

fn get_global_num() -> usize {
    let mut retval = 0;
    let lock = ITEMNUM.write().map(|mut cnt| {
        *cnt += 1;
        retval = *cnt;
    });
    match spawn(lock).wait_future() {
        Ok(_x) => {}
        Err(_) => {}
    }
    retval
}

// Encapsulate response for hyper
fn microservice_handler(
    req: Request<Body>,
) -> Box<Future<Item = Response<Body>, Error = std::io::Error> + Send> {
    let uri: String = req.uri().to_string();
    let method = req.method().to_string();

    if let None = RE_TABLE.captures(&uri) {
        return serve_file(&uri);
    }
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
                        let headerstring = format!("\"{}\":", i);
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
            let tablelist = &spawn(lock).wait_future().unwrap().vault;
            return match tablelist.get(table as usize) {
                //TODO replace None case and Some case here
                Some(_) => {       // Sic TODO: this finds the tables vector and then does not use it
                    let boxedresult=table_add_items(req.into_body(),table);
                    let f =boxedresult.map_err(teketeke_to_stdio_err).map(move |s|{ 
                            match s {
                                Ok(s) => Response::builder().status(200).body(Body::from(s)),
                                Err(TeketekeError::InternalError(s)) => Response::builder().status(417).body(Body::from(s)),
                                _ =>  Response::builder().status(418).body(Body::from("Unknown error")),
                            }.unwrap()
                    });
                    Box::new(f)
                },
                None => {
                    let err = "I am a tea pot Error: this table is not allocated - build a bigger restaurant";
                    let response = Response::builder()
                            .status(418)
                            .body(Body::from(err)).unwrap();
                    Box::new(future::ok(response))
                }
            };
        }
        ("DELETE", Some(table), Some(path)) => {
            // Remove something from table t
            //Todo find a way to identify items in table tab... maybe with id
            let table = table as usize;
            match table_remove_item(table, path) {
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
#[derive(Debug)]
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
// Magic tranform of one kind of error to other
fn other<E>(err: E) -> std::io::Error
where
    E: Into<Box<std::error::Error + Send + Sync>>,
{
    std::io::Error::new(std::io::ErrorKind::Other, err)
}

// Magic tranform of one kind of error to other
fn to_stdio_err(e:hyper::Error) -> std::io::Error
{
    std::io::Error::new(std::io::ErrorKind::Other, e)
}

enum TeketekeError<E>
where  E: Into<Box<std::error::Error + Send + Sync>>,
{
    ExternalError(E),
    InternalError(String),
}

fn intoTeketekeError<E>(err: E) -> TeketekeError<E>
where
    E: Into<Box<std::error::Error + Send + Sync>>,
{
    TeketekeError::ExternalError(err)
}

fn teketeke_to_stdio_err(e:TeketekeError<std::io::Error>) -> std::io::Error
{
    match e {
        TeketekeError::ExternalError(err) => err,
        _ => {
                let not_found = std::io::ErrorKind::NotFound;
                std::io::Error::from(not_found)
        }
    }
}

fn table_add_items(
    body: Body,
    table: usize) -> Box<Future<Item=Result<String,TeketekeError<std::io::Error>>, Error = TeketekeError<std::io::Error>> + Send> {
    let res = body.concat2()
        .map(move |chunks| {
            serde_json::from_slice::<TableRequestVec>(chunks.as_ref())
                .map(|t| table_store_new_items(table, t.tab)).map_err(other).map_err(|e|{intoTeketekeError::<std::io::Error>(e)})
                .and_then(|x|{ 
                    if x == 0 {
                        Err(TeketekeError::InternalError("Nothing modified".to_string()))
                    }else{                    
                        Ok(x.to_string())
                    }
                })    
        }).map_err(other).map_err(|e|{intoTeketekeError::<std::io::Error>(e)});
    Box::new(res)
}

fn table_remove_item(table: usize, path: String) -> ApiResult<String> {
    let removethis = match path.parse::<usize>() {
        Ok(x) => x,
        Err(_x) => return ApiResult::Err(503, "Illegal table number".to_string()),
    };
    let outerlock = STORAGE.read().map(|outer| {
        // range check done is outside
        let innerlock = (*outer).vault[table as usize].write().map(|mut inner| {
            // *inner is now the handle for table vector

            match (*inner).iter().position(|c| (*c).id == removethis) {
                Some(x) => {
                    (*inner).remove(x);
                }
                None => {}
            }
        });
        spawn(innerlock).wait_future()
    });
    match spawn(outerlock).wait_future() {
        Ok(_) => 0,
        Err(_) => 0,
    };
    ApiResult::Ok("".to_string())
}

fn table_store_new_items(table: usize, v: Vec<TableRequest>) -> u32 {
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
        Ok(_) => retval as u32,
        Err(_) => 0,
    }
}

fn serve_file(path: &str) -> Box<Future<Item = Response<Body>, Error = std::io::Error> + Send> {
    // Only serv the hard coded files needed for this project
    if let Some(cap) = RE_DOWNLOAD_FILE.captures(path) {
        let filename = format!("client/{}", cap.get(1).unwrap().as_str());
        let open_file = File::open(filename);
        let body = open_file.map(|file| {
            let chunks = FileChunkStream::new(file);
            Response::new(Body::wrap_stream(chunks))
        });
        Box::new(body)
    } else {
        let ans = "Thou shalt not read forbidden files";
        let resp = Response::builder()
            .status(403)
            .body(Body::from(ans))
            .unwrap();
        Box::new(future::ok(resp))
    }
}

fn main() {
    let portnum=8888;
    println!("Starting server port at http://localhost:{}/index.html",portnum);
    let addr = ([127, 0, 0, 1], portnum).into();

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
        let ans = spawn(ans).wait_future();
        match ans {
            Err(_) =>     {      }
            Ok(_) =>      {assert!(true,"should have failed")}
            _ => {assert!(true,"test case took unexpected path")}
        }
        //assert!(r.status() == 422);

        let order = r#"{"tab":[{"itemname": "Edamame","qty" : 100 ,"eta":100 },{"itemname": "Nama biru","qty" : 5 ,"eta":200} ]}"#;
        let chunks = vec![order];
        let stream = futures::stream::iter_ok::<_, ::std::io::Error>(chunks);
        let body = Body::wrap_stream(stream);
        let table: usize = 1;
        let ans = table_add_items(body, table);
        let ans = spawn(ans).wait_future();
        match ans {
            Err(_) => {assert!(true,"should not have failed")},
            Ok(Ok(x)) =>  {assert_eq!(x,2.to_string()) },
            _ => {assert!(true,"test case took unexpected path")},
        }

        
    }
    #[test]
    fn check_store_values() {
        let mut v: Vec<TableRequest> = Vec::new();
        v.push(TableRequest {
            itemname: "Something".to_string(),
            qty: 1,
            eta: 100,
        });

        let table_get_all_res = match table_get_all(10) {
            ApiResult::Ok(x) => x,
            _ => panic!(),
        };
        let before: Vec<Record> = serde_json::from_slice(table_get_all_res.as_bytes()).unwrap();
        let storednum = table_store_new_items(10, v);
        assert_eq!(1, storednum);
        let table_get_all_res = match table_get_all(10) {
            ApiResult::Ok(x) => x,
            _ => panic!(),
        };
        let after: Vec<Record> = serde_json::from_slice(table_get_all_res.as_bytes()).unwrap();

        // Should be one more entry
        assert!(after.len() - before.len() == 1);
        table_remove_item(10, "1".to_string());

        let table_get_all_res = match table_get_all(10) {
            ApiResult::Ok(x) => x,
            _ => panic!(),
        };
        let _after: Vec<Record> = serde_json::from_slice(table_get_all_res.as_bytes()).unwrap();

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
