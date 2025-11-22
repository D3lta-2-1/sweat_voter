var modal_1 = document.getElementById("modal_1");
var modal_2 = document.getElementById("modal_2");
var modal_3 = document.getElementById("modal_3");

var btn_1 = document.getElementById("button_1");
var btn_2 = document.getElementById("button_2");
var btn_3 = document.getElementById("button_3");

var span1 = document.getElementsByClassName("close_1")[0];
var span2 = document.getElementsByClassName("close_2")[0];
var span3 = document.getElementsByClassName("close_3")[0];


btn_1.onclick = function() {
  modal_1.style.display = "block";
}
btn_2.onclick = function() {
  modal_2.style.display = "block";
}
btn_3.onclick = function() {
  modal_3.style.display = "block";
}

span1.onclick = function() {
  modal_1.style.display = "none";
}
span2.onclick = function() {
  modal_2.style.display = "none";
}
span3.onclick = function() {
  modal_3.style.display = "none";
}

window.onclick = function(event) {
  if (event.target == modal) {
    modal.style.display = "none";
  }
} 