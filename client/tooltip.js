
/** 
 * A namespace for Tooltip related functions and variables
 */
var Tooltip = {
	div: document.getElementById("Tooltip"),
	hidden: true,
	onMouseMove: function (event) {
		// no need to move if hidden
		if(Tooltip.hidden) return;
		Tooltip.div.style.left = event.clientX+10 +"px";
		Tooltip.div.style.top = event.clientY+10 + "px";
	},
	show: function (event) {
		Tooltip.hidden = false;
		Tooltip.div.classList.remove("hidden");
		if(event) {
			Tooltip.div.style.left = event.clientX+10 +"px";
			Tooltip.div.style.top = event.clientY+10 + "px";
		}
	},
	hide: function () {
		Tooltip.hidden = true;
		Tooltip.div.classList.add("hidden");
	},
};


//Tooltip.div.innerHTML = " Test";
window.addEventListener("mousemove", Tooltip.onMouseMove);


var ContextMenu = {
	div: document.getElementById("ContextMenu"),
	hidden: true,
	toggle: function(event) {
		if(ContextMenu.hidden) ContextMenu.show(event);
		else ContextMenu.hide();
	},
	show: function(event) {
		Tooltip.hide();
		ContextMenu.div.style.left = event.clientX+10 +"px";
		ContextMenu.div.style.top = event.clientY+10 + "px";
		ContextMenu.hidden = false;
		ContextMenu.div.classList.remove("hidden");
	},
	hide: function () {
		if(ContextMenu.hidden) return;
		ContextMenu.hidden = true;
		ContextMenu.div.classList.add("hidden");
	},
}
window.addEventListener("mouseup", function(event) {
	if(event.button == 0) ContextMenu.hide();
});
