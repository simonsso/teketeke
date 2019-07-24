#!/bin/bash

function send_request() {
    echo -ne "- - - - - - - - - \nRequest: $1\nResponse: "
    curl --header "Content-Type: application/json" --request POST \
         --data "$1" \
         http://localhost:8888/table/2
    echo ""
}
# client calculates eta current time + preparation time stored in unixtime
# this then there is no need for server to know anything about it

send_request '{"tab":[{ "itemname": "Coffe","qty" : 4 ,"eta":1563963105},{ "itemname": "Mayonaise","qty" : 10,"eta":1563963105 }]}'

