# teketeke
Building a micro service based izakaya order system in Rust

Project Type
Personal development project to learn more about rust and microservices.


build with cargo run

Customer terminals can be emulated with 

scripts:
    excercice005.sh
    excercice006.sh

data can be verified with

curl http://localhost:8888/table/2|json_pp 
curl http://localhost:8888/table/9|json_pp 

