'use strict';

const e = React.createElement;
class LikeButton extends React.Component {
  constructor(props) {
    super(props);
    this.state = { liked: false , s:props.xitem };
  }
    
  render() {
    if (this.state.liked) { 

      return 'Table'+(tab)+' Orderd.'+this.state.s.itemname+" will be ready at "+((Date.now() / 1000 | 0)+this.state.s.time);
    }
    
    return e(
      'button',
      { onClick: () => {
              var tablenumer=document.getElementById("UITabNum");
              var tab=tablenumer?tablenumer.value:0;

              var qty =1;
              var ans=post_order({
                    table:tab,
                    itemname:this.state.s.itemname,
                    qty:qty,
                    eta:(Date.now() / 1000 | 0)+this.state.s.time,
              });
             }
      },
      this.state.s?this.state.s.itemname:'Like'
    );
  } 
}   

var dynamic_like=function(){
   const domContainer = document.querySelector('#top');
   var p= document.createElement("div");
   p.append("Some text");
   domContainer.append(p);
   ReactDOM.render(e(LikeButton), p);
}

var dynamic_menu=function(s){
   const domContainer = document.querySelector('#top');
   var p= document.createElement("div");
   p.append("Some text");
   domContainer.append(p);
   ReactDOM.render(e(LikeButton,{xitem:s}), p);
}

var post_order=function(o){
    return fetch('http://localhost:8888/table/'+o.table, {
      method: 'POST',
      headers: {
        'Accept': 'application/json',
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({tab:[{
        itemname: o.itemname,
        qty: o.qty,
        eta: o.eta,
      }]})
    })
}

window.setInterval(request_full_tab,10);

var request_full_tab = function(){
    let tablenumer=document.getElementById("UITabNum");
    let tab=tablenumer?tablenumer.value:0;
    let bartab=fetch('http://localhost:8888/table/'+tab, {
       method:"GET",
       headers: {
        'Accept': 'application/json',
        'Content-Type': 'application/json',
      }}
    ).then(response => response.json()).then(print_full_tab);
}

var print_full_tab=function(resp){
    let domContainer = document.querySelector('#bartab');
    while (domContainer.firstChild) {
      domContainer.removeChild(domContainer.firstChild);
  }
    for (let responesline of resp){
        let li=document.createElement("LI");
        let t=responesline.itemname;
        let time = responesline.eta-(Date.now() / 1000 | 0);

        li.appendChild(document.createTextNode(responesline.qty+"  "+t+" "+(time>0?time:" (overdue) ")))
        //domContainer.append(t);
        //domContainer.append(time>0?time:" DUE ");
        domContainer.appendChild(li);
    }
}