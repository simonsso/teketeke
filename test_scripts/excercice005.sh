#!/bin/bash

function send_request() {
    echo -ne "- - - - - - - - - \nRequest: $1\nResponse: "
    curl --header "Content-Type: application/json" --request POST \
         --data "$1" \
         http://localhost:8888/table/2
    echo ""
}

send_request '{"tab":[{"order": "order", "parameters": { "itemname": "Coffe","qty" : 4 }},{"order": "order", "parameters": { "itemname": "Mayonaise","qty" : 10 } }]}'

