# teketeke
Building a micro service based izakaya order system in Rust

Project Type
Personal development project to learn more about rust and microservices.

The store contains two micro services, the menu and the table. The main actors are the kitchen and the customer (using the staff as client or on table application)

## build
build with cargo run

Customer terminals can be emulated with  scripts:
    excercice005.sh
    excercice006.sh

data can be verified with

curl http://localhost:8888/table/2|json_pp 
curl http://localhost:8888/table/ |json_pp 

## typical use cases

Order someitems:
Client loads the menu from /menu/ (TODO) picks items from its json object and posts a new json to /table/NN/  (DONE)

Cook item
get item from http://localhost:8888/table/ (DONE)
update item on /table/item (TODO)
update menu with decreased inventory (TODO)
update item to invoice (TODO)

