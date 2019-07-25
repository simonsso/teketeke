# teketeke
Building a micro service based izakaya order system in Rust

Project Type
Personal development project to learn more about rust and microservices.

The store contains two micro services, the menu and the table. The main actors are the kitchen and the customer (using the staff as client or on table application)

## build
build with cargo build and cargo run
 
## Html Client
Html client is self contained at http://localhost:8888/index.html

## Curl and raw evaluation
Customer terminals can be emulated with  scripts:
    excercice005.sh
    excercice006.sh

data can be verified

curl http://localhost:8888/table/2|json_pp 
curl http://localhost:8888/table/ |json_pp 

## typical use cases

Order someitems:
Client loads the menu from /menu/ (TODO) picks items from its json object and posts a new json to /table/NN/  (DONE)

Cook item
get item from http://localhost:8888/table/ (DONE only in backend)
update item on /table/item (TODO)
update menu with decreased inventory (TODO)
update item to invoice (TODO)

Change order
Http DELETE can be requested to http://localhost/table/x/y where x is the table num and y is the item num as returned from the GET operations (DONE in client and backend)

