var Settings = {
	animateAnts: true,
	arrows: false,
	colorized: true,
	colorizationTarget: "pf",
	nodeName: "index1", // "name", "index", "index1"
	renderTeamArrows: true,
	renderNextStateInfo: false,
	renderNodeInfoOnMap: false,
	renderNextState: true,
	nopf: false,
	screenshotPad: 0.125,
}

const Map = L.map('map', {
	preferCanvas: true, // improves performance
	attributionControl: false,
	zoomControl: false
}).setView(
	{ lat: 41.059420776730676, lng: 29.068107604980472 },
	11
);

const NoMap = L.tileLayer('assets/WhiteBackground.jpg', {
	maxZoom: 19,
});

const OpenStreetMap = L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
	maxZoom: 19,
	attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors'
});

const StamenTerrain = L.tileLayer('https://stamen-tiles-{s}.a.ssl.fastly.net/terrain/{z}/{x}/{y}{r}.{ext}', {
	attribution: 'Map tiles by <a href="http://stamen.com">Stamen Design</a>, <a href="http://creativecommons.org/licenses/by/3.0">CC BY 3.0</a> &mdash; Map data &copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors',
	subdomains: 'abcd',
	minZoom: 0,
	maxZoom: 18,
	ext: 'png'
});

const StamenTonerLite = L.tileLayer('https://stamen-tiles-{s}.a.ssl.fastly.net/toner-lite/{z}/{x}/{y}{r}.{ext}', {
	attribution: 'Map tiles by <a href="http://stamen.com">Stamen Design</a>, <a href="http://creativecommons.org/licenses/by/3.0">CC BY 3.0</a> &mdash; Map data &copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors',
	subdomains: 'abcd',
	minZoom: 0,
	maxZoom: 20,
	ext: 'png'
});

var baseMaps = {
	"OpenStreetMap": OpenStreetMap,
	"StamenTerrain": StamenTerrain,
	"StamenTonerLite": StamenTonerLite,
	"Nomap": NoMap,
};
var selectedBaseMap = null;

function selectMap(name) {
	if(selectedBaseMap) {
		baseMaps[selectedBaseMap].remove();
	}
	baseMaps[name].addTo(Map).bringToBack();
	selectedBaseMap = name;
}

// selectMap("StamenTerrain");
selectMap("Nomap");
var TopRightPanel = document.getElementById("TopRightPanel");


(function() { // limits the scope
	var innerDiv;
	const wrapperDiv = d3.create("div")
		.attr("class","CustomSelect")
		.style("z-index", 20);
	var headDiv = wrapperDiv.append("div")
		.attr("class", "CustomSelectHead")
		.style("width", "12em")
		.text(selectedBaseMap)
		.on("click", function(_) {
			innerDiv.classList.toggle("open");
		}).node();
	const ul = wrapperDiv.append("div").attr("class", "CustomSelectList").append("div");
	innerDiv = ul.node();

	const data = Object.keys(baseMaps);
	ul.selectAll("div").data(data).join("div")
		.attr("class", "CustomSelectElement")
		.text(d => d)
		.on("click", d => {
			selectMap(d);
			headDiv.innerText=d;
			innerDiv.classList.remove("open");
		});

	TopRightPanel.appendChild(wrapperDiv.node());

	// {
	// 	let div = d3.create("div").classed("customCheckbox", true);
	// 	let checkbox = div.append("input")
	// 		.attr("id", "marchingAnts")
	// 		.attr("type", "checkbox")
	// 		.on("change", () => {
	// 			Settings.animateAnts = checkbox.node().checked;
	// 			if(graph) graph.rerender();
	// 		});
	// 	checkbox.node().checked = Settings.animateAnts;
	// 	div.append("label")
	// 		.attr("for", "marchingAnts")
	// 		.text("Marching Ants");
	// 	TopRightPanel.appendChild(div.node());
	// }
	// {
	// 	let div = d3.create("div").classed("customCheckbox", true);
	// 	let checkbox = div.append("input")
	// 		.attr("id", "arrows")
	// 		.attr("type", "checkbox")
	// 		.on("change", () => {
	// 			Settings.arrows = checkbox.node().checked;
	// 			if(graph) graph.rerender();
	// 		});
	// 	div.append("label")
	// 		.attr("for", "arrows")
	// 		.text("Arrows");
	// 	checkbox.node().checked = Settings.arrows;
	// 	TopRightPanel.appendChild(div.node());
	// }
	// {
	// 	let div = d3.create("div").classed("customCheckbox", true);
	// 	let checkbox = div.append("input")
	// 		.attr("id", "colorized")
	// 		.attr("type", "checkbox")
	// 		.on("change", () => {
	// 			Settings.colorized = checkbox.node().checked;
	// 			if(graph) graph.rerender();
	// 		});
	// 	checkbox.node().checked = Settings.colorized;
	// 	div.append("label")
	// 		.attr("for", "colorized")
	// 		.html("Colorize by P<sub>f</sub>");
	// 	TopRightPanel.appendChild(div.node());
	// }
	let colorOptions = [
		{
			name: "No colorization",
			func: () => {
				Settings.colorized = false;
				if(graph)
					graph.rerender();
			}
		},
		{
			name: "By P<sub>f</sub>",
			func: () => {
				Settings.colorized = true;
				Settings.colorizationTarget = "pf";
				if(graph)
					graph.rerender();
			}
		},
		{
			name: "By Cumulative P<sub>f</sub>",
			func: () => {
				Settings.colorized = true;
				Settings.colorizationTarget = "cpf";
				if(graph) {
					graph.calculateCumulativePfs();
					graph.rerender();
				}
			}
		},
	];
	createCustomSelectBox(d3.select(TopRightPanel), colorOptions, 1);
})();


var latdiv = document.getElementById("LatLang");

//var mouseLat
Map.on('mousemove', (event) => {
	Map.mousePos = [event.latlng.lat, event.latlng.lng];
	let lat = event.latlng.lat.toFixed(4);
	let lng = event.latlng.lng.toFixed(4);
	latdiv.innerHTML = lat + ", " + lng;
	// Pass the originalEvent
	//Tooltip.onMouseMove(event.originalEvent);
});


Map.on('click', event => {
	// NOTE: This fires even if user clicks on a marker
	let lat = Math.round(event.latlng.lat*10000.0)/10000.0;
	let lng = Math.round(event.latlng.lng*10000.0)/10000.0;
	console.log("{ \"latlng\": [",lat,",",lng,"]}");
});

Map.on("contextmenu", event => {
	console.log("contextmenu")
	if(graph) {
		graph.contextMenu(event);
		ContextMenu.toggle(event.originalEvent);
	}
	event.originalEvent.preventDefault();
});

Map.createPane("teams");
Map.getPane('teams').style.zIndex = 801;
Map.createPane("teamArrows");
Map.getPane('teamArrows').style.zIndex = 800;
Map.getPane('teamArrows').style.pointerEvents = "None";
Map.createPane("resources");
Map.getPane('resources').style.zIndex = 700;
Map.createPane("nodes");
Map.getPane('nodes').style.zIndex = 650;
Map.createPane("branches");
Map.getPane('branches').style.zIndex = 450;
Map.createPane("nodeInfo");
Map.getPane("nodeInfo").style.zIndex = 400;
//Map.getPane('branches').style.pointerEvents = "none";


// this function can be used to add some layers
// depending on the zoom level
/*
Map.on('zoomend', (event) => {
  if(Map.getZoom() > 16) {
	Graph.markerLayer.addTo(Map); //multiple consecutive calls have no effect
  } else {
	Graph.markerLayer.remove();
  }
});
*/


var CustomLeafletStyle = document.createElement("style");
CustomLeafletStyle.type = "text/css";
document.head.appendChild(CustomLeafletStyle);

const MapThemes = {
	"Default": {
		colors: {
			action: "#0000FF",
				energized: "#24B700",
				damaged: "#c70039",
				shadow: "#574f7d",
				risky: "#FDDC01",
		},
		css: ""
	},
	"Dark": {
		colors: {
			action: "#3050FF",
			energized: "#60FF60",
			damaged: "#FF0000",
			shadow: "#929292",
			risky: "#FFFF00",
		},
		css: `
			.leaflet-tile {
				filter: invert() brightness(0.5);
			}
			#map {
				background: black;
			}
		`
	}
};

function setMapTheme(name) {
	let theme = MapThemes[name];
	if(theme) {
		Colors = theme.colors;
		CustomLeafletStyle.innerText = theme.css;
	} else {
		throw new Error("Theme not found with name: "+name);
	}
}
// setMapTheme("Dark")


////////////////////////////////////////////////////////////////////////
//                            SCREEN SHOT                             //
////////////////////////////////////////////////////////////////////////

// L.simpleMapScreenshoter({
//    cropImageByInnerWH: true, // crop blank opacity from image borders
// }).addTo(Map);



// Set up snapshotter
const snapshotOptions = {
  hideElementsWithSelectors: [
    ".leaflet-control-container",
    ".leaflet-dont-include-pane",
    "#snapshot-button"
  ],
  hidden: true
};

// Add screenshotter to map
const screenshotter = L.simpleMapScreenshoter(snapshotOptions);
screenshotter.addTo(Map);

function takeScreenshot() {
	// Get bounds of features
	let featureBounds = null;
	Map.eachLayer(layer => {
		if (layer instanceof L.FeatureGroup) {
			if (featureBounds)
				featureBounds.extend(layer.getBounds());
			else
				featureBounds = layer.getBounds();
		}
	});
	if (featureBounds) {
		// Add padding
		featureBounds = featureBounds.pad(Settings.screenshotPad);
	} else {
		featureBounds = Map.getBounds();
	}

	// Get pixel position on screen of top left and bottom right
	// of the bounds of the feature
	const nw = featureBounds.getNorthWest();
	const se = featureBounds.getSouthEast();
	const topLeft = Map.latLngToContainerPoint(nw);
	const bottomRight = Map.latLngToContainerPoint(se);

	// Get the resulting image size that contains the feature
	const imageSize = bottomRight.subtract(topLeft);

	// Set up screenshot function
	screenshotter
		.takeScreen("image")
		.then((image) => {
			// Create <img> element to render img data
			var img = new Image();

			// once the image loads, do the following:
			img.onload = () => {
				// Create canvas to process image data
				const canvas = document.createElement("canvas");
				const ctx = canvas.getContext("2d");

				// Set canvas size to the size of your resultant image
				canvas.width = imageSize.x;
				canvas.height = imageSize.y;

				// Draw just the portion of the whole map image that contains
				// your feature to the canvas
				// from https://stackoverflow.com/questions/26015497/how-to-resize-then-crop-an-image-with-canvas
				ctx.drawImage(
					img,
					topLeft.x,
					topLeft.y,
					imageSize.x,
					imageSize.y,
					0,
					0,
					imageSize.x,
					imageSize.y
				);

				// Create URL for resultant png
				var imageurl = canvas.toDataURL("image/png");
				// console.log(imageurl);

				const resultantImage = new Image();
				resultantImage.style = "border: 1px solid black";
				resultantImage.src = imageurl;

				// document.body.appendChild(canvas);

				canvas.toBlob(function (blob) {
					// saveAs function installed as part of leaflet snapshot package
					saveAs(blob, "screenshot.png");
				});
			};

			// set the image source to what the snapshotter captured
			// img.onload will fire AFTER this
			img.src = image;
		})
		.catch((e) => {
			alert(e.toString());
		});
};
