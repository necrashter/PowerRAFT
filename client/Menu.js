

var Overlay = document.getElementById("Overlay");
var HUD = document.getElementById("HUD");
var MenuButton = document.getElementById("MenuButton");
var PanelContainer = document.getElementById("PanelContainer");

var attributionHTML = `
  <h1>Attributions</h1>
  <p>Icons made by <a href="http://www.freepik.com/" title="Freepik">Freepik</a> from <a href="https://www.flaticon.com/" title="Flaticon"> www.flaticon.com</a></p>
  <p>Icons made by <a href="https://www.flaticon.com/free-icon/solar-energy_2933972" title="Good Ware">Good Ware</a> from <a href="https://www.flaticon.com/" title="Flaticon"> www.flaticon.com</a></p>
  <p><b>Target Icon:</b>
  Icons made by <a href="https://www.flaticon.com/authors/freepik" title="Freepik">Freepik</a> from <a href="https://www.flaticon.com/" title="Flaticon"> www.flaticon.com</a></p>
  Footprint:
  <div>Icons made by <a href="https://www.flaticon.com/authors/vitaly-gorbachev" title="Vitaly Gorbachev">Vitaly Gorbachev</a> from <a href="https://www.flaticon.com/" title="Flaticon">www.flaticon.com</a></div>

  Leaflet
  https://github.com/bbecquet/Leaflet.PolylineDecorator
  `;


Overlay.hide = function() {
	Overlay.classList.add("hidden");
	PanelContainer.classList.remove("disabled");
}

function generateCentredDiv() {
	document.getElementById("OverlayContent").innerHTML = `
	<div id="Centred">
	  <div id="OverlayClose" class="blockButton">X</div>
	  <div id="CentredContent"></div>
	</div>
	`;
	document.getElementById("OverlayClose")
		.addEventListener("click", Overlay.hide);
	return document.getElementById("CentredContent");
}

function aboutButton() {
	let inner = generateCentredDiv();
	inner.innerHTML = attributionHTML;
	Overlay.classList.remove("hidden");
}
function settingsButton() {
	let inner = d3.select(generateCentredDiv());
	inner.append("h1").text("Time Machine");
	inner.append("p").text("Travel in time!");
	let buttonDiv = inner.append("div");
	buttonDiv.selectAll("div").data(Styles).join("div")
		.classed("blockButton", true)
		.text(d => d.name)
		.on("click", d => {
			changeStyle(d.folder)
		});
	Overlay.classList.remove("hidden");
}


function statsButton() {
	let content = d3.select("#OverlayContent").html("")
		.append("div").attr("id", "OverlayRight");
	let box0 = content.append("div").classed("halfBox", true)
		.append("div").classed("contentBox", true);
	box0.append("h1").text("Bus Status");
	box0.append(createNodeStats);


	let box1 = content.append("div").classed("halfBox", true)
		.append("div").classed("contentBox", true);
	box1.append("h1").text("Bus Status");
	var labels = ["Damaged", "Unknown", "Energized"];
	var data = [0, 0, 0];
	graph.nodes.forEach(branch => {
		data[branch.status+1] += 1;
	});
	let width = box1.node().getBoundingClientRect().width;
	let height = content.node().getBoundingClientRect().height / 4;
	box1.append(() => 
		horizontalBarPlot(data.map((d,i) => {
			return {name: labels[i],
				value: d}
		}), width, height, )
	);

	{
		let box = content.append("div").classed("halfBox", true)
			.append("div").classed("contentBox", true);
		box.append("h1").text("Failure Probabilities");
		let division = 10;
		let data = { "Unknown" : 0};
		let keys = [];
		for(let i =0; i<division; ++i) {
			let lower = i/division;
			let upper = (i+1)/division;
			let name = `[${lower}, ${upper})`;
			keys.push(name);
			data[name] = 0;
		}
		graph.nodes.forEach(branch => {
			if(!branch.pf || isNaN(branch.pf)) {
				data["Unknown"] += 1;
			} else {
				let index = Math.floor(branch.pf*division);
				if(keys[index])
					data[keys[index]] += 1;
				else
					data["Unknown"] += 1;
			}
		});
		let width = box.node().getBoundingClientRect().width;
		let height = content.node().getBoundingClientRect().height / 2 - 100;
		box.append(() => donutChart(Object.keys(data).map(d => {
			return { name: d, value: data[d] };
		}) , width, height) );
	}
	content.append("div").classed("halfBox", true)
		.append("div").classed("contentBox", true)
		.text("Testing testing testing testing testing testing ");
}


function generateSideBar() {
	let div = d3.create("div").attr("id", "OverlayLeftBar");
	div.append("div").attr("class","blockButton")
		.text("X")
		.on("click", () => {
			Overlay.classList.add("hidden");
		});
	div.append("hr");
	div.append("div").attr("class","blockButton")
		.text("Stats")
		.on("click", () => {
			Overlay.classList.add("hidden");
		});
	return div.node();
}


function d3testMenu() {
	let inner = generateCentredDiv();
	inner.innerHTML = `
  <h1>Test</h1>
  <p>Hello World!</p>
  <h2>D3js Test</h2>
  `;
	let div = createNodeStats();
	inner.appendChild(div);

	Overlay.classList.remove("hidden");
}

function onMenuButton() {
	Overlay.innerHTML = `
		<div id="OverlayContent"></div>
		<div id="OverlayLeftBar">
			<div class="blockButton" onclick="Overlay.hide()">X</div>
			<div class="block">
				<div class="blockButton" onclick="statsButton()">Stats</div>
				<div class="blockButton" onclick="settingsButton()">Settings</div>
			</div>
			<div class="blockButton" onclick="aboutButton()">About</div>
		</div>
		`;
	statsButton();
	Overlay.classList.remove("hidden");
	PanelContainer.classList.add("disabled");
}

MenuButton.addEventListener("click", onMenuButton);


function showModalOverlay(content, options={}) {
	Overlay.innerHTML = `
	<div id="Centred">
	  <div id="OverlayClose" class="blockButton">X</div>
	  <div id="CentredContent"></div>
	</div>
	`;
	document.getElementById("OverlayClose").addEventListener("click", function () {
		Overlay.classList.add("hidden");
		Overlay.classList.remove("warning");
	});
	if(options.warning) Overlay.classList.add("warning");
	Overlay.classList.remove("hidden");
	document.getElementById("CentredContent").appendChild(content);
}
