[![Build Status](https://travis-ci.org/simonsso/teketeke.svg?branch=master)](https://travis-ci.org/simonsso/teketeke)
# teketeke 
Building a micro service based izakaya order system in Rust

Project Type
Personal development project to experiment and learn more about rust and microservices. This is the first time I used tokio and the hyper webserver, and react javascript framework.

The store contains two micro services, the menu and the table. The main actors are the kitchen and the customer (using the staff as client or on table application)

## build
build with cargo build and cargo run
 
## Html Client
Html client is self-contained at http://localhost:8888/index.html

## Curl and raw evaluation
Customer terminals can be emulated with  scripts:
    excercice005.sh
    excercice006.sh

data can be verified

curl http://localhost:8888/table/2|json_pp 
curl http://localhost:8888/table/ |json_pp 

## Some typical use-cases
Order some items:
Client loads the menu from /menu/ (TODO) picks items from its json object and posts a new json to /table/NN/  (DONE)

Cook item
get item from http://localhost:8888/table/ (DONE only in back-end)
update item on /table/item (TODO)
update menu with decreased inventory (TODO)
update item to invoice (TODO)

Change order
Http DELETE can be requested to http://localhost/table/x/y where x is the table num and y is the item num as returned from the GET operations (DONE in client and back-end)

## Environment
Built and debugged with lldb and visual studio code on Linux. Also verified by a native build on android.

## Design decisions and limitations
* Micro service for handle the menu list is still unimplemented it is emulated by a hard-coded list in the client
* local storage is only stored in running application without any store to disk.

## Acknowledgements 
Lots of inspiration for this project was found in *Hands-On Microservices with Rust: Build, test, and deploy scalable and reactive microservices with Rust 2018* By *Denis Kolodin* https://www.amazon.co.jp/dp/1789342759/ref=cm_sw_em_r_mt_dp_U_v8KoDbBQNXD3Y
